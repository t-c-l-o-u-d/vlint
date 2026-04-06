// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

use crate::catalog::linter::LinterId;
use crate::detect::rules::{PATTERN_RULES, PatternKind, W_EXT, W_FILE, W_PREFIX};

struct PatternMaps {
    ext: HashMap<&'static str, LinterId>,
    file: HashMap<&'static str, LinterId>,
    prefixes: Vec<(&'static str, LinterId)>,
    dirs: Vec<(&'static str, LinterId)>,
    globs: Vec<(&'static str, LinterId)>,
}

static MAPS: OnceLock<PatternMaps> = OnceLock::new();

fn maps() -> &'static PatternMaps {
    MAPS.get_or_init(|| {
        let mut ext = HashMap::new();
        let mut file = HashMap::new();
        let mut prefixes = Vec::new();
        let mut dirs = Vec::new();
        let mut globs = Vec::new();

        for rule in PATTERN_RULES {
            match rule.kind {
                PatternKind::Ext => {
                    ext.insert(rule.pattern, rule.linter);
                }
                PatternKind::File => {
                    file.insert(rule.pattern, rule.linter);
                }
                PatternKind::Prefix => prefixes.push((rule.pattern, rule.linter)),
                PatternKind::Dir => dirs.push((rule.pattern, rule.linter)),
                PatternKind::Glob => globs.push((rule.pattern, rule.linter)),
            }
        }

        PatternMaps {
            ext,
            file,
            prefixes,
            dirs,
            globs,
        }
    })
}

#[must_use]
pub fn match_extension(ext: &str) -> Option<(LinterId, u32)> {
    maps().ext.get(ext).map(|&id| (id, W_EXT))
}

#[must_use]
pub fn match_filename(name: &str) -> Option<(LinterId, u32)> {
    maps().file.get(name).map(|&id| (id, W_FILE))
}

#[must_use]
pub fn match_prefix(name: &str) -> Option<(LinterId, u32)> {
    for &(prefix, linter) in &maps().prefixes {
        if name == prefix || name.starts_with(&format!("{prefix}.")) {
            return Some((linter, W_PREFIX));
        }
    }
    None
}

#[must_use]
pub fn match_project_markers(workspace: &Path) -> Vec<LinterId> {
    let mut found = Vec::new();
    let m = maps();

    for &(dir, linter) in &m.dirs {
        if workspace.join(dir).is_dir() {
            found.push(linter);
        }
    }

    for &(pattern, linter) in &m.globs {
        if has_glob_match(workspace, pattern) {
            found.push(linter);
        }
    }

    found
}

fn has_glob_match(workspace: &Path, pattern: &str) -> bool {
    let Some((dir, file_pattern)) = pattern.rsplit_once('/') else {
        return false;
    };
    let dir_path = workspace.join(dir);
    if !dir_path.is_dir() {
        return false;
    }
    let Some(ext) = file_pattern.strip_prefix("*.") else {
        return false;
    };
    let Ok(entries) = std::fs::read_dir(&dir_path) else {
        return false;
    };
    entries
        .filter_map(Result::ok)
        .any(|e| e.path().extension().is_some_and(|e| e == ext))
}
