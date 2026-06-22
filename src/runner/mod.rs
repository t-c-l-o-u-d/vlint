// SPDX-License-Identifier: AGPL-3.0-or-later

pub mod format;
pub mod lint;

#[cfg(test)]
mod exec_tests;

use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use crate::backend::{self, Backend};
use crate::catalog::linter::{DetectionResult, LinterId, OwnedToolDef, ToolResult};
use crate::config::resolve::resolve_config;

/// Per-file lint/format outcome.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FileStatus {
    Pass,
    Fail,
    Error,
}

#[derive(Clone)]
pub struct FileOutcome {
    pub path: PathBuf,
    pub status: FileStatus,
}

/// Tool-level rollup status.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RunStatus {
    Pass,
    Fail,
    Error,
}

/// The config file vlint resolved for a tool (its own, or vlint's bundled default).
#[derive(Clone)]
pub struct ConfigRef {
    pub path: String,
    pub is_default: bool,
}

/// The result of running one tool, with per-file attribution.
pub struct ToolRun {
    pub tool_name: String,
    pub status: RunStatus,
    /// Whether the tool runs per file (`pass_files`); whole-project tools share one result.
    pub pass_files: bool,
    /// Logical command line for verbose output: no backend wrapper, no config flag, no file paths.
    pub cli: String,
    /// The resolved config file, if the tool uses one.
    pub config: Option<ConfigRef>,
    /// Output from the batch run, used for single-file display and error messages.
    pub batch_stdout: String,
    pub batch_stderr: String,
    /// False when the batch failed but every per-file re-run passed (not attributable).
    pub attributed: bool,
    pub files: Vec<FileOutcome>,
}

pub struct SkippedLinter {
    pub linter_id: LinterId,
    pub files: Vec<PathBuf>,
}

pub struct LintOutput {
    pub results: Vec<ToolRun>,
    pub skipped: Vec<SkippedLinter>,
    pub single_file: bool,
}

#[derive(Clone, Copy)]
pub enum Mode {
    Lint,
    FormatPrint,
    Format,
}

/// Whether vlint was invoked on exactly one file (vs a directory or multiple files).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Invocation {
    SingleFile,
    Directory,
}

#[must_use]
pub fn run_linters(
    detection: &DetectionResult,
    chain: &[Box<dyn Backend>],
    tools: &[OwnedToolDef],
    workspace: &Path,
    mode: Mode,
    invocation: Invocation,
) -> LintOutput {
    let mut results = Vec::new();
    let mut skipped = Vec::new();

    let mut assignments: Vec<_> = detection.file_assignments.iter().collect();
    assignments.sort_by_key(|(id, _)| format!("{id}"));

    for (&linter_id, files) in assignments {
        if files.is_empty() {
            continue;
        }

        let mut linter_tools: Vec<&OwnedToolDef> =
            tools.iter().filter(|t| t.linter_id == linter_id).collect();
        linter_tools.sort_by_key(|t| t.name.as_str());

        if linter_tools.is_empty() {
            skipped.push(SkippedLinter {
                linter_id,
                files: files.clone(),
            });
            continue;
        }

        let has_format_support = match mode {
            Mode::Lint => true,
            Mode::FormatPrint => linter_tools.iter().any(|t| t.format_print_args.is_some()),
            Mode::Format => linter_tools.iter().any(|t| t.format_args.is_some()),
        };
        if !has_format_support {
            skipped.push(SkippedLinter {
                linter_id,
                files: files.clone(),
            });
            continue;
        }

        for tool in linter_tools {
            if let Some(run) = run_tool(tool, chain, files, workspace, mode) {
                results.push(run);
            }
        }
    }

    LintOutput {
        results,
        skipped,
        single_file: matches!(invocation, Invocation::SingleFile),
    }
}

