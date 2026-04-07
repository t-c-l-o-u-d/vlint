// SPDX-License-Identifier: AGPL-3.0-or-later

use vlint::catalog::linter::{ConfigLocation, ConfigPrecedence, LinterId, OwnedToolDef, WalkStop};
use vlint::runner::build_args;

fn minimal_tool(config_precedence: Option<ConfigPrecedence>) -> OwnedToolDef {
    OwnedToolDef {
        linter_id: LinterId::Yaml,
        name: "test-tool".to_string(),
        binary_name: "test-tool".to_string(),
        container_image: "test-tool".to_string(),
        container_needs_network: false,
        container_needs_rw_mount: false,
        pass_files: false,
        subcommand: vec![],
        lint_args: vec![],
        format_print_args: None,
        format_args: None,
        config_precedence,
        env_vars: vec![],
        container_env_vars: vec![],
        probe_args: vec![],
        min_version: None,
    }
}

static TEST_PRECEDENCE: ConfigPrecedence = ConfigPrecedence {
    locations: &[ConfigLocation::Walk {
        filename: ".testrc",
        stop_at: WalkStop::Root,
    }],
    flag: "-c",
    cache_filename: "test.yaml",
    default: "extends: default\n",
};

#[test]
fn build_args_injects_config_flag() {
    let tool = minimal_tool(Some(TEST_PRECEDENCE));
    let config_path = "/some/path/config.yaml";
    let args = build_args(&tool, &[], Some(config_path));

    assert!(
        args.contains(&"-c".to_string()),
        "flag not found in {args:?}"
    );
    let idx = args.iter().position(|a| a == "-c").unwrap();
    assert_eq!(args[idx + 1], config_path);
}

#[test]
fn build_args_no_config_flag_without_precedence() {
    let tool = minimal_tool(None);
    let args = build_args(&tool, &["--strict".to_string()], None);
    assert_eq!(args, vec!["--strict".to_string()]);
}

#[test]
fn build_args_no_config_flag_when_path_is_none() {
    // Tool has precedence configured but resolve returned None (e.g. cache write failed)
    let tool = minimal_tool(Some(TEST_PRECEDENCE));
    let args = build_args(&tool, &["--strict".to_string()], None);
    assert_eq!(args, vec!["--strict".to_string()]);
}

#[test]
fn build_args_config_flag_comes_before_other_flags() {
    let tool = minimal_tool(Some(TEST_PRECEDENCE));
    let other_flags = vec!["--strict".to_string()];
    let args = build_args(&tool, &other_flags, Some("/path/config.yaml"));

    let flag_idx = args.iter().position(|a| a == "-c").unwrap();
    let strict_idx = args.iter().position(|a| a == "--strict").unwrap();
    assert!(
        flag_idx < strict_idx,
        "config flag should come before other flags: {args:?}"
    );
}

#[test]
fn build_args_with_subcommand() {
    let mut tool = minimal_tool(Some(TEST_PRECEDENCE));
    tool.subcommand = vec!["lint".to_string()];
    let args = build_args(&tool, &[], Some("/path/config.yaml"));

    assert_eq!(args[0], "lint");
    assert_eq!(args[1], "-c");
    assert_eq!(args[2], "/path/config.yaml");
}

#[test]
fn build_args_cargo_deny_ordering() {
    // cargo-deny: subcommand=["deny"], flag="--config", lint_args=["check", "advisories", ...]
    let mut tool = minimal_tool(Some(ConfigPrecedence {
        locations: &[],
        flag: "--config",
        cache_filename: "cargo-deny.toml",
        default: "",
    }));
    tool.subcommand = vec!["deny".to_string()];
    let lint_args = vec!["check".to_string(), "advisories".to_string()];
    let args = build_args(&tool, &lint_args, Some("/path/deny.toml"));

    assert_eq!(args[0], "deny");
    assert_eq!(args[1], "--config");
    assert_eq!(args[2], "/path/deny.toml");
    assert_eq!(args[3], "check");
    assert_eq!(args[4], "advisories");
}
