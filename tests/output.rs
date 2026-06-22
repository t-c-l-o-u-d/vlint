// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

use vlint::catalog::linter::LinterId;
use vlint::output::print_results;
use vlint::runner::{FileOutcome, FileStatus, LintOutput, RunStatus, SkippedLinter, ToolRun};

fn tool_run(name: &str, status: RunStatus) -> ToolRun {
    let file_status = match status {
        RunStatus::Pass => FileStatus::Pass,
        RunStatus::Fail => FileStatus::Fail,
        RunStatus::Error => FileStatus::Error,
    };
    ToolRun {
        tool_name: name.to_string(),
        status,
        pass_files: true,
        cli: name.to_string(),
        config: None,
        batch_stdout: String::new(),
        batch_stderr: String::new(),
        attributed: true,
        files: vec![FileOutcome {
            path: PathBuf::from("file"),
            status: file_status,
        }],
    }
}

fn passing_tool(name: &str) -> ToolRun {
    tool_run(name, RunStatus::Pass)
}

fn failing_tool(name: &str) -> ToolRun {
    tool_run(name, RunStatus::Fail)
}

fn error_tool(name: &str) -> ToolRun {
    tool_run(name, RunStatus::Error)
}

fn output(results: Vec<ToolRun>, skipped: Vec<SkippedLinter>) -> LintOutput {
    LintOutput {
        results,
        skipped,
        single_file: false,
    }
}

#[test]
fn all_pass_returns_zero() {
    let out = output(
        vec![passing_tool("cargo-clippy"), passing_tool("cargo-fmt")],
        vec![],
    );
    assert_eq!(print_results(&out, false), 0);
}

#[test]
fn any_fail_returns_one() {
    let out = output(
        vec![passing_tool("cargo-fmt"), failing_tool("cargo-clippy")],
        vec![],
    );
    assert_eq!(print_results(&out, false), 1);
}

#[test]
fn any_error_returns_two() {
    let out = output(vec![error_tool("cargo-clippy")], vec![]);
    assert_eq!(print_results(&out, false), 2);
}

#[test]
fn error_takes_precedence_over_fail() {
    let out = output(
        vec![failing_tool("cargo-fmt"), error_tool("cargo-clippy")],
        vec![],
    );
    assert_eq!(print_results(&out, false), 2);
}

#[test]
fn empty_output_returns_zero() {
    let out = output(vec![], vec![]);
    assert_eq!(print_results(&out, false), 0);
}

#[test]
fn skipped_only_returns_zero() {
    let out = output(
        vec![],
        vec![SkippedLinter {
            linter_id: LinterId::Javascript,
            files: vec![PathBuf::from("index.js")],
        }],
    );
    assert_eq!(print_results(&out, false), 0);
}

#[test]
fn mixed_pass_and_fail_returns_one() {
    let out = output(
        vec![passing_tool("cargo-clippy"), failing_tool("ruff-check")],
        vec![],
    );
    assert_eq!(print_results(&out, false), 1);
}

#[test]
fn per_file_detail_with_one_failing_file_returns_one() {
    let tool = ToolRun {
        tool_name: "ruff-check".to_string(),
        status: RunStatus::Fail,
        pass_files: true,
        cli: "ruff check".to_string(),
        config: None,
        batch_stdout: String::new(),
        batch_stderr: String::new(),
        attributed: true,
        files: vec![
            FileOutcome {
                path: PathBuf::from("a.py"),
                status: FileStatus::Pass,
            },
            FileOutcome {
                path: PathBuf::from("b.py"),
                status: FileStatus::Fail,
            },
        ],
    };
    let out = output(vec![tool], vec![]);
    // Both regular and verbose rollups treat a single failing file as a fail.
    assert_eq!(print_results(&out, false), 1);
    assert_eq!(print_results(&out, true), 1);
}

#[test]
fn whole_project_tool_fail_returns_one() {
    let tool = ToolRun {
        tool_name: "cargo-clippy".to_string(),
        status: RunStatus::Fail,
        pass_files: false,
        cli: "cargo clippy".to_string(),
        config: None,
        batch_stdout: String::new(),
        batch_stderr: String::new(),
        attributed: true,
        files: vec![
            FileOutcome {
                path: PathBuf::from("src/main.rs"),
                status: FileStatus::Fail,
            },
            FileOutcome {
                path: PathBuf::from("src/lib.rs"),
                status: FileStatus::Fail,
            },
        ],
    };
    let out = output(vec![tool], vec![]);
    assert_eq!(print_results(&out, false), 1);
}
