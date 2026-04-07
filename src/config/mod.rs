// SPDX-License-Identifier: AGPL-3.0-or-later

mod ini_parse;
pub mod resolve;

use std::collections::HashMap;
use std::hash::BuildHasher;
use std::path::{Path, PathBuf};

use crate::catalog::linter::OwnedToolDef;
use crate::color::ColorMode;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FormatMode {
    /// Show what formatters would change, exit without linting
    Check,
    /// Apply formatter changes, then lint
    Apply,
}

#[derive(Debug, Default)]
pub struct Config {
    pub format: Option<FormatMode>,
    pub registry: Option<String>,
    pub image_prefix: Option<String>,
    pub tag: Option<String>,
    pub backend: Option<String>,
    pub color: Option<ColorMode>,
    pub pass_color: Option<String>,
    pub fail_color: Option<String>,
    pub skip_color: Option<String>,
    pub tool_color: Option<String>,
    pub auto_update: Option<bool>,
    pub man_page_install: Option<bool>,
    pub tool_overrides: HashMap<String, HashMap<String, String>>,
}

impl Config {
    fn merge_vlint<S: BuildHasher>(&mut self, map: &HashMap<String, String, S>) {
        if let Some(v) = map.get("format") {
            self.format = match v.as_str() {
                "check" => Some(FormatMode::Check),
                "apply" => Some(FormatMode::Apply),
                _ => None,
            };
        }
        if let Some(v) = map.get("registry") {
            self.registry = Some(v.clone());
        }
        if let Some(v) = map.get("image_prefix") {
            self.image_prefix = Some(v.clone());
        }
        if let Some(v) = map.get("tag") {
            self.tag = Some(v.clone());
        }
        if let Some(v) = map.get("backend") {
            self.backend = Some(v.clone());
        }
        if let Some(v) = map.get("color") {
            self.color = v.parse().ok();
        }
        if let Some(v) = map.get("pass_color") {
            self.pass_color = Some(v.clone());
        }
        if let Some(v) = map.get("fail_color") {
            self.fail_color = Some(v.clone());
        }
        if let Some(v) = map.get("skip_color") {
            self.skip_color = Some(v.clone());
        }
        if let Some(v) = map.get("tool_color") {
            self.tool_color = Some(v.clone());
        }
        if let Some(v) = map.get("auto_update") {
            self.auto_update = match v.as_str() {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            };
        }
        if let Some(v) = map.get("man_page_install") {
            self.man_page_install = match v.as_str() {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            };
        }
    }

    fn merge_tools<S: BuildHasher, T: BuildHasher>(
        &mut self,
        tools: &HashMap<String, HashMap<String, String, T>, S>,
    ) {
        for (name, overrides) in tools {
            let entry = self.tool_overrides.entry(name.clone()).or_default();
            for (key, value) in overrides {
                entry.insert(key.clone(), value.clone());
            }
        }
    }
}

/// Load config from the user config file (`$XDG_CONFIG_HOME/vlint/config.ini`,
/// or `config_path` if provided). The `workspace` parameter is reserved for
/// future workspace-level config merging.
pub fn load_config(_workspace: &Path, config_path: Option<&Path>) -> Config {
    let mut config = Config::default();

    let path = config_path.map(Path::to_path_buf).or_else(user_config_path);
    if let Some(parsed) = path.and_then(|p| ini_parse::parse_ini(&p).ok()) {
        config.merge_vlint(&parsed.vlint);
        config.merge_tools(&parsed.tools);
    }

    config
}

/// Apply tool overrides from config onto the owned catalog.
pub fn apply_tool_overrides<S: BuildHasher, T: BuildHasher>(
    tools: &mut [OwnedToolDef],
    overrides: &HashMap<String, HashMap<String, String, T>, S>,
) {
    for tool in tools.iter_mut() {
        if let Some(ovr) = overrides.get(&tool.name) {
            merge_tool(tool, ovr);
        }
    }
}

fn merge_tool<S: BuildHasher>(tool: &mut OwnedToolDef, ovr: &HashMap<String, String, S>) {
    if let Some(v) = ovr.get("binary_name") {
        tool.binary_name.clone_from(v);
    }
    if let Some(v) = ovr.get("container_image") {
        tool.container_image.clone_from(v);
    }
    if let Some(v) = ovr.get("lint_args") {
        tool.lint_args = v.split_whitespace().map(String::from).collect();
    }
    if let Some(v) = ovr.get("format_print_args") {
        tool.format_print_args = if v.is_empty() {
            None
        } else {
            Some(v.split_whitespace().map(String::from).collect())
        };
    }
    if let Some(v) = ovr.get("format_args") {
        tool.format_args = if v.is_empty() {
            None
        } else {
            Some(v.split_whitespace().map(String::from).collect())
        };
    }
    if let Some(v) = ovr.get("env_vars") {
        tool.env_vars = v
            .split_whitespace()
            .filter_map(|pair| {
                let (k, v) = pair.split_once('=')?;
                Some((k.to_string(), v.to_string()))
            })
            .collect();
    }
    if let Some(v) = ovr.get("container_env_vars") {
        tool.container_env_vars = v
            .split_whitespace()
            .filter_map(|pair| {
                let (k, v) = pair.split_once('=')?;
                Some((k.to_string(), v.to_string()))
            })
            .collect();
    }
}

fn user_config_path() -> Option<PathBuf> {
    let config_home = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join(".config"))
        })?;
    Some(config_home.join("vlint").join("config.ini"))
}
