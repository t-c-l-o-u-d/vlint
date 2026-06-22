// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Execution-model tests for run_linters: per-file isolation on failure, format-mode
// ("Option 1") hash-diff attribution, exit-code mapping, and the error paths. A scripted
// MockBackend makes the outcomes deterministic without invoking real tools.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use tempfile::TempDir;

use super::{FileStatus, Invocation, LintOutput, Mode, RunStatus, ToolRun, run_linters};
use crate::backend::{Backend, BackendKind};
use crate::catalog::linter::{DetectionResult, LinterId, OwnedToolDef, ToolResult};

type Handler = Box<dyn Fn(&[&str], &Path) -> anyhow::Result<ToolResult> + Send + Sync>;

/// A backend whose `run()` returns a scripted result based on the args it receives,
/// counting how many times it is invoked.
struct MockBackend {
    calls: Arc<Mutex<u32>>,
    handler: Handler,
}

impl Backend for MockBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Path
    }
    fn available(&self) -> Result<(), String> {
        Ok(())
    }
    fn has_tool(&self, _tool: &OwnedToolDef) -> Result<(), String> {
        Ok(())
    }
    fn run(
        &self,
        _tool: &OwnedToolDef,
        args: &[&str],
        workspace: &Path,
        _config_path: Option<&Path>,
    ) -> anyhow::Result<ToolResult> {
        *self.calls.lock().unwrap() += 1;
        (self.handler)(args, workspace)
    }
}

fn result(success: bool, exit_code: i32) -> anyhow::Result<ToolResult> {
    Ok(ToolResult {
        success,
        stdout: String::new(),
        stderr: String::new(),
        exit_code,
    })
}

fn base_tool(pass_files: bool) -> OwnedToolDef {
    OwnedToolDef {
        linter_id: LinterId::Python,
        name: "mock".to_string(),
        binary_name: "mock".to_string(),
        container_image: String::new(),
        container_needs_network: false,
        container_needs_rw_mount: false,
        pass_files,
        subcommand: Vec::new(),
        lint_args: Vec::new(),
        format_print_args: None,
        format_args: None,
        config_precedence: None,
        env_vars: Vec::new(),
        container_env_vars: Vec::new(),
        probe_args: Vec::new(),
        min_version: None,
        version_regex: None,
    }
}

/// Create a temp workspace containing the given (relative) files with stable content.
fn workspace(files: &[&str]) -> TempDir {
    let dir = TempDir::new().unwrap();
    for f in files {
        std::fs::write(dir.path().join(f), "original\n").unwrap();
    }
    dir
}

fn run_mock(
    tool: OwnedToolDef,
    files: &[&str],
    ws: &Path,
    mode: Mode,
    invocation: Invocation,
    handler: impl Fn(&[&str], &Path) -> anyhow::Result<ToolResult> + Send + Sync + 'static,
) -> (LintOutput, u32) {
    let calls = Arc::new(Mutex::new(0u32));
    let detection = DetectionResult {
        file_assignments: HashMap::from([(
            tool.linter_id,
            files.iter().map(PathBuf::from).collect::<Vec<_>>(),
        )]),
    };
    let chain: Vec<Box<dyn Backend>> = vec![Box::new(MockBackend {
        calls: Arc::clone(&calls),
        handler: Box::new(handler),
    })];
    let out = run_linters(&detection, &chain, &[tool], ws, mode, invocation);
    let n = *calls.lock().unwrap();
    (out, n)
}

fn only(out: &LintOutput) -> &ToolRun {
    assert_eq!(out.results.len(), 1, "expected exactly one tool run");
    &out.results[0]
}

fn status_of(run: &ToolRun, rel: &str) -> FileStatus {
    run.files
        .iter()
        .find(|f| f.path == Path::new(rel))
        .unwrap_or_else(|| panic!("no outcome for {rel}"))
        .status
}

// --- Lint mode: per-file isolation ---

#[test]
fn lint_isolation_attributes_only_the_failing_file() {
    let ws = workspace(&["a.py", "b.py"]);
    let (out, calls) = run_mock(
        base_tool(true),
        &["a.py", "b.py"],
        ws.path(),
        Mode::Lint,
        Invocation::Directory,
        |args, _| {
            if args.len() > 1 {
                result(false, 1) // the batch fails
            } else if args.contains(&"b.py") {
                result(false, 1) // b.py fails on its own
            } else {
                result(true, 0) // a.py passes on its own
            }
        },
    );
    let run = only(&out);
    assert_eq!(run.status, RunStatus::Fail);
    assert!(run.attributed);
    assert_eq!(status_of(run, "a.py"), FileStatus::Pass);
    assert_eq!(status_of(run, "b.py"), FileStatus::Fail);
    assert_eq!(calls, 3, "1 batch + 2 per-file isolation re-runs");
}

