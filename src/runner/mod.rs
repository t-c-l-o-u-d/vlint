// SPDX-License-Identifier: AGPL-3.0-or-later

pub mod format;
pub mod lint;

use std::path::{Path, PathBuf};

use crate::backend::{self, Backend};
use crate::catalog::linter::{DetectionResult, LinterId, OwnedToolDef, ToolResult};
use crate::color;
use crate::config::resolve::{ResolvedConfig, resolve_config};

pub struct RunResult {
    pub linter_id: LinterId,
    pub tool_results: Vec<ToolResult>,
}

pub struct SkippedLinter {
    pub linter_id: LinterId,
    pub files: Vec<PathBuf>,
}

pub struct LintOutput {
    pub results: Vec<RunResult>,
    pub skipped: Vec<SkippedLinter>,
}

#[derive(Clone, Copy)]
pub enum Mode {
    Lint,
    FormatPrint,
    Format,
}

#[must_use]
pub fn run_linters(
    detection: &DetectionResult,
    chain: &[Box<dyn Backend>],
    tools: &[OwnedToolDef],
    workspace: &Path,
    mode: Mode,
    verbose: bool,
) -> LintOutput {
    let mut results = Vec::new();
    let mut skipped = Vec::new();

    for (&linter_id, files) in &detection.file_assignments {
        if files.is_empty() {
            continue;
        }

        let linter_tools: Vec<&OwnedToolDef> =
            tools.iter().filter(|t| t.linter_id == linter_id).collect();

        if linter_tools.is_empty() {
            println!(
                "  {}: No tools available for {linter_id} ({} file(s))",
                color::skip("SKIPPED"),
                files.len()
            );
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
            let reason = match mode {
                Mode::FormatPrint if linter_tools.iter().any(|t| t.format_args.is_some()) => {
                    "no format:check support; use format = apply"
                }
                _ => "no format support",
            };
            println!("  {}: {linter_id} ({reason})", color::skip("SKIPPED"));
            skipped.push(SkippedLinter {
                linter_id,
                files: files.clone(),
            });
            continue;
        }

        let mut tool_results = Vec::new();
        for tool in linter_tools {
            if let Some(result) = run_tool(tool, chain, files, workspace, mode, verbose) {
                tool_results.push(result);
            }
        }

        results.push(RunResult {
            linter_id,
            tool_results,
        });
    }

    LintOutput { results, skipped }
}

fn run_tool(
    tool: &OwnedToolDef,
    chain: &[Box<dyn Backend>],
    files: &[PathBuf],
    workspace: &Path,
    mode: Mode,
    verbose: bool,
) -> Option<ToolResult> {
    let args: &[String] = match mode {
        Mode::Lint => &tool.lint_args,
        Mode::FormatPrint => tool.format_print_args.as_deref()?,
        Mode::Format => tool.format_args.as_deref()?,
    };

    let Some(backend) = backend::resolve_tool(tool, chain) else {
        eprintln!("  {}: no backend available", tool.name);
        return Some(ToolResult {
            tool_name: tool.name.clone(),
            success: false,
            stdout: String::new(),
            stderr: "no backend available".to_string(),
            exit_code: 2,
        });
    };

    let resolved = tool
        .config_precedence
        .as_ref()
        .and_then(|prec| resolve_config(prec, workspace));

    let mut final_args = build_args(tool, args, resolved.as_ref().map(|r| r.path.as_str()));
    if tool.pass_files {
        for f in files {
            if let Ok(rel) = f.strip_prefix(workspace) {
                final_args.push(rel.to_string_lossy().into_owned());
            } else {
                final_args.push(f.to_string_lossy().into_owned());
            }
        }
    }
    let arg_refs: Vec<&str> = final_args.iter().map(String::as_str).collect();

    if verbose {
        let backend_str = backend.kind().to_string();
        verbose_line(&tool.name, &backend_str, resolved.as_ref(), &arg_refs);
    }
    println!("  Running {}...", tool.name);

    match backend.run(tool, &arg_refs, workspace) {
        Ok(result) => {
            let status = if result.success {
                color::pass("PASS")
            } else {
                color::fail("FAIL")
            };
            println!("  {status}: {}", tool.name);
            print_tool_output(&result, verbose);
            Some(result)
        }
        Err(e) => {
            println!("  {}: {}: {e}", color::error("ERROR"), tool.name);
            Some(ToolResult {
                tool_name: tool.name.clone(),
                success: false,
                stdout: String::new(),
                stderr: e.to_string(),
                exit_code: 2,
            })
        }
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

fn verbose_line(name: &str, backend: &str, resolved: Option<&ResolvedConfig>, args: &[&str]) {
    let home = std::env::var("HOME").unwrap_or_default();
    match resolved {
        Some(r) => {
            let config_str = if r.is_default {
                "vlint default".to_string()
            } else if !home.is_empty() && r.path.starts_with(&home) {
                r.path.replacen(&home, "~", 1)
            } else {
                r.path.clone()
            };
            eprintln!("  [verbose] {name}: backend={backend}, config={config_str}, args={args:?}");
        }
        None => {
            eprintln!("  [verbose] {name}: backend={backend}, args={args:?}");
        }
    }
}

fn print_tool_output(result: &ToolResult, verbose: bool) {
    if verbose || !result.success {
        for line in result.stdout.lines() {
            println!("    {line}");
        }
        for line in result.stderr.lines() {
            println!("    {line}");
        }
    }
}
