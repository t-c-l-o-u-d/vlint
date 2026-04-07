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
        println!("  {}: no backend available", tool.name);
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

    let config_path = resolved
        .as_ref()
        .map(|r| std::path::Path::new(r.path.as_str()));

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

    let tool_name = color::tool(&tool.name);
    let running_line = if verbose {
        format!(
            "  Running {}...",
            verbose_tag(&tool_name, &backend.kind().to_string(), resolved.as_ref())
        )
    } else {
        format!("  Running {tool_name}...")
    };
    println!("{running_line}");

    match backend.run(tool, &arg_refs, workspace, config_path) {
        Ok(result) => {
            let status = if result.success {
                color::pass("PASS")
            } else {
                color::fail("FAIL")
            };
            print_tool_output(&result, verbose);
            println!("  {status}: {tool_name}");
            Some(result)
        }
        Err(e) => {
            println!("  {}: {tool_name}: {e}", color::error("ERROR"));
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

fn verbose_tag(name: &str, backend: &str, resolved: Option<&ResolvedConfig>) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let config_str = resolved.map(|r| {
        if r.is_default {
            "vlint default".to_string()
        } else if !home.is_empty() && r.path.starts_with(&home) {
            r.path.replacen(&home, "~", 1)
        } else {
            r.path.clone()
        }
    });
    match config_str {
        Some(c) => format!("{name} [{backend}, {c}]"),
        None => format!("{name} [{backend}]"),
    }
}

fn terminal_columns() -> Option<usize> {
    use std::io::IsTerminal;
    if !std::io::stdout().is_terminal() {
        return None;
    }
    unsafe {
        let mut ws: libc::winsize = std::mem::zeroed();
        if libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) == 0 && ws.ws_col > 0 {
            Some(ws.ws_col as usize)
        } else {
            None
        }
    }
}

fn print_indented(text: &str, indent: &str, max_cols: Option<usize>) {
    let continuation = format!("{indent}  ");
    for line in text.lines() {
        let fits = max_cols.is_none_or(|cols| indent.len() + line.len() <= cols);
        if fits {
            println!("{indent}{line}");
            continue;
        }
        let avail = max_cols.unwrap().saturating_sub(indent.len());
        let cont_avail = max_cols.unwrap().saturating_sub(continuation.len());
        let mut rest = line;
        let mut first = true;
        while !rest.is_empty() {
            let (cur_indent, cur_avail) = if first {
                (indent, avail)
            } else {
                (continuation.as_str(), cont_avail)
            };
            let take = if rest.len() <= cur_avail {
                rest.len()
            } else if let Some(pos) = rest[..cur_avail].rfind(' ') {
                pos + 1
            } else {
                cur_avail
            };
            println!("{cur_indent}{}", rest[..take].trim_end());
            rest = rest[take..].trim_start_matches(' ');
            first = false;
        }
    }
}

fn print_tool_output(result: &ToolResult, verbose: bool) {
    if verbose || !result.success {
        let cols = terminal_columns();
        print_indented(&result.stdout, "    ", cols);
        print_indented(&result.stderr, "    ", cols);
    }
}
