// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::catalog::linter::LinterId;

// --- Consensus scoring weights ---
pub const W_CONTENT: u32 = 3;
pub const W_MIME: u32 = 3;
pub const W_SHEBANG: u32 = 3;
pub const W_EXT: u32 = 1;
pub const W_FILE: u32 = 1;
pub const W_PREFIX: u32 = 1;

// --- MIME type rules ---

pub struct MimeRule {
    pub mime: &'static str,
    pub linter: LinterId,
}

pub static MIME_RULES: &[MimeRule] = &[
    // file --brief --mime-type detects these from content:
    MimeRule {
        mime: "application/json",
        linter: LinterId::Json,
    },
    MimeRule {
        mime: "text/html",
        linter: LinterId::Html,
    },
    MimeRule {
        mime: "text/x-shellscript",
        linter: LinterId::Bash,
    },
    MimeRule {
        mime: "application/x-shellscript",
        linter: LinterId::Bash,
    },
    MimeRule {
        mime: "text/x-script.python",
        linter: LinterId::Python,
    },
    MimeRule {
        mime: "text/x-python",
        linter: LinterId::Python,
    },
    // mimetype (XDG MIME, --magic-only) adds these:
    MimeRule {
        mime: "application/yaml",
        linter: LinterId::Yaml,
    },
    MimeRule {
        mime: "text/markdown",
        linter: LinterId::Markdown,
    },
    MimeRule {
        mime: "text/css",
        linter: LinterId::Css,
    },
    MimeRule {
        mime: "text/csv",
        linter: LinterId::Csv,
    },
    MimeRule {
        mime: "text/xml",
        linter: LinterId::Skip,
    },
    MimeRule {
        mime: "application/xml",
        linter: LinterId::Skip,
    },
    MimeRule {
        mime: "application/x-pem-file",
        linter: LinterId::Skip,
    },
];

// --- Shebang interpreter rules ---

pub struct ShebangRule {
    pub interp: &'static str,
    pub linter: LinterId,
}

pub static SHEBANG_RULES: &[ShebangRule] = &[
    ShebangRule {
        interp: "bash",
        linter: LinterId::Bash,
    },
    ShebangRule {
        interp: "python",
        linter: LinterId::Python,
    },
    ShebangRule {
        interp: "python3",
        linter: LinterId::Python,
    },
];

// --- Pattern rules (name-based detection) ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternKind {
    Ext,
    File,
    Prefix,
    Dir,
    Glob,
}

pub struct PatternRule {
    pub linter: LinterId,
    pub kind: PatternKind,
    pub pattern: &'static str,
}

