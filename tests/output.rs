// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

use vlint::catalog::linter::{LinterId, ToolResult};
use vlint::output::print_results;
use vlint::runner::{LintOutput, RunResult, SkippedLinter};

fn passing_tool(name: &str) -> ToolResult {
    ToolResult {
        tool_name: name.to_string(),
        success: true,
        stdout: String::new(),
        stderr: String::new(),
        exit_code: 0,
    }
}

fn failing_tool(name: &str) -> ToolResult {
    ToolResult {
        tool_name: name.to_string(),
        success: false,
        stdout: String::new(),
        stderr: String::new(),
        exit_code: 1,
    }
}

fn error_tool(name: &str) -> ToolResult {
    ToolResult {
        tool_name: name.to_string(),
        success: false,
        stdout: String::new(),
        stderr: String::new(),
        exit_code: 2,
    }
}

#[test]
fn all_pass_returns_zero() {
    let output = LintOutput {
        results: vec![RunResult {
            linter_id: LinterId::Rust,
            tool_results: vec![passing_tool("cargo-clippy"), passing_tool("cargo-fmt")],
        }],
        skipped: vec![],
    };
    assert_eq!(print_results(&output), 0);
}

#[test]
fn any_fail_returns_one() {
    let output = LintOutput {
        results: vec![RunResult {
            linter_id: LinterId::Rust,
            tool_results: vec![passing_tool("cargo-fmt"), failing_tool("cargo-clippy")],
        }],
        skipped: vec![],
    };
    assert_eq!(print_results(&output), 1);
}

#[test]
fn any_error_returns_two() {
    let output = LintOutput {
        results: vec![RunResult {
            linter_id: LinterId::Rust,
            tool_results: vec![error_tool("cargo-clippy")],
        }],
        skipped: vec![],
    };
    assert_eq!(print_results(&output), 2);
}

#[test]
fn error_takes_precedence_over_fail() {
    let output = LintOutput {
        results: vec![RunResult {
            linter_id: LinterId::Rust,
            tool_results: vec![failing_tool("cargo-fmt"), error_tool("cargo-clippy")],
        }],
        skipped: vec![],
    };
    assert_eq!(print_results(&output), 2);
}

#[test]
fn empty_output_returns_zero() {
    let output = LintOutput {
        results: vec![],
        skipped: vec![],
    };
    assert_eq!(print_results(&output), 0);
}

#[test]
fn skipped_only_returns_zero() {
    let output = LintOutput {
        results: vec![],
        skipped: vec![SkippedLinter {
            linter_id: LinterId::Javascript,
            files: vec![PathBuf::from("index.js")],
        }],
    };
    assert_eq!(print_results(&output), 0);
}
