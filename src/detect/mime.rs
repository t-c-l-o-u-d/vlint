// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::Path;
use std::process::Command;

use crate::catalog::linter::LinterId;
use crate::detect::rules::{MIME_RULES, is_binary_mime};

/// Run `file --brief --mime-type` on a path.
#[must_use]
pub fn detect_mime(path: &Path) -> Option<String> {
    let output = Command::new("file")
        .args(["--brief", "--mime-type"])
        .arg(path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let mime = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if mime.is_empty() { None } else { Some(mime) }
}

/// Run `mimetype --brief --magic-only` (XDG MIME) on a path.
#[must_use]
pub fn detect_xdg_mime(path: &Path) -> Option<String> {
    let output = Command::new("mimetype")
        .args(["--brief", "--magic-only"])
        .arg(path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let mime = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if mime.is_empty() { None } else { Some(mime) }
}

/// Look up a MIME type in the rules table.
#[must_use]
pub fn mime_to_linter(mime: &str) -> Option<LinterId> {
    MIME_RULES.iter().find(|r| r.mime == mime).map(|r| r.linter)
}

/// Check if a MIME type indicates a binary/media file that should be skipped entirely.
#[must_use]
pub fn should_skip_binary(mime: &str) -> bool {
    is_binary_mime(mime)
}