#[test]
fn single_matched_file_is_not_isolated() {
    let ws = workspace(&["a.py"]);
    let (out, calls) = run_mock(
        base_tool(true),
        &["a.py"],
        ws.path(),
        Mode::Lint,
        Invocation::Directory,
        |_, _| result(false, 1),
    );
    let run = only(&out);
    assert_eq!(run.status, RunStatus::Fail);
    assert!(run.attributed);
    assert_eq!(status_of(run, "a.py"), FileStatus::Fail);
    assert_eq!(calls, 1, "files.len()==1 skips isolation");
}

#[test]
fn indeterminate_batch_is_not_attributed() {
    let ws = workspace(&["a.py", "b.py"]);
    let (out, calls) = run_mock(
        base_tool(true),
        &["a.py", "b.py"],
        ws.path(),
        Mode::Lint,
        Invocation::Directory,
        // batch fails, but every file passes individually
        |args, _| {
            if args.len() > 1 {
                result(false, 1)
            } else {
                result(true, 0)
            }
        },
    );
    let run = only(&out);
    assert_eq!(run.status, RunStatus::Fail);
    assert!(!run.attributed, "no single file is to blame");
    assert_eq!(status_of(run, "a.py"), FileStatus::Pass);
    assert_eq!(status_of(run, "b.py"), FileStatus::Pass);
    assert_eq!(calls, 3);
}

#[test]
fn whole_project_fail_shares_result_without_isolation() {
    let ws = workspace(&[]);
    let (out, calls) = run_mock(
        base_tool(false), // pass_files=false
        &["a.py", "b.py"],
        ws.path(),
        Mode::Lint,
        Invocation::Directory,
        |_, _| result(false, 1),
    );
    let run = only(&out);
    assert_eq!(run.status, RunStatus::Fail);
    assert!(run.attributed);
    assert_eq!(status_of(run, "a.py"), FileStatus::Fail);
    assert_eq!(status_of(run, "b.py"), FileStatus::Fail);
    assert_eq!(calls, 1, "whole-project tools are never re-run per file");
}

// --- Exit-code and error mapping ---

#[test]
fn exit_two_batch_is_error_and_skips_isolation() {
    let ws = workspace(&["a.py", "b.py"]);
    let (out, calls) = run_mock(
        base_tool(true),
        &["a.py", "b.py"],
        ws.path(),
        Mode::Lint,
        Invocation::Directory,
        |_, _| result(false, 2),
    );
    let run = only(&out);
    assert_eq!(run.status, RunStatus::Error);
    assert_eq!(status_of(run, "a.py"), FileStatus::Error);
    assert_eq!(calls, 1, "exit 2 short-circuits before isolation");
}

#[test]
fn exit_two_during_isolation_is_error() {
    let ws = workspace(&["a.py", "b.py"]);
    let (out, calls) = run_mock(
        base_tool(true),
        &["a.py", "b.py"],
        ws.path(),
        Mode::Lint,
        Invocation::Directory,
        |args, _| {
            if args.len() > 1 {
                result(false, 1)
            } else if args.contains(&"b.py") {
                result(false, 2)
            } else {
                result(true, 0)
            }
        },
    );
    let run = only(&out);
    assert_eq!(
        run.status,
        RunStatus::Error,
        "any file Error rolls up to Error"
    );
    assert_eq!(status_of(run, "a.py"), FileStatus::Pass);
    assert_eq!(status_of(run, "b.py"), FileStatus::Error);
    assert_eq!(calls, 3);
}

#[test]
fn no_backend_is_error() {
    let ws = workspace(&["a.py", "b.py"]);
    let detection = DetectionResult {
        file_assignments: HashMap::from([(
            LinterId::Python,
            vec![PathBuf::from("a.py"), PathBuf::from("b.py")],
        )]),
    };
    let chain: Vec<Box<dyn Backend>> = vec![];
    let out = run_linters(
        &detection,
        &chain,
        &[base_tool(true)],
        ws.path(),
        Mode::Lint,
        Invocation::Directory,
    );
    let run = only(&out);
    assert_eq!(run.status, RunStatus::Error);
    assert_eq!(status_of(run, "a.py"), FileStatus::Error);
    assert_eq!(status_of(run, "b.py"), FileStatus::Error);
    assert!(run.batch_stderr.contains("no backend"));
}

