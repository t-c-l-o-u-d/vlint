// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::backend::Backend;
use crate::catalog;
use crate::catalog::linter::{DetectionResult, OwnedToolDef};
use crate::color;
use crate::runner::LintOutput;

pub fn print_detection_summary(detection: &DetectionResult) {
    println!("Detection results:");
    let mut entries: Vec<_> = detection
        .file_assignments
        .iter()
        .filter(|(_, files)| !files.is_empty())
        .collect();
    entries.sort_by_key(|(id, _)| format!("{id}"));

    for (linter_id, files) in &entries {
        let available = catalog::has_tools_for(**linter_id);
        let tag = if available { "" } else { " (no tools)" };
        println!("  {linter_id}: {} file(s){tag}", files.len());
    }

    if !detection.undetected.is_empty() {
        println!("  skipped: {} file(s)", detection.undetected.len());
    }
    println!();
}

#[must_use]
pub fn print_results(output: &LintOutput, verbose: bool) -> u8 {
    if output.results.is_empty() && output.skipped.is_empty() {
        println!("No linters ran.");
        return 0;
    }

    println!();
    println!("Summary:");

    let mut any_fail = false;
    let mut any_error = false;

    let mut results: Vec<_> = output.results.iter().collect();
    results.sort_by_key(|r| format!("{}", r.linter_id));

    for run in &results {
        println!("  {}:", run.linter_id);
        let mut tools: Vec<_> = run.tool_results.iter().collect();
        tools.sort_by_key(|t| t.tool_name.as_str());
        for tool in &tools {
            let status = if tool.exit_code == 2 {
                any_error = true;
                color::error("ERROR")
            } else if tool.success {
                color::pass("PASS")
            } else {
                any_fail = true;
                color::fail("FAIL")
            };
            println!("    {status}: {}", color::tool(&tool.tool_name));
        }
    }

    let mut skipped: Vec<_> = output.skipped.iter().collect();
    skipped.sort_by_key(|s| format!("{}", s.linter_id));
    for s in &skipped {
        println!("  {}:", s.linter_id);
        println!("    {}: {} file(s)", color::skip("SKIPPED"), s.files.len());
        if verbose {
            for f in &s.files {
                println!("      - {}", f.display());
            }
        }
    }

    if any_error { 2 } else { u8::from(any_fail) }
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
