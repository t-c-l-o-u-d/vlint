// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::OnceLock;

use regex::Regex;

use crate::catalog::linter::LinterId;
use crate::detect::rules::{CONTENT_RULES, ContentContext, W_CONTENT};

const MAX_SCAN_LINES: usize = 50;

static COMPILED_RULES: OnceLock<Vec<CompiledContentRule>> = OnceLock::new();

struct CompiledContentRule {
    context: ContentContext,
    linter: LinterId,
    regex: Regex,
}

fn compiled_rules() -> &'static [CompiledContentRule] {
    COMPILED_RULES.get_or_init(|| {
        CONTENT_RULES
            .iter()
            .map(|r| CompiledContentRule {
                context: r.context,
                linter: r.linter,
                regex: Regex::new(r.pattern)
                    .unwrap_or_else(|e| panic!("bad content rule regex '{}': {e}", r.pattern)),
            })
            .collect()
    })
}

/// Derive the content context from MIME detection results.
///
/// Returns the context for content heuristic matching, plus an optional
/// synthetic MIME vote (linter, weight) for the .yml/.yaml text/plain edge case.
#[must_use]
pub fn derive_context(
    mime: Option<&str>,
    xdg_mime: Option<&str>,
    ext: &str,
) -> (Option<ContentContext>, Option<(LinterId, u32)>) {
    match mime {
        Some("application/yaml") => (Some(ContentContext::Yaml), None),
        Some("text/plain") => {
            if xdg_mime == Some("application/yaml") {
                return (Some(ContentContext::Yaml), None);
            }
            if ext == "yml" || ext == "yaml" {
                return (
                    Some(ContentContext::Yaml),
                    Some((LinterId::Yaml, crate::detect::rules::W_MIME)),
                );
            }
            (Some(ContentContext::Plain), None)
        }
        _ => (None, None),
    }
}

/// Run content heuristics on a file within the given context.
pub fn detect_content(path: &Path, context: ContentContext) -> Vec<(LinterId, u32)> {
    let Ok(file) = File::open(path) else {
        return Vec::new();
    };

    let reader = BufReader::new(file);
    let sample: Vec<String> = reader
        .lines()
        .take(MAX_SCAN_LINES)
        .filter_map(Result::ok)
        .collect();

    let mut votes = Vec::new();

    for rule in compiled_rules() {
        if rule.context != context {
            continue;
        }
        for line in &sample {
            if rule.regex.is_match(line) {
                votes.push((rule.linter, W_CONTENT));
                break;
            }
        }
    }

    votes
}