#[test]
fn backend_run_error_is_error() {
    let ws = workspace(&["a.py"]);
    let (out, _) = run_mock(
        base_tool(true),
        &["a.py"],
        ws.path(),
        Mode::Lint,
        Invocation::Directory,
        |_, _| Err(anyhow::anyhow!("boom")),
    );
    let run = only(&out);
    assert_eq!(run.status, RunStatus::Error);
    assert!(run.batch_stderr.contains("boom"));
}

// --- Format mode: hash-diff attribution ("Option 1") ---

#[test]
fn format_attributes_only_rewritten_files() {
    let ws = workspace(&["a.py", "b.py"]);
    let mut tool = base_tool(true);
    tool.format_args = Some(Vec::new());
    let (out, calls) = run_mock(
        tool,
        &["a.py", "b.py"],
        ws.path(),
        Mode::Format,
        Invocation::Directory,
        |_, ws| {
            std::fs::write(ws.join("a.py"), "reformatted\n").unwrap(); // only a.py changes
            result(true, 0)
        },
    );
    let run = only(&out);
    assert_eq!(run.status, RunStatus::Fail);
    assert!(run.attributed);
    assert_eq!(status_of(run, "a.py"), FileStatus::Fail);
    assert_eq!(status_of(run, "b.py"), FileStatus::Pass);
    assert_eq!(calls, 1, "format uses hash-diff, not per-file re-runs");
}

#[test]
fn format_no_change_passes_despite_nonzero_exit() {
    let ws = workspace(&["a.py"]);
    let mut tool = base_tool(true);
    tool.format_args = Some(Vec::new());
    let (out, calls) = run_mock(
        tool,
        &["a.py"],
        ws.path(),
        Mode::Format,
        Invocation::Directory,
        |_, _| result(false, 1), // exited non-zero but rewrote nothing
    );
    let run = only(&out);
    assert_eq!(run.status, RunStatus::Pass, "Option 1: no rewrite => Pass");
    assert_eq!(status_of(run, "a.py"), FileStatus::Pass);
    assert_eq!(calls, 1);
}

#[test]
fn format_exit_two_is_error_before_change_check() {
    let ws = workspace(&["a.py"]);
    let mut tool = base_tool(true);
    tool.format_args = Some(Vec::new());
    let (out, _) = run_mock(
        tool,
        &["a.py"],
        ws.path(),
        Mode::Format,
        Invocation::Directory,
        |_, ws| {
            std::fs::write(ws.join("a.py"), "changed\n").unwrap(); // even if the file changed
            result(false, 2)
        },
    );
    let run = only(&out);
    assert_eq!(
        run.status,
        RunStatus::Error,
        "exit 2 takes precedence over the change check"
    );
    assert_eq!(status_of(run, "a.py"), FileStatus::Error);
}

#[test]
fn format_check_fails_on_nonzero_without_rewrite() {
    // FormatPrint (format = check) must NOT use the change-hash rule: a misformatted file
    // fails on its non-zero exit even though nothing is rewritten.
    let ws = workspace(&["a.py"]);
    let mut tool = base_tool(true);
    tool.format_print_args = Some(Vec::new());
    let (out, _) = run_mock(
        tool,
        &["a.py"],
        ws.path(),
        Mode::FormatPrint,
        Invocation::Directory,
        |_, _| result(false, 1),
    );
    let run = only(&out);
    assert_eq!(run.status, RunStatus::Fail);
    assert_eq!(status_of(run, "a.py"), FileStatus::Fail);
}

// --- Invocation plumbing ---

#[test]
fn invocation_sets_single_file_flag() {
    let ws = workspace(&["a.py"]);
    let (single, _) = run_mock(
        base_tool(true),
        &["a.py"],
        ws.path(),
        Mode::Lint,
        Invocation::SingleFile,
        |_, _| result(true, 0),
    );
    assert!(single.single_file);

    let ws2 = workspace(&["a.py"]);
    let (dir, _) = run_mock(
        base_tool(true),
        &["a.py"],
        ws2.path(),
        Mode::Lint,
        Invocation::Directory,
        |_, _| result(true, 0),
    );
    assert!(!dir.single_file);
}
