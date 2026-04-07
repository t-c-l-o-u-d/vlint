// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

use crate::backend::container::{ContainerConfig, build_run_args};
use crate::backend::{Backend, BackendKind};
use crate::catalog::linter::{OwnedToolDef, ToolResult};

pub struct PodmanBackend {
    registry: Option<String>,
    image_prefix: String,
    tag: String,
}

impl PodmanBackend {
    pub fn new(registry: Option<&str>, image_prefix: &str, tag: &str) -> Self {
        Self {
            registry: registry.map(str::to_owned),
            image_prefix: image_prefix.to_owned(),
            tag: tag.to_owned(),
        }
    }
}

impl Backend for PodmanBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Podman
    }

    fn available(&self) -> Result<(), String> {
        if which::which("podman").is_err() {
            return Err("podman not found".to_string());
        }
        Ok(())
    }

    fn has_tool(&self, _tool: &OwnedToolDef) -> Result<(), String> {
        Ok(())
    }

    fn run(
        &self,
        tool: &OwnedToolDef,
        args: &[&str],
        workspace: &Path,
        config_path: Option<&Path>,
    ) -> Result<ToolResult> {
        let config = ContainerConfig {
            runtime: "podman".to_string(),
            registry: self.registry.clone(),
            image_prefix: self.image_prefix.clone(),
            tag: self.tag.clone(),
        };
        let run_args = build_run_args(&config, tool, workspace, args, config_path);

        let output = Command::new("podman")
            .args(&run_args)
            .output()
            .with_context(|| format!("failed to run podman for {}", tool.name))?;

        Ok(ToolResult {
            tool_name: tool.name.clone(),
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_code: output.status.code().unwrap_or(-1),
        })
    }
}
