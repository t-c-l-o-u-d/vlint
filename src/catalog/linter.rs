// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LinterId {
    Ansible,
    Bash,
    Containerfile,
    Css,
    Csv,
    Go,
    Html,
    Javascript,
    Json,
    Markdown,
    Mkosi,
    Python,
    Ruby,
    Rust,
    Systemd,
    Vim,
    Yaml,
    Skip,
}

impl fmt::Display for LinterId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ansible => write!(f, "ansible"),
            Self::Bash => write!(f, "bash"),
            Self::Containerfile => write!(f, "containerfile"),
            Self::Css => write!(f, "css"),
            Self::Csv => write!(f, "csv"),
            Self::Go => write!(f, "go"),
            Self::Html => write!(f, "html"),
            Self::Javascript => write!(f, "javascript"),
            Self::Json => write!(f, "json"),
            Self::Markdown => write!(f, "markdown"),
            Self::Mkosi => write!(f, "mkosi"),
            Self::Python => write!(f, "python"),
            Self::Ruby => write!(f, "ruby"),
            Self::Rust => write!(f, "rust"),
            Self::Systemd => write!(f, "systemd"),
            Self::Vim => write!(f, "vim"),
            Self::Yaml => write!(f, "yaml"),
            Self::Skip => write!(f, "skip"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum WalkStop {
    Home,
    Root,
}

#[derive(Debug, Clone, Copy)]
pub enum ConfigLocation {
    Walk {
        filename: &'static str,
        stop_at: WalkStop,
    },
    EnvVar(&'static str),
    Xdg(&'static str),
    Home(&'static str),
}

#[derive(Debug, Clone, Copy)]
pub struct ConfigPrecedence {
    pub locations: &'static [ConfigLocation],
    pub flag: &'static str,
    pub cache_filename: &'static str,
    pub default: &'static str,
}

#[derive(Debug)]
pub struct ToolDef {
    pub linter_id: LinterId,
    pub name: &'static str,
    pub binary_name: &'static str,
    pub container_image: &'static str,
    pub container_needs_network: bool,
    pub container_needs_rw_mount: bool,
    pub pass_files: bool,
    pub subcommand: &'static [&'static str],
    pub lint_args: Option<&'static str>,
    pub format_print_args: Option<&'static str>,
    pub format_args: Option<&'static str>,
    pub config_precedence: Option<ConfigPrecedence>,
    pub env_vars: &'static [(&'static str, &'static str)],
    pub container_env_vars: &'static [(&'static str, &'static str)],
    pub probe_args: &'static [&'static str],
    pub min_version: Option<&'static str>,
    pub version_regex: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub struct OwnedToolDef {
    pub linter_id: LinterId,
    pub name: String,
    pub binary_name: String,
    pub container_image: String,
    pub container_needs_network: bool,
    pub container_needs_rw_mount: bool,
    pub pass_files: bool,
    pub subcommand: Vec<String>,
    pub lint_args: Vec<String>,
    pub format_print_args: Option<Vec<String>>,
    pub format_args: Option<Vec<String>>,
    pub config_precedence: Option<ConfigPrecedence>,
    pub env_vars: Vec<(String, String)>,
    pub container_env_vars: Vec<(String, String)>,
    pub probe_args: Vec<String>,
    pub min_version: Option<String>,
    pub version_regex: Option<String>,
}

/// Parse an args string: whitespace-separated tokens, `#` line comments ignored.
fn parse_args(input: &str) -> Vec<String> {
    input
        .lines()
        .flat_map(|line| line.split('#').next().unwrap_or("").split_whitespace())
        .map(String::from)
        .collect()
}

impl From<&ToolDef> for OwnedToolDef {
    fn from(t: &ToolDef) -> Self {
        Self {
            linter_id: t.linter_id,
            name: t.name.to_string(),
            binary_name: t.binary_name.to_string(),
            container_image: t.container_image.to_string(),
            container_needs_network: t.container_needs_network,
            container_needs_rw_mount: t.container_needs_rw_mount,
            pass_files: t.pass_files,
            subcommand: t.subcommand.iter().map(|s| (*s).to_string()).collect(),
            lint_args: t.lint_args.map_or_else(Vec::new, parse_args),
            format_print_args: t.format_print_args.map(parse_args),
            format_args: t.format_args.map(parse_args),
            config_precedence: t.config_precedence,
            env_vars: t
                .env_vars
                .iter()
                .map(|&(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            container_env_vars: t
                .container_env_vars
                .iter()
                .map(|&(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            probe_args: t.probe_args.iter().map(|s| (*s).to_string()).collect(),
            min_version: t.min_version.map(str::to_string),
            version_regex: t.version_regex.map(str::to_string),
        }
    }
}

pub struct ToolResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

pub struct DetectionResult {
    pub file_assignments: std::collections::HashMap<LinterId, Vec<PathBuf>>,
}
