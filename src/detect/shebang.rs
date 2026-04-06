// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::catalog::linter::LinterId;
use crate::detect::rules::SHEBANG_RULES;

/// Extract the interpreter name from a file's shebang line.
///
/// Handles:
/// - `#!/usr/bin/bash` → "bash"
/// - `#!/usr/bin/env bash` → "bash"
/// - `#!/usr/bin/env -S python3 -u` → "python3"
#[must_use]
pub fn detect_shebang(path: &Path) -> Option<String> {
    let file = File::open(path).ok()?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;

    if !line.starts_with("#!") {
        return None;
    }

    let shebang = line[2..].trim();
    let parts: Vec<&str> = shebang.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    let binary = basename(parts[0]);

    if binary == "env" {
        // Skip flags (start with '-') and take first non-flag argument.
        for part in &parts[1..] {
            if !part.starts_with('-') {
                return Some(basename(part).to_string());
            }
        }
        None
    } else {
        Some(binary.to_string())
    }
}

/// Look up a shebang interpreter in the rules table.
#[must_use]
pub fn shebang_to_linter(interp: &str) -> Option<LinterId> {
    SHEBANG_RULES
        .iter()
        .find(|r| r.interp == interp)
        .map(|r| r.linter)
}

fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}
