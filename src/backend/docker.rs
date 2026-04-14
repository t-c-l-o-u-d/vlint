// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

use crate::backend::container::{
    ContainerConfig, build_run_args, image_name, local_image_id, remove_image,
};
use crate::backend::{Backend, BackendKind};
use crate::catalog::linter::{OwnedToolDef, ToolResult};

pub struct DockerBackend {
    registry: Option<String>,
    image_prefix: String,
    tag: String,
}

impl DockerBackend {
    pub fn new(registry: Option<&str>, image_prefix: &str, tag: &str) -> Self {
        Self {
            registry: registry.map(str::to_owned),
            image_prefix: image_prefix.to_owned(),
            tag: tag.to_owned(),
        }
    }
}

impl Backend for DockerBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Docker
    }

    fn available(&self) -> Result<(), String> {
        if which::which("docker").is_err() {
            return Err("docker not found".to_string());
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
            runtime: "docker".to_string(),
            registry: self.registry.clone(),
            image_prefix: self.image_prefix.clone(),
            tag: self.tag.clone(),
        };

        let image = image_name(
            tool,
            config.registry.as_deref(),
            &config.image_prefix,
            &config.tag,
        );
        let old_id = local_image_id("docker", &image);

        let run_args = build_run_args(&config, tool, workspace, args, config_path);
        let output = Command::new("docker")
            .args(&run_args)
            .output()
            .with_context(|| format!("failed to run docker for {}", tool.name))?;

        if let Some(old) = &old_id
            && local_image_id("docker", &image).as_ref() != Some(old)
        {
            remove_image("docker", old);
        }

        Ok(ToolResult {
            tool_name: tool.name.clone(),
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_code: output.status.code().unwrap_or(-1),
        })
    }
}