fn run_tool(
    tool: &OwnedToolDef,
    chain: &[Box<dyn Backend>],
    files: &[PathBuf],
    workspace: &Path,
    mode: Mode,
) -> Option<ToolRun> {
    let args: &[String] = match mode {
        Mode::Lint => &tool.lint_args,
        Mode::FormatPrint => tool.format_print_args.as_deref()?,
        Mode::Format => tool.format_args.as_deref()?,
    };

    let resolved = tool
        .config_precedence
        .as_ref()
        .and_then(|prec| resolve_config(prec, workspace));
    let config_path_str = resolved.as_ref().map(|r| r.path.as_str());
    let config = resolved.as_ref().map(|r| ConfigRef {
        path: r.path.clone(),
        is_default: r.is_default,
    });

    // The verbose `cli:` line: tool command without the config flag or file paths.
    let cli = render_cli(tool, args);

    let Some(backend) = backend::resolve_tool(tool, chain) else {
        return Some(error_run(tool, files, cli, config, "no backend available"));
    };

    let config_path = config_path_str.map(Path::new);
    // Args used for the actual run include the injected config flag.
    let base_args = build_args(tool, args, config_path_str);

    // Format mode: capture per-file hashes so a rewritten file can be flagged. Hash the file at
    // its real location (workspace-relative paths are resolved against the workspace, not vlint's
    // cwd, which is where the tool actually reads/writes it).
    let pre_hashes: Vec<u64> = if matches!(mode, Mode::Format) {
        files.iter().map(|f| hash_one(&workspace.join(f))).collect()
    } else {
        Vec::new()
    };

    // Run the whole batch once.
    let mut batch_args = base_args.clone();
    if tool.pass_files {
        for f in files {
            batch_args.push(rel_path(f, workspace));
        }
    }
    let batch_refs: Vec<&str> = batch_args.iter().map(String::as_str).collect();

    let result = match backend.run(tool, &batch_refs, workspace, config_path) {
        Ok(r) => r,
        Err(e) => return Some(error_run(tool, files, cli, config, &e.to_string())),
    };

    // Format mode: which files the run rewrote (they were not already formatted).
    let changed: Vec<bool> = pre_hashes
        .iter()
        .zip(files)
        .map(|(&pre, f)| hash_one(&workspace.join(f)) != pre)
        .collect();

    // A tool that exits 2 reported a tool execution error, not lint findings.
    if result.exit_code == 2 {
        return Some(make_run(
            tool,
            RunStatus::Error,
            cli,
            config,
            result,
            true,
            outcomes(files, FileStatus::Error),
        ));
    }

    // In apply mode a formatter fails a file only by rewriting it (it was unformatted); a
    // non-zero exit that changed nothing is a lint finding, left to the lint pass to report.
    let batch_success = if matches!(mode, Mode::Format) {
        changed.iter().all(|&c| !c)
    } else {
        result.success
    };
    if batch_success {
        return Some(make_run(
            tool,
            RunStatus::Pass,
            cli,
            config,
            result,
            true,
            outcomes(files, FileStatus::Pass),
        ));
    }

    // Batch failed: attribute to individual files.
    let file_outcomes: Vec<FileOutcome> = if matches!(mode, Mode::Format) {
        format_outcomes(files, &changed)
    } else if tool.pass_files && files.len() > 1 {
        isolate_per_file(tool, backend, &base_args, config_path, files, workspace)
    } else {
        // Whole-project tool, or a single file: the result applies to every matched file.
        outcomes(files, FileStatus::Fail)
    };

    let status = if file_outcomes.iter().any(|o| o.status == FileStatus::Error) {
        RunStatus::Error
    } else {
        RunStatus::Fail
    };
    // Not attributable when the batch failed but every per-file re-run passed.
    let attributed = file_outcomes.iter().any(|o| o.status != FileStatus::Pass);
    Some(make_run(
        tool,
        status,
        cli,
        config,
        result,
        attributed,
        file_outcomes,
    ))
}

fn make_run(
    tool: &OwnedToolDef,
    status: RunStatus,
    cli: String,
    config: Option<ConfigRef>,
    result: ToolResult,
    attributed: bool,
    files: Vec<FileOutcome>,
) -> ToolRun {
    ToolRun {
        tool_name: tool.name.clone(),
        status,
        pass_files: tool.pass_files,
        cli,
        config,
        batch_stdout: result.stdout,
        batch_stderr: result.stderr,
        attributed,
        files,
    }
}

fn isolate_per_file(
    tool: &OwnedToolDef,
    backend: &dyn Backend,
    base_args: &[String],
    config_path: Option<&Path>,
    files: &[PathBuf],
    workspace: &Path,
) -> Vec<FileOutcome> {
    files
        .iter()
        .map(|f| {
            let mut a = base_args.to_vec();
            a.push(rel_path(f, workspace));
            let refs: Vec<&str> = a.iter().map(String::as_str).collect();
            let status = match backend.run(tool, &refs, workspace, config_path) {
                Ok(r) if r.success => FileStatus::Pass,
                Ok(r) if r.exit_code == 2 => FileStatus::Error,
                Ok(_) => FileStatus::Fail,
                Err(_) => FileStatus::Error,
            };
            FileOutcome {
                path: f.clone(),
                status,
            }
        })
        .collect()
}

fn render_cli(tool: &OwnedToolDef, args: &[String]) -> String {
    let parts = build_args(tool, args, None);
    if parts.is_empty() {
        tool.binary_name.clone()
    } else {
        format!("{} {}", tool.binary_name, parts.join(" "))
    }
}

fn format_outcomes(files: &[PathBuf], changed: &[bool]) -> Vec<FileOutcome> {
    files
        .iter()
        .zip(changed)
        .map(|(f, &ch)| FileOutcome {
            path: f.clone(),
            status: if ch {
                FileStatus::Fail
            } else {
                FileStatus::Pass
            },
        })
        .collect()
}

fn outcomes(files: &[PathBuf], status: FileStatus) -> Vec<FileOutcome> {
    files
        .iter()
        .map(|f| FileOutcome {
            path: f.clone(),
            status,
        })
        .collect()
}

fn error_run(
    tool: &OwnedToolDef,
    files: &[PathBuf],
    cli: String,
    config: Option<ConfigRef>,
    message: &str,
) -> ToolRun {
    ToolRun {
        tool_name: tool.name.clone(),
        status: RunStatus::Error,
        pass_files: tool.pass_files,
        cli,
        config,
        batch_stdout: String::new(),
        batch_stderr: message.to_string(),
        attributed: true,
        files: outcomes(files, FileStatus::Error),
    }
}

fn rel_path(file: &Path, workspace: &Path) -> String {
    match file.strip_prefix(workspace) {
        Ok(rel) => rel.to_string_lossy().into_owned(),
        Err(_) => file.to_string_lossy().into_owned(),
    }
}

#[must_use]
pub fn build_args(tool: &OwnedToolDef, flags: &[String], config_path: Option<&str>) -> Vec<String> {
    let mut out = tool.subcommand.clone();
    if let (Some(path), Some(prec)) = (config_path, &tool.config_precedence) {
        out.push(prec.flag.to_string());
        out.push(path.to_string());
    }
    out.extend_from_slice(flags);
    out
}

fn hash_one(path: &Path) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    if let Ok(content) = std::fs::read(path) {
        content.hash(&mut hasher);
    }
    hasher.finish()
}
