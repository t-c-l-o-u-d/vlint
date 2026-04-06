// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use anyhow::{Context, Result};

use crate::backend::{Backend, BackendKind};
use crate::catalog::linter::{OwnedToolDef, ToolResult};

const MIN_SYSTEMD_VERSION: u32 = 242;

static SYSTEMD_VERSION: OnceLock<Option<u32>> = OnceLock::new();

fn systemd_version() -> Option<u32> {
    *SYSTEMD_VERSION.get_or_init(|| {
        let output = Command::new("systemd-nspawn")
            .arg("--version")
            .output()
            .ok()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        // First line: "systemd 256 (256.11-1-arch)"
        let first_line = stdout.lines().next()?;
        first_line.split_whitespace().nth(1)?.parse::<u32>().ok()
    })
}

/// Resolve the OCI bundle path for a tool.
/// Searches: `$XDG_DATA_HOME/vlint/images/{name}` then `/var/lib/vlint/images/{name}`.
/// A valid bundle has `config.json` inside.
fn bundle_path(tool: &OwnedToolDef) -> Option<PathBuf> {
    let candidates = bundle_search_dirs(&tool.container_image);
    candidates
        .into_iter()
        .find(|p| p.join("config.json").is_file())
}

fn bundle_search_dirs(image_name: &str) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(data_home) = std::env::var("XDG_DATA_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join(".local/share"))
        })
    {
        dirs.push(data_home.join("vlint/images").join(image_name));
    }
    dirs.push(PathBuf::from("/var/lib/vlint/images").join(image_name));
    dirs
}

pub struct NspawnBackend;

impl Backend for NspawnBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Nspawn
    }

    fn available(&self) -> Result<(), String> {
        let Some(version) = systemd_version() else {
            return Err("systemd-nspawn not found".to_string());
        };
        if version < MIN_SYSTEMD_VERSION {
            return Err(format!(
                "systemd {version} < {MIN_SYSTEMD_VERSION} (no OCI support)"
            ));
        }
        Ok(())
    }

    fn has_tool(&self, tool: &OwnedToolDef) -> Result<(), String> {
        if bundle_path(tool).is_some() {
            Ok(())
        } else {
            Err(format!("no OCI bundle for {}", tool.container_image))
        }
    }

    fn run(&self, tool: &OwnedToolDef, args: &[&str], workspace: &Path) -> Result<ToolResult> {
        let bundle =
            bundle_path(tool).with_context(|| format!("OCI bundle not found for {}", tool.name))?;

        let mut cmd = Command::new("systemd-nspawn");
        cmd.arg(format!("--oci-bundle={}", bundle.display()));
        cmd.arg(format!("--bind={}:/workspace", workspace.display()));

        // Mount $HOME read-only so tool configs are accessible at their original paths.
        if let Ok(home) = std::env::var("HOME")
            && !home.is_empty()
        {
            cmd.arg(format!("--bind-ro={home}"));
        }

        if !tool.container_needs_network {
            cmd.arg("--private-network");
        }

        if !tool.container_needs_rw_mount {
            cmd.arg("--read-only");
        }

        for (key, val) in tool.env_vars.iter().chain(&tool.container_env_vars) {
            cmd.arg(format!("--setenv={key}={val}"));
        }

        cmd.arg("--").arg(&tool.binary_name).args(args);

        let output = cmd
            .output()
            .with_context(|| format!("failed to run nspawn for {}", tool.name))?;

        Ok(ToolResult {
            tool_name: tool.name.clone(),
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_code: output.status.code().unwrap_or(-1),
        })
    }
}
