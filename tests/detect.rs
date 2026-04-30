// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fs;
use std::io::Write;

use tempfile::{NamedTempFile, TempDir};
use vlint::catalog::linter::LinterId;
use vlint::detect;
use vlint::detect::content::{derive_context, detect_content};
use vlint::detect::pattern::{match_extension, match_filename, match_prefix};
use vlint::detect::rules::ContentContext;
use vlint::detect::scoring::FileScore;
use vlint::detect::shebang::{detect_shebang, shebang_to_linter};

// --- scoring ---

#[test]
fn highest_score_wins() {
    let mut s = FileScore::new();
    s.vote(LinterId::Bash, 3);
    s.vote(LinterId::Bash, 3);
    s.vote(LinterId::Bash, 1);
    s.vote(LinterId::Containerfile, 1);
    assert_eq!(s.winner(), Some(LinterId::Bash));
}

#[test]
fn skip_loses_tie() {
    let mut s = FileScore::new();
    s.vote(LinterId::Skip, 1);
    s.vote(LinterId::Rust, 1);
    assert_eq!(s.winner(), Some(LinterId::Rust));
}

#[test]
fn ansible_needs_two_keywords() {
    let mut s = FileScore::new();
    s.vote(LinterId::Yaml, 3);
    s.vote(LinterId::Yaml, 1);
    s.vote(LinterId::Ansible, 3);
    assert_eq!(s.winner(), Some(LinterId::Yaml));
}

#[test]
fn ansible_two_keywords_wins() {
    let mut s = FileScore::new();
    s.vote(LinterId::Yaml, 3);
    s.vote(LinterId::Yaml, 1);
    s.vote(LinterId::Ansible, 3);
    s.vote(LinterId::Ansible, 3);
    assert_eq!(s.winner(), Some(LinterId::Ansible));
}

#[test]
fn max_weight_tiebreaker() {
    let mut s = FileScore::new();
    s.vote(LinterId::Bash, 3);
    s.vote(LinterId::Bash, 1);
    s.vote(LinterId::Css, 1);
    s.vote(LinterId::Css, 1);
    s.vote(LinterId::Css, 1);
    s.vote(LinterId::Css, 1);
    assert_eq!(s.winner(), Some(LinterId::Bash));
}

// --- shebang ---

fn file_with_first_line(line: &str) -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "{line}").unwrap();
    f
}

#[test]
fn shebang_bash() {
    let f = file_with_first_line("#!/usr/bin/bash");
    assert_eq!(detect_shebang(f.path()), Some("bash".to_string()));
    assert_eq!(shebang_to_linter("bash"), Some(LinterId::Bash));
}

#[test]
fn shebang_python3() {
    let f = file_with_first_line("#!/usr/bin/python3");
    assert_eq!(detect_shebang(f.path()), Some("python3".to_string()));
    assert_eq!(shebang_to_linter("python3"), Some(LinterId::Python));
}

#[test]
fn shebang_env_bash() {
    let f = file_with_first_line("#!/usr/bin/env bash");
    assert_eq!(detect_shebang(f.path()), Some("bash".to_string()));
}

#[test]
fn shebang_env_with_flags() {
    let f = file_with_first_line("#!/usr/bin/env -S python3 -u");
    assert_eq!(detect_shebang(f.path()), Some("python3".to_string()));
}

#[test]
fn no_shebang_returns_none() {
    let f = file_with_first_line("# just a comment");
    assert_eq!(detect_shebang(f.path()), None);
}

#[test]
fn unknown_interpreter_no_linter() {
    assert_eq!(shebang_to_linter("perl"), None);
}

// --- pattern matching ---

#[test]
fn extension_yaml_matches() {
    let (linter, _) = match_extension("yaml").expect("yaml extension should match");
    assert_eq!(linter, LinterId::Yaml);
}

#[test]
fn extension_yml_matches_yaml() {
    let (linter, _) = match_extension("yml").expect("yml extension should match");
    assert_eq!(linter, LinterId::Yaml);
}

#[test]
fn extension_rs_matches_rust() {
    let (linter, _) = match_extension("rs").expect("rs extension should match");
    assert_eq!(linter, LinterId::Rust);
}

#[test]
fn extension_unknown_no_match() {
    assert!(match_extension("xyz123").is_none());
}

#[test]
fn filename_bashrc_matches_bash() {
    let (linter, _) = match_filename(".bashrc").expect(".bashrc should match");
    assert_eq!(linter, LinterId::Bash);
}

#[test]
fn prefix_dockerfile_matches_containerfile() {
    let (linter, _) = match_prefix("Dockerfile").expect("Dockerfile prefix should match");
    assert_eq!(linter, LinterId::Containerfile);
}

