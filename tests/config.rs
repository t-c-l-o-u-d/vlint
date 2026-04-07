// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use tempfile::NamedTempFile;
use vlint::catalog::linter::{LinterId, OwnedToolDef};
use vlint::config::{FormatMode, apply_tool_overrides, load_config};

fn minimal_tool() -> OwnedToolDef {
    OwnedToolDef {
        linter_id: LinterId::Yaml,
        name: "test".to_string(),
        binary_name: "test".to_string(),
        container_image: "test".to_string(),
        container_needs_network: false,
        container_needs_rw_mount: false,
        pass_files: false,
        subcommand: vec![],
        lint_args: vec!["--default".to_string()],
        format_print_args: None,
        format_args: None,
        config_precedence: None,
        env_vars: vec![],
        container_env_vars: vec![],
        probe_args: vec!["--version".to_string()],
        min_version: None,
    }
}

fn ini_config(body: &str) -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    write!(f, "{body}").unwrap();
    f
}

fn ovr(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

// --- tool overrides: settable fields ---

#[test]
fn override_binary_name() {
    let mut tools = vec![minimal_tool()];
    let mut overrides = HashMap::new();
    overrides.insert("test".to_string(), ovr(&[("binary_name", "mytool")]));
    apply_tool_overrides(&mut tools, &overrides);
    assert_eq!(tools[0].binary_name, "mytool");
}

#[test]
fn override_lint_args_whitespace_split() {
    let mut tools = vec![minimal_tool()];
    let mut overrides = HashMap::new();
    overrides.insert("test".to_string(), ovr(&[("lint_args", "--foo --bar baz")]));
    apply_tool_overrides(&mut tools, &overrides);
    assert_eq!(tools[0].lint_args, vec!["--foo", "--bar", "baz"]);
}

#[test]
fn override_format_print_args_empty_clears() {
    let mut tools = vec![minimal_tool()];
    tools[0].format_print_args = Some(vec!["--diff".to_string()]);
    let mut overrides = HashMap::new();
    overrides.insert("test".to_string(), ovr(&[("format_print_args", "")]));
    apply_tool_overrides(&mut tools, &overrides);
    assert!(tools[0].format_print_args.is_none());
}

#[test]
fn override_format_args_empty_clears() {
    let mut tools = vec![minimal_tool()];
    tools[0].format_args = Some(vec!["--fix".to_string()]);
    let mut overrides = HashMap::new();
    overrides.insert("test".to_string(), ovr(&[("format_args", "")]));
    apply_tool_overrides(&mut tools, &overrides);
    assert!(tools[0].format_args.is_none());
}

#[test]
fn override_env_vars_parsed() {
    let mut tools = vec![minimal_tool()];
    let mut overrides = HashMap::new();
    overrides.insert("test".to_string(), ovr(&[("env_vars", "FOO=bar BAZ=qux")]));
    apply_tool_overrides(&mut tools, &overrides);
    assert_eq!(
        tools[0].env_vars,
        vec![
            ("FOO".to_string(), "bar".to_string()),
            ("BAZ".to_string(), "qux".to_string()),
        ]
    );
}

#[test]
fn override_container_env_vars_parsed() {
    let mut tools = vec![minimal_tool()];
    let mut overrides = HashMap::new();
    overrides.insert(
        "test".to_string(),
        ovr(&[("container_env_vars", "A=1 B=2")]),
    );
    apply_tool_overrides(&mut tools, &overrides);
    assert_eq!(
        tools[0].container_env_vars,
        vec![
            ("A".to_string(), "1".to_string()),
            ("B".to_string(), "2".to_string()),
        ]
    );
}

// --- tool overrides: locked fields ---

#[test]
fn override_container_needs_network_locked() {
    let mut tools = vec![minimal_tool()];
    let mut overrides = HashMap::new();
    overrides.insert(
        "test".to_string(),
        ovr(&[("container_needs_network", "true")]),
    );
    apply_tool_overrides(&mut tools, &overrides);
    assert!(!tools[0].container_needs_network);
}

#[test]
fn override_container_needs_rw_mount_locked() {
    let mut tools = vec![minimal_tool()];
    let mut overrides = HashMap::new();
    overrides.insert(
        "test".to_string(),
        ovr(&[("container_needs_rw_mount", "true")]),
    );
    apply_tool_overrides(&mut tools, &overrides);
    assert!(!tools[0].container_needs_rw_mount);
}

#[test]
fn override_probe_args_locked() {
    let mut tools = vec![minimal_tool()];
    let mut overrides = HashMap::new();
    overrides.insert("test".to_string(), ovr(&[("probe_args", "--custom")]));
    apply_tool_overrides(&mut tools, &overrides);
    assert_eq!(tools[0].probe_args, vec!["--version".to_string()]);
}

// --- apply_tool_overrides: matching ---

#[test]
fn unknown_tool_override_ignored() {
    let mut tools = vec![minimal_tool()];
    let mut overrides = HashMap::new();
    overrides.insert("nonexistent".to_string(), ovr(&[("binary_name", "other")]));
    apply_tool_overrides(&mut tools, &overrides);
    assert_eq!(tools[0].binary_name, "test");
}

#[test]
fn matching_tool_override_applied() {
    let mut tools = vec![minimal_tool()];
    let mut overrides = HashMap::new();
    overrides.insert("test".to_string(), ovr(&[("binary_name", "updated")]));
    apply_tool_overrides(&mut tools, &overrides);
    assert_eq!(tools[0].binary_name, "updated");
}

// --- load_config: format mode ---

#[test]
fn load_config_format_check() {
    let f = ini_config("[vlint]\nformat = check\n");
    let cfg = load_config(Path::new("/"), Some(f.path()));
    assert_eq!(cfg.format, Some(FormatMode::Check));
}

#[test]
fn load_config_format_apply() {
    let f = ini_config("[vlint]\nformat = apply\n");
    let cfg = load_config(Path::new("/"), Some(f.path()));
    assert_eq!(cfg.format, Some(FormatMode::Apply));
}

#[test]
fn load_config_format_invalid_is_none() {
    let f = ini_config("[vlint]\nformat = garbage\n");
    let cfg = load_config(Path::new("/"), Some(f.path()));
    assert_eq!(cfg.format, None);
}

// --- load_config: auto_update ---

#[test]
fn load_config_auto_update_true() {
    let f = ini_config("[vlint]\nauto_update = true\n");
    let cfg = load_config(Path::new("/"), Some(f.path()));
    assert_eq!(cfg.auto_update, Some(true));
}

#[test]
fn load_config_auto_update_false() {
    let f = ini_config("[vlint]\nauto_update = false\n");
    let cfg = load_config(Path::new("/"), Some(f.path()));
    assert_eq!(cfg.auto_update, Some(false));
}
