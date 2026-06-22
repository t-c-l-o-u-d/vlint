// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

use vlint::catalog::linter::LinterId;
use vlint::output::{print_results, render, render_failure_hint};
use vlint::runner::{
    ConfigRef, FileOutcome, FileStatus, LintOutput, RunStatus, SkippedLinter, ToolRun,
};

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

// --- Rendered-output contract ---
//
// Assertions compare against color-stripped output, so they hold regardless of the process-global
// color state (whether or not any test, now or later, enables color). The expected strings are
// always plain text.

/// Remove ANSI SGR escape sequences (ESC `[` ... `m`) that `color::*` may wrap labels in, so
/// rendered output can be compared as plain text.
fn strip_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            for c2 in chars.by_ref() {
                if c2 == 'm' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// `render` with color stripped, for deterministic text assertions.
fn rendered(output: &LintOutput, verbose: bool) -> String {
    strip_ansi(&render(output, verbose))
}

fn outcome(path: &str, status: FileStatus) -> FileOutcome {
    FileOutcome {
        path: PathBuf::from(path),
        status,
    }
}

/// A directory-mode tool run with the given attribution and per-file outcomes.
fn dir_run(
    name: &str,
    status: RunStatus,
    pass_files: bool,
    attributed: bool,
    files: Vec<FileOutcome>,
) -> ToolRun {
    ToolRun {
        tool_name: name.to_string(),
        status,
        pass_files,
        cli: name.to_string(),
        config: None,
        batch_stdout: String::new(),
        batch_stderr: String::new(),
        attributed,
        files,
    }
}

fn single_file_output(results: Vec<ToolRun>) -> LintOutput {
    LintOutput {
        results,
        skipped: vec![],
        single_file: true,
    }
}

#[test]
fn regular_all_pass_renders_nothing() {
    let out = output(vec![passing_tool("ruff-check")], vec![]);
    assert_eq!(rendered(&out, false), "");
}

#[test]
fn regular_lists_only_failing_files_with_no_tool_suffix() {
    let tool = dir_run(
        "ruff-check",
        RunStatus::Fail,
        true,
        true,
        vec![
            outcome("a.py", FileStatus::Pass),
            outcome("b.py", FileStatus::Fail),
        ],
    );
    // Tool header, then only the failing file -- a.py is omitted and there is no "(ruff-check)".
    assert_eq!(
        rendered(&output(vec![tool], vec![]), false),
        "ruff-check\n  FAIL: b.py\n"
    );
}

#[test]
fn regular_non_attributable_failure_is_a_bare_fail() {
    let tool = dir_run(
        "mypy",
        RunStatus::Fail,
        true,
        false, // batch failed but every file passed on its own
        vec![
            outcome("a.py", FileStatus::Pass),
            outcome("b.py", FileStatus::Pass),
        ],
    );
    assert_eq!(
        rendered(&output(vec![tool], vec![]), false),
        "mypy\n  FAIL\n"
    );
}

#[test]
fn regular_whole_project_failure_is_a_bare_fail() {
    let tool = dir_run(
        "cargo-clippy",
        RunStatus::Fail,
        false, // whole-project tool
        true,
        vec![outcome("src/main.rs", FileStatus::Fail)],
    );
    assert_eq!(
        rendered(&output(vec![tool], vec![]), false),
        "cargo-clippy\n  FAIL\n"
    );
}

#[test]
fn regular_error_renders_tool_and_message() {
    let tool = ToolRun {
        batch_stderr: "no backend available".to_string(),
        ..dir_run(
            "golangci-lint",
            RunStatus::Error,
            false,
            true,
            vec![outcome("main.go", FileStatus::Error)],
        )
    };
    assert_eq!(
        rendered(&output(vec![tool], vec![]), false),
        "golangci-lint\n  ERROR: no backend available\n"
    );
}

#[test]
fn verbose_renders_cli_config_and_per_file_status() {
    let tool = ToolRun {
        cli: "ruff check".to_string(),
        // Absolute /etc path so abbreviate() is a no-op regardless of $HOME.
        config: Some(ConfigRef {
            path: "/etc/ruff.toml".to_string(),
            is_default: true,
        }),
        ..dir_run(
            "ruff-check",
            RunStatus::Fail,
            true,
            true,
            vec![
                outcome("a.py", FileStatus::Pass),
                outcome("b.py", FileStatus::Fail),
            ],
        )
    };
    assert_eq!(
        rendered(&output(vec![tool], vec![]), true),
        "ruff-check\n  cli: ruff check\n  config: /etc/ruff.toml (vlint default)\n  PASS: a.py\n  FAIL: b.py\n"
    );
}

#[test]
fn verbose_non_attributable_failure_shows_batch_output() {
    let tool = ToolRun {
        cli: "mypy".to_string(),
        batch_stdout: "error: Duplicate module named 'x'".to_string(),
        ..dir_run(
            "mypy",
            RunStatus::Fail,
            true,
            false,
            vec![
                outcome("a.py", FileStatus::Pass),
                outcome("b.py", FileStatus::Pass),
            ],
        )
    };
    assert_eq!(
        rendered(&output(vec![tool], vec![]), true),
        "mypy\n  cli: mypy\n  FAIL (not attributable to a single file)\n  error: Duplicate module named 'x'\n"
    );
}

#[test]
fn single_file_error_shows_stdout_and_stderr() {
    // Regression guard: a tool that exits 2 with its diagnostic on stdout must not lose it.
    let tool = ToolRun {
        batch_stdout: "stdout diagnostic".to_string(),
        batch_stderr: "stderr note".to_string(),
        ..dir_run(
            "ruff",
            RunStatus::Error,
            true,
            true,
            vec![outcome("a.py", FileStatus::Error)],
        )
    };
    assert_eq!(
        rendered(&single_file_output(vec![tool]), false),
        "ruff\n  stdout diagnostic\n  stderr note\n  ERROR\n"
    );
}

#[test]
fn single_file_pass_shows_tool_then_pass() {
    let tool = dir_run(
        "ruff-format",
        RunStatus::Pass,
        true,
        true,
        vec![outcome("a.py", FileStatus::Pass)],
    );
    assert_eq!(
        rendered(&single_file_output(vec![tool]), false),
        "ruff-format\n  PASS\n"
    );
}

#[test]
fn failure_hint_text_varies_by_mode_and_is_suppressed_when_clean() {
    let failing = output(vec![failing_tool("ruff-check")], vec![]);
    assert_eq!(
        render_failure_hint(&failing, false),
        "\nRun `vlint <filename>` or `vlint -v` to see more detail.\n"
    );
    assert_eq!(
        render_failure_hint(&failing, true),
        "\nRun `vlint <filename>` to see more detail.\n"
    );

    // No failure -> no hint.
    let clean = output(vec![passing_tool("ruff-check")], vec![]);
    assert_eq!(render_failure_hint(&clean, false), "");

    // Single-file mode already shows the tool output, so the hint is suppressed.
    let single = single_file_output(vec![failing_tool("ruff-check")]);
    assert_eq!(render_failure_hint(&single, false), "");
}
