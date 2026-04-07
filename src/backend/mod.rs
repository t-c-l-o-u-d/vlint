// SPDX-License-Identifier: AGPL-3.0-or-later

pub mod container;
pub mod docker;
pub mod nspawn;
pub mod path;
pub mod podman;

use std::path::Path;

use anyhow::Result;

use crate::catalog::linter::{OwnedToolDef, ToolResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Path,
    Nspawn,
    Podman,
    Docker,
}

impl std::fmt::Display for BackendKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Path => write!(f, "PATH"),
            Self::Nspawn => write!(f, "nspawn"),
            Self::Podman => write!(f, "podman"),
            Self::Docker => write!(f, "docker"),
        }
    }
}

pub trait Backend: Send + Sync {
    fn kind(&self) -> BackendKind;
    /// Check if this backend is usable.
    ///
    /// # Errors
    /// Returns `Err(reason)` if the backend is not available.
    fn available(&self) -> Result<(), String>;
    /// Check if this backend can run the tool.
    ///
    /// # Errors
    /// Returns `Err(reason)` if the tool is not available on this backend.
    fn has_tool(&self, tool: &OwnedToolDef) -> Result<(), String>;
    /// Run the tool on this backend.
    ///
    /// # Errors
    /// Returns `Err` if the tool could not be launched or its output could not be read.
    fn run(
        &self,
        tool: &OwnedToolDef,
        args: &[&str],
        workspace: &Path,
        config_path: Option<&Path>,
    ) -> Result<ToolResult>;
}

#[must_use]
pub fn discovery_chain(
    registry: Option<&str>,
    image_prefix: &str,
    tag: &str,
) -> Vec<Box<dyn Backend>> {
    vec![
        Box::new(path::PathBackend),
        Box::new(nspawn::NspawnBackend),
        Box::new(podman::PodmanBackend::new(registry, image_prefix, tag)),
        Box::new(docker::DockerBackend::new(registry, image_prefix, tag)),
    ]
}

pub fn resolve_tool<'a>(
    tool: &OwnedToolDef,
    chain: &'a [Box<dyn Backend>],
) -> Option<&'a dyn Backend> {
    chain
        .iter()
        .find(|b| b.available().is_ok() && b.has_tool(tool).is_ok())
        .map(AsRef::as_ref)
}
