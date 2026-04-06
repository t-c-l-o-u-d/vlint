// SPDX-License-Identifier: AGPL-3.0-or-later

pub mod content;
pub mod mime;
pub mod pattern;
pub mod rules;
pub mod scoring;
pub mod shebang;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::catalog::linter::{DetectionResult, LinterId};
use crate::detect::content::derive_context;
use crate::detect::rules::{W_MIME, W_SHEBANG};
use crate::detect::scoring::FileScore;

#[must_use]
pub fn detect_all(workspace: &Path, verbose: bool) -> DetectionResult {
    let mut file_assignments: HashMap<LinterId, Vec<PathBuf>> = HashMap::new();
    let mut undetected: Vec<PathBuf> = Vec::new();

    // Project-level markers (ansible directory/glob detection)
    let project_markers = pattern::match_project_markers(workspace);
    if verbose && !project_markers.is_empty() {
        eprintln!("  project markers:");
        for linter in &project_markers {
            eprintln!("    {linter} (directory/glob)");
        }
    }
    for linter in project_markers {
        file_assignments.entry(linter).or_default();
    }

    let files = walk_directory(workspace);
    classify_files(
        workspace,
        &files,
        verbose,
        &mut file_assignments,
        &mut undetected,
    );

    DetectionResult {
        file_assignments,
        undetected,
    }
}

/// Detect and classify an explicit list of paths. Directories are walked;
/// files are included directly. No project-level marker detection.
#[must_use]
pub fn detect_explicit(workspace: &Path, paths: &[PathBuf], verbose: bool) -> DetectionResult {
    let mut file_assignments: HashMap<LinterId, Vec<PathBuf>> = HashMap::new();
    let mut undetected: Vec<PathBuf> = Vec::new();

    let mut files: Vec<PathBuf> = Vec::new();
    for path in paths {
        let abs = if path.is_absolute() {
            path.clone()
        } else {
            workspace.join(path)
        };
        if abs.is_dir() {
            files.extend(walk_directory(&abs));
        } else {
            files.push(abs);
        }
    }
    files.sort();

    classify_files(
        workspace,
        &files,
        verbose,
        &mut file_assignments,
        &mut undetected,
    );

    DetectionResult {
        file_assignments,
        undetected,
    }
}

fn classify_files(
    workspace: &Path,
    files: &[PathBuf],
    verbose: bool,
    file_assignments: &mut HashMap<LinterId, Vec<PathBuf>>,
    undetected: &mut Vec<PathBuf>,
) {
    for path in files {
        let relative = path.strip_prefix(workspace).unwrap_or(path);

        let filename = relative.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let ext = relative.extension().and_then(|e| e.to_str()).unwrap_or("");

        // MIME detection
        let libmagic_mime = mime::detect_mime(path);
        let xdg_mime_result = mime::detect_xdg_mime(path);

        // Skip binary/media files entirely
        if libmagic_mime
            .as_ref()
            .is_some_and(|m| mime::should_skip_binary(m))
        {
            continue;
        }

        let mut score = FileScore::new();

        // MIME votes
        if let Some(linter) = libmagic_mime.as_ref().and_then(|m| mime::mime_to_linter(m)) {
            score.vote(linter, W_MIME);
        } else if let Some(linter) = xdg_mime_result
            .as_ref()
            .and_then(|m| mime::mime_to_linter(m))
        {
            score.vote(linter, W_MIME);
        }

        // Shebang vote
        if let Some(linter) = shebang::detect_shebang(path)
            .as_ref()
            .and_then(|i| shebang::shebang_to_linter(i))
        {
            score.vote(linter, W_SHEBANG);
        }

        // Pattern votes (extension, filename, prefix)
        if let Some((linter, weight)) = pattern::match_extension(ext) {
            score.vote(linter, weight);
        }
        if let Some((linter, weight)) = pattern::match_filename(filename) {
            score.vote(linter, weight);
        }
        if let Some((linter, weight)) = pattern::match_prefix(filename) {
            score.vote(linter, weight);
        }

        // Content heuristics (only when context applies)
        let (context, synthetic_vote) =
            derive_context(libmagic_mime.as_deref(), xdg_mime_result.as_deref(), ext);
        if let Some(vote) = synthetic_vote {
            score.vote(vote.0, vote.1);
        }
        if let Some(ctx) = context {
            for (linter, weight) in content::detect_content(path, ctx) {
                score.vote(linter, weight);
            }
        }

        let winner = score.winner();

        if verbose {
            let scores = score.summary();
            if !scores.is_empty() {
                let winner_str = winner.map_or("none".to_string(), |w| w.to_string());
                let detail: Vec<String> =
                    scores.iter().map(|(id, s)| format!("{id}={s}")).collect();
                eprintln!(
                    "  {}: {} -> {winner_str}",
                    relative.display(),
                    detail.join(", ")
                );
            }
        }

        match winner {
            Some(LinterId::Skip) | None => {
                undetected.push(relative.to_path_buf());
            }
            Some(linter) => {
                file_assignments
                    .entry(linter)
                    .or_default()
                    .push(relative.to_path_buf());
            }
        }
    }
}

fn walk_directory(dir: &Path) -> Vec<PathBuf> {
    ignore::WalkBuilder::new(dir)
        .hidden(false)
        .require_git(false)
        .filter_entry(|e| e.file_name() != ".git")
        .sort_by_file_name(std::cmp::Ord::cmp)
        .build()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_some_and(|ft| ft.is_file()))
        .map(ignore::DirEntry::into_path)
        .collect()
}
