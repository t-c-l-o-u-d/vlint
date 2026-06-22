// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt::Write as _;

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
    print!("{}", render(output, verbose));
    exit_code(output)
}

/// Render the results block (failures, or per-file detail in verbose) as a string, without the
/// trailing failure hint. Exposed so tests can assert on the exact rendered output.
#[must_use]
pub fn render(output: &LintOutput, verbose: bool) -> String {
    let mut out = String::new();
    if output.single_file {
        render_single_file(&mut out, output, verbose);
    } else if verbose {
        render_directory_verbose(&mut out, output);
    } else {
        render_directory_regular(&mut out, output);
    }
    out
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

fn render_directory_regular(out: &mut String, output: &LintOutput) {
    for tool in &output.results {
        match tool.status {
            RunStatus::Pass => {}
            RunStatus::Error => {
                writeln!(out, "{}", color::tool(&tool.tool_name)).unwrap();
                writeln!(out, "  {}", error_detail(tool)).unwrap();
            }
            RunStatus::Fail => {
                writeln!(out, "{}", color::tool(&tool.tool_name)).unwrap();
                if tool.pass_files && tool.attributed {
                    for f in &tool.files {
                        if f.status == FileStatus::Fail {
                            writeln!(out, "  {}: {}", color::fail("FAIL"), f.path.display())
                                .unwrap();
                        }
                    }
                } else {
                    // Whole-project tool, or a failure not attributable to one file.
                    writeln!(out, "  {}", color::fail("FAIL")).unwrap();
                }
            }
        }
    }
}

fn render_directory_verbose(out: &mut String, output: &LintOutput) {
    for tool in &output.results {
        render_verbose_header(out, tool);
        match tool.status {
            RunStatus::Error => writeln!(out, "  {}", error_detail(tool)).unwrap(),
            RunStatus::Fail if !tool.attributed => {
                // Aggregate-only failure (e.g. mypy duplicate-module): no single file is to blame,
                // and re-running any file on its own would pass -- so single-file mode cannot
                // surface the cause. The batch output is shown here because this is the only place
                // the user can see why the tool failed.
                writeln!(
                    out,
                    "  {} (not attributable to a single file)",
                    color::fail("FAIL")
                )
                .unwrap();
                let cols = terminal_columns();
                append_indented(out, &tool.batch_stdout, "  ", cols);
                append_indented(out, &tool.batch_stderr, "  ", cols);
            }
            _ => {
                for f in &tool.files {
                    writeln!(out, "  {}: {}", file_label(f.status), f.path.display()).unwrap();
                }
            }
        }
    }
}

fn render_single_file(out: &mut String, output: &LintOutput, verbose: bool) {
    for tool in &output.results {
        if verbose {
            render_verbose_header(out, tool);
        } else {
            writeln!(out, "{}", color::tool(&tool.tool_name)).unwrap();
        }
        let cols = terminal_columns();
        // Show the tool's own output for every outcome -- including ERROR, whose diagnostic may be
        // on stdout (e.g. a config-parse error) rather than stderr.
        append_indented(out, &tool.batch_stdout, "  ", cols);
        append_indented(out, &tool.batch_stderr, "  ", cols);
        let label = match tool.status {
            RunStatus::Error => color::error("ERROR"),
            RunStatus::Pass => color::pass("PASS"),
            RunStatus::Fail => color::fail("FAIL"),
        };
        writeln!(out, "  {label}").unwrap();
    }
}

/// Verbose per-tool header: the tool name, its `cli:` line, and the resolved `config:` line.
fn render_verbose_header(out: &mut String, tool: &ToolRun) {
    writeln!(out, "{}", color::tool(&tool.tool_name)).unwrap();
    writeln!(out, "  cli: {}", tool.cli).unwrap();
    if let Some(cfg) = &tool.config {
        let suffix = if cfg.is_default {
            " (vlint default)"
        } else {
            ""
        };
        writeln!(out, "  config: {}{suffix}", abbreviate(&cfg.path)).unwrap();
    }
}

/// Render the linters that were skipped (no tools, or no format support). Shown only under `--debug`.
#[must_use]
pub fn render_skipped(output: &LintOutput) -> String {
    let mut skipped: Vec<_> = output.skipped.iter().collect();
    skipped.sort_by_key(|s| format!("{}", s.linter_id));
    let mut out = String::new();
    for s in &skipped {
        writeln!(
            out,
            "{}: {} ({} file(s))",
            color::skip("SKIPPED"),
            s.linter_id,
            s.files.len()
        )
        .unwrap();
        for f in &s.files {
            writeln!(out, "  - {}", f.display()).unwrap();
        }
    }
    out
}

/// The hint that tells the user how to see more detail about failures, or an empty string when none
/// is warranted. Suppressed in single-file mode (the tool's own output is already shown) and when
/// nothing failed. Call once, after the final pass (in `format = apply` that is the lint pass), so
/// it is not emitted per pass.
#[must_use]
pub fn render_failure_hint(output: &LintOutput, verbose: bool) -> String {
    let has_failure = output.results.iter().any(|t| t.status == RunStatus::Fail);
    if output.single_file || !has_failure {
        return String::new();
    }
    if verbose {
        "\nRun `vlint <filename>` to see more detail.\n".to_string()
    } else {
        "\nRun `vlint <filename>` or `vlint -v` to see more detail.\n".to_string()
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

/// Abbreviate a leading `$HOME` to `~`, but only on a path-component boundary so a sibling
/// directory that merely shares the prefix (e.g. `/home/userdata` under `HOME=/home/user`) is
/// left untouched.
fn abbreviate(path: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let home = home.trim_end_matches('/');
    if !home.is_empty()
        && (path == home || path.strip_prefix(home).is_some_and(|r| r.starts_with('/')))
    {
        path.replacen(home, "~", 1)
    } else {
        path.to_string()
    }
}

fn terminal_columns() -> Option<usize> {
    use std::io::IsTerminal;
    if !std::io::stdout().is_terminal() {
        return None;
    }
    // SAFETY: `ws` is zeroed before the ioctl, and `ws_col` is read only after the call returns 0.
    unsafe {
        let mut ws: libc::winsize = std::mem::zeroed();
        if libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) == 0 && ws.ws_col > 0 {
            Some(ws.ws_col as usize)
        } else {
            None
        }
    }
}

fn append_indented(out: &mut String, text: &str, indent: &str, max_cols: Option<usize>) {
    let continuation = format!("{indent}  ");
    for line in text.lines() {
        let fits = max_cols.is_none_or(|cols| indent.len() + line.len() <= cols);
        if fits {
            writeln!(out, "{indent}{line}").unwrap();
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
            writeln!(out, "{cur_indent}{}", rest[..take].trim_end()).unwrap();
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
