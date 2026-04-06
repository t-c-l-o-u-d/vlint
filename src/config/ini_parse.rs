// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};

pub struct ParsedConfig {
    pub vlint: HashMap<String, String>,
    pub tools: HashMap<String, HashMap<String, String>>,
}

pub fn parse_ini(path: &Path) -> Result<ParsedConfig> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;

    let mut vlint = HashMap::new();
    let mut tools: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut current_section: Option<String> = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if let Some(inner) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            current_section = Some(inner.trim().to_string());
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim().to_string();
            let value = value.trim().to_string();
            match current_section.as_deref() {
                Some(name) if name.starts_with("tool:") => {
                    let tool_name = &name["tool:".len()..];
                    tools
                        .entry(tool_name.to_string())
                        .or_default()
                        .insert(key, value);
                }
                Some("vlint") | None => {
                    vlint.insert(key, value);
                }
                _ => {}
            }
        }
    }

    Ok(ParsedConfig { vlint, tools })
}
