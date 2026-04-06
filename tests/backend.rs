// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

use vlint::backend::container::build_run_args;
use vlint::backend::path::PathBackend;
use vlint::backend::{Backend, BackendKind};
use vlint::catalog::linter::{LinterId, OwnedToolDef};

fn minimal_tool() -> OwnedToolDef {
    OwnedToolDef {
        linter_id: LinterId::Yaml,
        name: "test-tool".to_string(),
        binary_name: "test-tool".to_string(),
        container_image: "test-image".to_string(),
        container_needs_network: false,
        container_needs_rw_mount: false,
        pass_files: false,
        subcommand: vec![],
        lint_args: vec![],
        format_print_args: None,
        format_args: None,
        config_precedence: None,
        env_vars: vec![],
        container_env_vars: vec![],
        probe_args: vec![],
    }
}

// --- PathBackend ---

#[test]
fn path_backend_always_available() {
    assert!(PathBackend.available().is_ok());
}

#[test]
fn path_backend_kind_is_path() {
    assert_eq!(PathBackend.kind(), BackendKind::Path);
}

#[test]
fn path_backend_missing_binary_unavailable() {
    let mut tool = minimal_tool();
    tool.binary_name = "vlint-nonexistent-binary-xyz".to_string();
    assert!(PathBackend.has_tool(&tool).is_err());
}

#[test]
fn path_backend_sh_available() {
    let mut tool = minimal_tool();
    tool.binary_name = "sh".to_string();
    // sh has no probe_args, so has_tool just checks which::which
    assert!(PathBackend.has_tool(&tool).is_ok());
}

// --- container::build_run_args ---

fn container_config(runtime: &str) -> vlint::backend::container::ContainerConfig {
    vlint::backend::container::ContainerConfig {
        runtime: runtime.to_string(),
    }
}

#[test]
fn build_run_args_starts_with_run_rm() {
    let tool = minimal_tool();
    let workspace = PathBuf::from("/workspace");
    let args = build_run_args(
        &container_config("podman"),
        &tool,
        &workspace,
        &[],
        None,
        "",
        "latest",
    );
    assert_eq!(args[0], "run");
    assert_eq!(args[1], "--rm");
}

#[test]
fn build_run_args_mounts_workspace_readonly() {
    let tool = minimal_tool();
    let workspace = PathBuf::from("/myproject");
    let args = build_run_args(
        &container_config("podman"),
        &tool,
        &workspace,
        &[],
        None,
        "",
        "latest",
    );
    let mount_idx = args.iter().position(|a| a == "-v").unwrap();
    let mount = &args[mount_idx + 1];
    assert!(mount.starts_with("/myproject:"));
    assert!(mount.contains(":ro,z") || mount.contains(":z"));
}

#[test]
fn build_run_args_rw_mount_when_needed() {
    let mut tool = minimal_tool();
    tool.container_needs_rw_mount = true;
    let workspace = PathBuf::from("/myproject");
    let args = build_run_args(
        &container_config("podman"),
        &tool,
        &workspace,
        &[],
        None,
        "",
        "latest",
    );
    let mount_idx = args.iter().position(|a| a == "-v").unwrap();
    let mount = &args[mount_idx + 1];
    assert!(!mount.contains("ro,"), "rw mount should not have ro flag");
}

#[test]
fn build_run_args_no_network_flag_when_not_needed() {
    let tool = minimal_tool();
    let workspace = PathBuf::from("/workspace");
    let args = build_run_args(
        &container_config("podman"),
        &tool,
        &workspace,
        &[],
        None,
        "",
        "latest",
    );
    assert!(args.contains(&"--network=none".to_string()));
}

#[test]
fn build_run_args_no_network_flag_absent_when_needed() {
    let mut tool = minimal_tool();
    tool.container_needs_network = true;
    let workspace = PathBuf::from("/workspace");
    let args = build_run_args(
        &container_config("docker"),
        &tool,
        &workspace,
        &[],
        None,
        "",
        "latest",
    );
    assert!(!args.contains(&"--network=none".to_string()));
}

#[test]
fn build_run_args_podman_uses_keep_id() {
    let tool = minimal_tool();
    let workspace = PathBuf::from("/workspace");
    let args = build_run_args(
        &container_config("podman"),
        &tool,
        &workspace,
        &[],
        None,
        "",
        "latest",
    );
    assert!(args.contains(&"--userns=keep-id".to_string()));
}

#[test]
fn build_run_args_tool_args_at_end() {
    let tool = minimal_tool();
    let workspace = PathBuf::from("/workspace");
    let extra = &["--strict", "--format=json"];
    let args = build_run_args(
        &container_config("podman"),
        &tool,
        &workspace,
        extra,
        None,
        "",
        "latest",
    );
    let last_two: Vec<&str> = args.iter().map(String::as_str).rev().take(2).collect();
    assert!(last_two.contains(&"--strict"));
    assert!(last_two.contains(&"--format=json"));
}
