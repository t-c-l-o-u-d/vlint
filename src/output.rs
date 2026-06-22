// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::backend::Backend;
use crate::catalog::linter::OwnedToolDef;
use crate::color;
use crate::runner::{FileStatus, LintOutput, RunStatus, ToolRun};

/// Print lint/format results and return the process exit code (2 error / 1 fail / 0 pass).
///
/// Regular mode only reports failures; verbose mode reports every file plus each tool's
/// command line and resolved config. Single-file mode shows the tool's own output.
#[must_use]
pub fn print_results(output: &LintOutput, verbose: bool) -> u8 {
    if output.single_file {
        print_single_file(output, verbose);
    } else if verbose {
        print_directory_verbose(output);
    } else {
        print_directory_regular(output);
    }

    exit_code(output)
}

fn exit_code(output: &LintOutput) -> u8 {
    let mut code = 0;
    for tool in &output.results {
        match tool.status {
            RunStatus::Error => return 2,
            RunStatus::Fail => code = 1,
            RunStatus::Pass => {}
        }
    }
    code
}

fn print_directory_regular(output: &LintOutput) {
    for tool in &output.results {
        match tool.status {
            RunStatus::Pass => {}
            RunStatus::Error => {
                println!("{}", color::tool(&tool.tool_name));
                println!("  {}", error_detail(tool));
            }
            RunStatus::Fail => {
                println!("{}", color::tool(&tool.tool_name));
                if tool.pass_files && tool.attributed {
                    for f in &tool.files {
                        if f.status == FileStatus::Fail {
                            println!("  {}: {}", color::fail("FAIL"), f.path.display());
                        }
                    }
                } else {
                    // Whole-project tool, or a failure not attributable to one file.
                    println!("  {}", color::fail("FAIL"));
                }
            }
        }
    }
}

fn print_directory_verbose(output: &LintOutput) {
    for tool in &output.results {
        print_verbose_header(tool);
        match tool.status {
            RunStatus::Error => println!("  {}", error_detail(tool)),
            RunStatus::Fail if !tool.attributed => {
                println!(
                    "  {} (not attributable to a single file)",
                    color::fail("FAIL")
                );
                let cols = terminal_columns();
                print_indented(&tool.batch_stdout, "  ", cols);
                print_indented(&tool.batch_stderr, "  ", cols);
            }
            _ => {
                for f in &tool.files {
                    println!("  {}: {}", file_label(f.status), f.path.display());
                }
            }
        }
    }
}

fn print_single_file(output: &LintOutput, verbose: bool) {
    for tool in &output.results {
        if verbose {
            print_verbose_header(tool);
        } else {
            println!("{}", color::tool(&tool.tool_name));
        }
        let cols = terminal_columns();
        match tool.status {
            RunStatus::Error => {
                print_indented(&tool.batch_stderr, "  ", cols);
                println!("  {}", color::error("ERROR"));
            }
            RunStatus::Pass => {
                print_indented(&tool.batch_stdout, "  ", cols);
                print_indented(&tool.batch_stderr, "  ", cols);
                println!("  {}", color::pass("PASS"));
            }
            RunStatus::Fail => {
                print_indented(&tool.batch_stdout, "  ", cols);
                print_indented(&tool.batch_stderr, "  ", cols);
                println!("  {}", color::fail("FAIL"));
            }
        }
    }
}

/// Verbose per-tool header: the tool name, its `cli:` line, and the resolved `config:` line.
fn print_verbose_header(tool: &ToolRun) {
    println!("{}", color::tool(&tool.tool_name));
    println!("  cli: {}", tool.cli);
    if let Some(cfg) = &tool.config {
        let suffix = if cfg.is_default {
            " (vlint default)"
        } else {
            ""
        };
        println!("  config: {}{suffix}", abbreviate(&cfg.path));
    }
}

/// List linters that were skipped (no tools, or no format support). Shown only under `--debug`.
pub fn print_skipped(output: &LintOutput) {
    let mut skipped: Vec<_> = output.skipped.iter().collect();
    skipped.sort_by_key(|s| format!("{}", s.linter_id));
    for s in &skipped {
        println!(
            "{}: {} ({} file(s))",
            color::skip("SKIPPED"),
            s.linter_id,
            s.files.len()
        );
        for f in &s.files {
            println!("  - {}", f.display());
        }
    }
}

/// Point the user at single-file mode to see why a file failed, since directory modes
/// only report which files failed, not the tool's error output.
/// Tell the user how to see more detail about failures. Call once, after the final pass
/// (in `format = apply` that is the lint pass), so it is not printed per pass.
pub fn print_failure_hint(output: &LintOutput, verbose: bool) {
    let has_failure = output.results.iter().any(|t| t.status == RunStatus::Fail);
    if output.single_file || !has_failure {
        return;
    }
    if verbose {
        println!("\nRun `vlint <filename>` to see more detail.");
    } else {
        println!("\nRun `vlint <filename>` or `vlint -v` to see more detail.");
    }
}

fn file_label(status: FileStatus) -> String {
    match status {
        FileStatus::Pass => color::pass("PASS"),
        FileStatus::Fail => color::fail("FAIL"),
        FileStatus::Error => color::error("ERROR"),
    }
}

/// Indented ERROR detail (the tool name is already on the header line).
fn error_detail(tool: &ToolRun) -> String {
    let msg = tool.batch_stderr.trim();
    if msg.is_empty() {
        color::error("ERROR")
    } else {
        format!("{}: {msg}", color::error("ERROR"))
    }
}

fn abbreviate(path: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    if !home.is_empty() && path.starts_with(&home) {
        path.replacen(&home, "~", 1)
    } else {
        path.to_string()
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

pub fn print_tool_list(tools: &[OwnedToolDef], chain: &[Box<dyn Backend>], verbose: bool) {
    let mut tools: Vec<&OwnedToolDef> = tools.iter().collect();
    tools.sort_by_key(|t| t.name.as_str());
    println!("Supported tools:");
    for tool in tools {
        let backend = chain
            .iter()
            .find(|b| b.available().is_ok() && b.has_tool(tool).is_ok());
        let resolution = match backend {
            Some(b) => format!("{}", b.kind()),
            None => "unavailable".to_string(),
        };
        let format_tag = if tool.format_print_args.is_some() || tool.format_args.is_some() {
            " [+format]"
        } else {
            ""
        };
        println!(
            "  {} ({}): {resolution}{format_tag}",
            tool.name, tool.linter_id
        );
        if verbose {
            for b in chain {
                if let Err(reason) = b.available() {
                    println!("    x {}: {reason}", b.kind());
                    continue;
                }
                match b.has_tool(tool) {
                    Ok(()) => {
                        println!("    -> {}", b.kind());
                        break;
                    }
                    Err(reason) => println!("    x {}: {reason}", b.kind()),
                }
            }
        }
    }
}