pub static PATTERN_RULES: &[PatternRule] = &[
    // ansible (project-level markers)
    PatternRule {
        linter: LinterId::Ansible,
        kind: PatternKind::Dir,
        pattern: "roles",
    },
    PatternRule {
        linter: LinterId::Ansible,
        kind: PatternKind::File,
        pattern: "ansible.cfg",
    },
    PatternRule {
        linter: LinterId::Ansible,
        kind: PatternKind::File,
        pattern: "site.yml",
    },
    PatternRule {
        linter: LinterId::Ansible,
        kind: PatternKind::File,
        pattern: "site.yaml",
    },
    PatternRule {
        linter: LinterId::Ansible,
        kind: PatternKind::Glob,
        pattern: "playbooks/*.yml",
    },
    PatternRule {
        linter: LinterId::Ansible,
        kind: PatternKind::Glob,
        pattern: "playbooks/*.yaml",
    },
    // bash
    PatternRule {
        linter: LinterId::Bash,
        kind: PatternKind::Ext,
        pattern: "bash",
    },
    PatternRule {
        linter: LinterId::Bash,
        kind: PatternKind::Ext,
        pattern: "sh",
    },
    PatternRule {
        linter: LinterId::Bash,
        kind: PatternKind::File,
        pattern: ".bashrc",
    },
    PatternRule {
        linter: LinterId::Bash,
        kind: PatternKind::File,
        pattern: ".bash_profile",
    },
    // containerfile (prefix-based)
    PatternRule {
        linter: LinterId::Containerfile,
        kind: PatternKind::Prefix,
        pattern: "Containerfile",
    },
    PatternRule {
        linter: LinterId::Containerfile,
        kind: PatternKind::Prefix,
        pattern: "Dockerfile",
    },
    // css
    PatternRule {
        linter: LinterId::Css,
        kind: PatternKind::Ext,
        pattern: "css",
    },
    // go
    PatternRule {
        linter: LinterId::Go,
        kind: PatternKind::Ext,
        pattern: "go",
    },
    PatternRule {
        linter: LinterId::Go,
        kind: PatternKind::File,
        pattern: "go.mod",
    },
    PatternRule {
        linter: LinterId::Go,
        kind: PatternKind::File,
        pattern: "go.sum",
    },
    PatternRule {
        linter: LinterId::Css,
        kind: PatternKind::Ext,
        pattern: "scss",
    },
    // csv
    PatternRule {
        linter: LinterId::Csv,
        kind: PatternKind::Ext,
        pattern: "csv",
    },
    // html
    PatternRule {
        linter: LinterId::Html,
        kind: PatternKind::Ext,
        pattern: "html",
    },
    // javascript
    PatternRule {
        linter: LinterId::Javascript,
        kind: PatternKind::Ext,
        pattern: "js",
    },
    PatternRule {
        linter: LinterId::Javascript,
        kind: PatternKind::Ext,
        pattern: "mjs",
    },
    PatternRule {
        linter: LinterId::Javascript,
        kind: PatternKind::Ext,
        pattern: "cjs",
    },
    // json
    PatternRule {
        linter: LinterId::Json,
        kind: PatternKind::Ext,
        pattern: "json",
    },
    // markdown
    PatternRule {
        linter: LinterId::Markdown,
        kind: PatternKind::Ext,
        pattern: "md",
    },
    // mkosi
    PatternRule {
        linter: LinterId::Mkosi,
        kind: PatternKind::File,
        pattern: "mkosi.conf",
    },
    PatternRule {
        linter: LinterId::Mkosi,
        kind: PatternKind::Dir,
        pattern: "mkosi.conf.d",
    },
    PatternRule {
        linter: LinterId::Mkosi,
        kind: PatternKind::Dir,
        pattern: "mkosi.images",
    },
    // python
    PatternRule {
        linter: LinterId::Python,
        kind: PatternKind::Ext,
        pattern: "py",
    },
    // ruby
    PatternRule {
        linter: LinterId::Ruby,
        kind: PatternKind::Ext,
        pattern: "gemspec",
    },
    PatternRule {
        linter: LinterId::Ruby,
        kind: PatternKind::Ext,
        pattern: "rake",
    },
    PatternRule {
        linter: LinterId::Ruby,
        kind: PatternKind::Ext,
        pattern: "rb",
    },
    PatternRule {
        linter: LinterId::Ruby,
        kind: PatternKind::File,
        pattern: "Gemfile",
    },
    PatternRule {
        linter: LinterId::Ruby,
        kind: PatternKind::File,
        pattern: "Rakefile",
    },
    // rust
    PatternRule {
        linter: LinterId::Rust,
        kind: PatternKind::Ext,
        pattern: "rs",
    },
    PatternRule {
        linter: LinterId::Rust,
        kind: PatternKind::File,
        pattern: "Cargo.toml",
    },
    // systemd
    PatternRule {
        linter: LinterId::Systemd,
        kind: PatternKind::Ext,
        pattern: "service",
    },
    PatternRule {
        linter: LinterId::Systemd,
        kind: PatternKind::Ext,
        pattern: "timer",
    },
    PatternRule {
        linter: LinterId::Systemd,
        kind: PatternKind::Ext,
        pattern: "socket",
    },
    PatternRule {
        linter: LinterId::Systemd,
        kind: PatternKind::Ext,
        pattern: "path",
    },
    PatternRule {
        linter: LinterId::Systemd,
        kind: PatternKind::Ext,
        pattern: "mount",
    },
    PatternRule {
        linter: LinterId::Systemd,
        kind: PatternKind::Ext,
        pattern: "target",
    },
    PatternRule {
        linter: LinterId::Systemd,
        kind: PatternKind::Ext,
        pattern: "slice",
    },
    // vim
    PatternRule {
        linter: LinterId::Vim,
        kind: PatternKind::Ext,
        pattern: "vim",
    },
    PatternRule {
        linter: LinterId::Vim,
        kind: PatternKind::File,
        pattern: "vimrc",
    },
    // yaml
    PatternRule {
        linter: LinterId::Yaml,
        kind: PatternKind::Ext,
        pattern: "yml",
    },
    PatternRule {
        linter: LinterId::Yaml,
        kind: PatternKind::Ext,
        pattern: "yaml",
    },
    // --- skip: known non-code extensions ---
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::Ext,
        pattern: "bak",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::Ext,
        pattern: "bu",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::Ext,
        pattern: "build",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::Ext,
        pattern: "cfg",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::Ext,
        pattern: "conf",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::Ext,
        pattern: "crt",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::Ext,
        pattern: "editorconfig",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::Ext,
        pattern: "eot",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::Ext,
        pattern: "gif",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::Ext,
        pattern: "gitattributes",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::Ext,
        pattern: "gitignore",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::Ext,
        pattern: "gotemplate",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::Ext,
        pattern: "ico",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::Ext,
        pattern: "in",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::Ext,
        pattern: "ini",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::Ext,
        pattern: "internal",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::Ext,
        pattern: "j2",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::Ext,
        pattern: "jpg",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::Ext,
        pattern: "jpeg",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::Ext,
        pattern: "locale",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::Ext,
        pattern: "lock",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::Ext,
        pattern: "mp3",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::Ext,
        pattern: "pdf",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::Ext,
        pattern: "placeholder",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::Ext,
        pattern: "png",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::Ext,
        pattern: "pub",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::Ext,
        pattern: "sixel",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::Ext,
        pattern: "svg",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::Ext,
        pattern: "toml",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::Ext,
        pattern: "trivyignore",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::Ext,
        pattern: "ttf",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::Ext,
        pattern: "txt",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::Ext,
        pattern: "webp",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::Ext,
        pattern: "woff",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::Ext,
        pattern: "woff2",
    },
    // --- skip: known non-code filenames ---
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::File,
        pattern: ".ansible-lint",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::File,
        pattern: ".yamllint",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::File,
        pattern: "AUTHORS",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::File,
        pattern: "CHANGELOG",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::File,
        pattern: "COPYING",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::File,
        pattern: "LICENCE",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::File,
        pattern: "LICENSE",
    },
    PatternRule {
        linter: LinterId::Skip,
        kind: PatternKind::File,
        pattern: "Makefile",
    },
];