#[test]
fn prefix_dockerfile_dot_matches_containerfile() {
    let (linter, _) = match_prefix("Dockerfile.dev").expect("Dockerfile.dev prefix should match");
    assert_eq!(linter, LinterId::Containerfile);
}

// --- detect_explicit (full pipeline) ---

fn workspace_with_file(name: &str, content: &str) -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(name);
    fs::write(&path, content).unwrap();
    (dir, path)
}

#[test]
fn detect_explicit_rs_file_is_rust() {
    let (workspace, path) = workspace_with_file("main.rs", "fn main() {}");
    let result = detect::detect_explicit(workspace.path(), &[path], false);
    assert!(
        result.file_assignments.contains_key(&LinterId::Rust),
        "expected Rust detection"
    );
}

#[test]
fn detect_explicit_yaml_file_is_yaml() {
    let (workspace, path) = workspace_with_file("config.yaml", "key: value\n");
    let result = detect::detect_explicit(workspace.path(), &[path], false);
    assert!(
        result.file_assignments.contains_key(&LinterId::Yaml),
        "expected Yaml detection"
    );
}

#[test]
fn detect_explicit_bash_shebang_is_bash() {
    let (workspace, path) = workspace_with_file("script", "#!/usr/bin/env bash\necho hello\n");
    let result = detect::detect_explicit(workspace.path(), &[path], false);
    assert!(
        result.file_assignments.contains_key(&LinterId::Bash),
        "expected Bash detection"
    );
}

#[test]
fn detect_explicit_py_with_html_pattern_is_python() {
    // Regression test for issue #3: a short .py file with HTML-shaped content
    // (e.g. an HTML-scraping regex) used to be tagged text/html by libmagic and
    // misclassified as Html. The .py extension must win.
    let content = "import re\n\
        \n\
        _TAG_RE = re.compile(r\"<title[^>]*>([^<]+)</title>\", re.IGNORECASE)\n\
        \n\
        # Module parses an HTML <title> element and the surrounding chrome.\n\
        # The <foo> in this comment is unrelated.\n\
        \n\
        def scrape(html: str):\n\
            m = _TAG_RE.search(html)\n\
            return m.group(1) if m else None\n";
    let (workspace, path) = workspace_with_file("scraper.py", content);
    let result = detect::detect_explicit(workspace.path(), &[path], false);
    assert!(
        result.file_assignments.contains_key(&LinterId::Python),
        "expected Python detection, got {:?}",
        result.file_assignments.keys().collect::<Vec<_>>()
    );
    assert!(
        !result.file_assignments.contains_key(&LinterId::Html),
        ".py must not be classified as Html, got {:?}",
        result.file_assignments.keys().collect::<Vec<_>>()
    );
}

#[test]
fn detect_explicit_html_file_is_html() {
    // Confirms that .html files still classify as Html when MIME and extension
    // agree (the suppression in issue #3's fix only triggers on disagreement).
    let content = "<!DOCTYPE html>\n<html><head><title>x</title></head><body>hi</body></html>\n";
    let (workspace, path) = workspace_with_file("page.html", content);
    let result = detect::detect_explicit(workspace.path(), &[path], false);
    assert!(
        result.file_assignments.contains_key(&LinterId::Html),
        "expected Html detection, got {:?}",
        result.file_assignments.keys().collect::<Vec<_>>()
    );
}

#[test]
fn detect_explicit_nonexistent_file_is_undetected() {
    let workspace = TempDir::new().unwrap();
    let missing = workspace.path().join("does_not_exist.xyz");
    let result = detect::detect_explicit(workspace.path(), &[missing], false);
    assert!(result.file_assignments.is_empty());
}

// --- content detection ---

#[test]
fn derive_context_yaml_mime_gives_yaml() {
    let (ctx, vote) = derive_context(Some("application/yaml"), None, "yaml");
    assert_eq!(ctx, Some(ContentContext::Yaml));
    assert!(vote.is_none());
}

#[test]
fn derive_context_text_plain_with_yaml_ext_gives_yaml_and_vote() {
    let (ctx, vote) = derive_context(Some("text/plain"), None, "yaml");
    assert_eq!(ctx, Some(ContentContext::Yaml));
    assert!(vote.is_some());
}

#[test]
fn derive_context_unknown_mime_gives_none() {
    let (ctx, _) = derive_context(Some("application/pdf"), None, "pdf");
    assert!(ctx.is_none());
}

#[test]
fn detect_content_ansible_keywords_vote_ansible() {
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "- name: Install packages").unwrap();
    writeln!(f, "  become: true").unwrap();
    writeln!(f, "  tasks:").unwrap();
    let votes = detect_content(f.path(), ContentContext::Yaml);
    let linters: Vec<LinterId> = votes.into_iter().map(|(l, _)| l).collect();
    assert!(
        linters.contains(&LinterId::Ansible),
        "expected Ansible content vote"
    );
}
