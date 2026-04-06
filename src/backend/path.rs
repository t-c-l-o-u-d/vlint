// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

use crate::backend::{Backend, BackendKind};
use crate::catalog::linter::{OwnedToolDef, ToolResult};

pub struct PathBackend;

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
        let ok = Command::new(&tool.binary_name)
            .args(&tool.probe_args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|s| s.success());
        if ok {
            Ok(())
        } else {
            Err(format!(
                "{} {} not installed",
                tool.binary_name,
                tool.probe_args.first().map_or("", String::as_str)
            ))
        }
    }

    fn run(&self, tool: &OwnedToolDef, args: &[&str], workspace: &Path) -> Result<ToolResult> {
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
