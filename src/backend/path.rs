// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

use crate::backend::{Backend, BackendKind};
use crate::catalog::linter::{OwnedToolDef, ToolResult};

pub struct PathBackend;

/// Extract the first `X.Y.Z` version triple from a string, ignoring a leading `v`.
fn parse_version(s: &str) -> Option<(u32, u32, u32)> {
    s.split_whitespace().find_map(|token| {
        let t = token.trim_start_matches('v');
        let mut parts = t.splitn(4, '.');
        let major = parts.next()?.parse::<u32>().ok()?;
        let minor = parts
            .next()?
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse::<u32>()
            .ok()?;
        let patch = parts
            .next()?
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse::<u32>()
            .ok()?;
        Some((major, minor, patch))
    })
}

impl Backend for PathBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Path
    }

    fn available(&self) -> Result<(), String> {
        Ok(())
    }

    fn has_tool(&self, tool: &OwnedToolDef) -> Result<(), String> {
        if which::which(&tool.binary_name).is_err() {
            return Err(format!("{} not on PATH", tool.binary_name));
        }
        if tool.probe_args.is_empty() {
            return Ok(());
        }
        if let Some(min) = &tool.min_version {
            let output = Command::new(&tool.binary_name)
                .args(&tool.probe_args)
                .output()
                .map_err(|e| format!("failed to probe {}: {e}", tool.binary_name))?;
            if !output.status.success() {
                return Err(format!(
                    "{} {} not installed",
                    tool.binary_name,
                    tool.probe_args.first().map_or("", String::as_str)
                ));
            }
            let combined = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            let installed = parse_version(&combined).ok_or_else(|| {
                format!("could not parse version from {} output", tool.binary_name)
            })?;
            let required = parse_version(min).ok_or_else(|| {
                format!("invalid min_version configured for {}", tool.binary_name)
            })?;
            if installed < required {
                return Err(format!(
                    "{} {}.{}.{} is below minimum {min}",
                    tool.binary_name, installed.0, installed.1, installed.2
                ));
            }
        } else {
            let ok = Command::new(&tool.binary_name)
                .args(&tool.probe_args)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .is_ok_and(|s| s.success());
            if !ok {
                return Err(format!(
                    "{} {} not installed",
                    tool.binary_name,
                    tool.probe_args.first().map_or("", String::as_str)
                ));
            }
        }
        Ok(())
    }

    fn run(
        &self,
        tool: &OwnedToolDef,
        args: &[&str],
        workspace: &Path,
        _config_path: Option<&Path>,
    ) -> Result<ToolResult> {
        let mut cmd = Command::new(&tool.binary_name);
        cmd.current_dir(workspace);

        for (key, val) in &tool.env_vars {
            cmd.env(key, val);
        }

        cmd.args(args);

        let output = cmd
            .output()
            .with_context(|| format!("failed to execute {}", tool.name))?;

        Ok(ToolResult {
            tool_name: tool.name.clone(),
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_code: output.status.code().unwrap_or(-1),
        })
    }
}