// --- Content heuristic rules ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentContext {
    Yaml,
    Plain,
}

pub struct ContentRule {
    pub context: ContentContext,
    pub linter: LinterId,
    pub pattern: &'static str,
}

// Patterns ported from bash extended regexes to Rust regex syntax.
pub static CONTENT_RULES: &[ContentRule] = &[
    // ansible heuristics (context: yaml)
    ContentRule {
        context: ContentContext::Yaml,
        linter: LinterId::Ansible,
        pattern: r"^\s*-?\s*become\s*:",
    },
    ContentRule {
        context: ContentContext::Yaml,
        linter: LinterId::Ansible,
        pattern: r"^\s*-?\s*gather_facts\s*:",
    },
    ContentRule {
        context: ContentContext::Yaml,
        linter: LinterId::Ansible,
        pattern: r"^\s*-?\s*tasks\s*:",
    },
    ContentRule {
        context: ContentContext::Yaml,
        linter: LinterId::Ansible,
        pattern: r"^\s*-?\s*handlers\s*:",
    },
    // containerfile heuristics (context: plain)
    ContentRule {
        context: ContentContext::Plain,
        linter: LinterId::Containerfile,
        pattern: r"^FROM\s+\S",
    },
    ContentRule {
        context: ContentContext::Plain,
        linter: LinterId::Containerfile,
        pattern: r"^(RUN|COPY|ADD|CMD|ENTRYPOINT|EXPOSE|WORKDIR|ENV|ARG|LABEL)\s",
    },
];

/// Binary/media MIME types that should be silently skipped before scoring.
#[must_use]
pub fn is_binary_mime(mime: &str) -> bool {
    mime == "application/octet-stream"
        || mime == "application/gzip"
        || mime == "application/zip"
        || mime.starts_with("inode/")
        || mime.starts_with("image/")
        || mime.starts_with("audio/")
        || mime.starts_with("video/")
}
