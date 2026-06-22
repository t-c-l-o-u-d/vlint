// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

pub const VLINT_1: &[u8] = br#".TH VLINT 1 "" "vlint" "User Commands"
.SH NAME
vlint \- lint and format code using native tools or containers
.SH SYNOPSIS
.B vlint
[\fB\-v\fR]
[\fB\-c\fR \fIFILE\fR]
[\fB\-t\fR]
[\fB\-\-\fR]
[\fIFILE|DIR\fR]...
.SH DESCRIPTION
vlint auto-detects file types using weighted consensus scoring and runs
the right linters.
Native tools on PATH are preferred, with automatic fallback to containers
via podman or docker.
.PP
Each tool independently resolves through the backend chain.
shellcheck might run natively while golangci-lint falls back to a container.
.SH OPTIONS
.TP
.BR \-t ", " \-\-tools
List all tools and how each resolves through the backend chain.
.TP
.BR \-v ", " \-\-verbose
Verbose output: each tool's command line, resolved config file, and a
PASS or FAIL line for every file.
.TP
.BR \-d ", " \-\-debug
Debug output: per-file detection ranking and skipped linters. Intentionally
omitted from
.BR \-\-help .
.TP
\fB\-c\fR \fIFILE\fR, \fB\-\-config\fR=\fIFILE\fR
Path to a vlint config file.
Default: \fI$XDG_CONFIG_HOME/vlint/config.ini\fR.
.TP
.BR \-h ", " \-\-help
Print help.
.TP
.BR \-V ", " \-\-version
Print version.
.SH EXIT CODES
.TP
.B 0
All tools passed.
.TP
.B 1
One or more tools reported lint errors.
.TP
.B 2
Runtime or tool execution error.
.SH OUTPUT
By default vlint reports only failures, grouped under each tool, and prints
nothing when everything passes.
Run vlint on a failing file, or with \fB\-v\fR, to see more detail.
When linting a single file it prints the tool name followed by the tool's
own output and a PASS or FAIL line.
With
.B \-v
it prints, for every tool, the resolved command line, the config file in
use, and a PASS or FAIL line for each file.
.SH SEE ALSO
.BR vlint (5)
"#;

pub const VLINT_5: &[u8] = br#".TH VLINT 5 "" "vlint" "File Formats"
.SH NAME
vlint \- vlint configuration file format
.SH DESCRIPTION
vlint reads configuration from an INI file.
.SH FILE LOCATIONS
.TP
.B ~/.config/vlint/config.ini
User-level defaults. Respects $XDG_CONFIG_HOME.
.SH FORMAT
Standard INI format. All options are commented out showing their defaults.
.PP
.RS 4
.nf
[vlint]
# Automatically update vlint to the latest release on startup.
#auto_update = true

# Install man pages to the system or user man directory on startup.
#man_page_install = true

# Format mode:
# check | show what formatters would change and exit
# apply | apply formatter changes and continue with linting
#format = apply

# Color output:
# auto   | use color if the terminal supports it
# always | always output color
# never  | disable color output
#color = auto

# Terminal color for passing tools.
#pass_color = green

# Terminal color for failing tools.
#fail_color = red

# Terminal color for skipped tools.
#skip_color = yellow

# Terminal color for tool names.
#tool_color = blue

# Container registry hostname.
#registry = ghcr.io/t-c-l-o-u-d/vlint-images

# Prefix prepended to all container image names.
#image_prefix =

# Container image tag to pull.
#tag = latest

# Backend:
# auto   | auto-detect in order: path, podman, docker
# path   | run tools ONLY from PATH
# podman | run tools ONLY via Podman
# docker | run tools ONLY via Docker
#backend = auto

# Per-tool overrides (one section per tool, e.g. shellcheck, yamllint):
#[tool:shellcheck]
#binary_name = shellcheck
#container_image = shellcheck
#lint_args = --external-sources
#format_print_args = --diff
#format_args = --fix
#env_vars = KEY=value
#container_env_vars = KEY=value
.fi
.RE
.SH OPTIONS
.TP 4
.B auto_update=
Controls automatic self-update on startup. Takes "true" or "false". If enabled,
vlint checks for a newer release and updates itself before running. Defaults to
"true".

Added in version 0.0.1.
.TP 4
.B man_page_install=
Controls automatic man page installation on startup. Takes "true" or "false".
If enabled, vlint writes its man pages to \fI/usr/local/share/man\fR when run
as root, or \fI$XDG_DATA_HOME/man\fR (default: \fI~/.local/share/man\fR) otherwise.
Defaults to "true".

Added in version 0.0.1.
.TP 4
.B format=
Configures the format mode. Takes one of "check" or "apply". If set to
"check", each formatter runs in dry-run mode to show what it would change,
then vlint exits without linting. If set to "apply", formatter changes are
written to disk silently (never reported, and they do not affect the exit
code) and then linting proceeds. Defaults to unset (lint only).

Added in version 0.0.1.
.TP 4
.B backend=
Selects the execution backend. Takes one of "auto", "path", "podman",
or "docker". If set to "auto", backends are probed in order: PATH,
podman, docker. If set to any other value, only that backend is
used and tools that cannot run on it are skipped. Defaults to "auto".

Added in version 0.0.1.
.TP 4
.B color=
Controls color output. Takes one of "auto", "always", or "never". If set to
"auto", color is enabled when standard output is a terminal. Defaults to "auto".

Added in version 0.0.1.
.TP 4
.B pass_color=\fR, \fBfail_color=\fR, \fBskip_color=\fR, \fBtool_color=\fR
Sets the ANSI color for PASS, FAIL, SKIPPED, and tool name output respectively.
Takes a color name: "red", "green", "yellow", "blue", "magenta", "cyan",
"white", or any bright variant ("bright-red", "bright-green", etc.). Defaults
to "green", "red", "yellow", and "blue" respectively.

Added in version 0.0.1.
.TP 4
.B registry=
Overrides the container image registry hostname used when pulling images.
Defaults to "ghcr.io/t-c-l-o-u-d/vlint-images".

Added in version 0.0.1.
.TP 4
.B image_prefix=
A prefix prepended to all container image names, useful for routing images
through a local mirror or private namespace. For example, setting "myorg/"
results in images resolved as "registry/myorg/shellcheck:tag". Defaults to
no prefix.

Added in version 0.0.1.
.TP 4
.B tag=
The container image tag to pull when a tool is not available on PATH. Defaults
to "latest". The special value "today" resolves to the current date as
\fIYYYY-MM-DD\fR at runtime, matching the dated tags produced by the
vlint-images local build scripts.

Added in version 0.0.1.
.SH PER-TOOL OVERRIDES
Individual tools can be configured under \fB[tool:\fIname\fB]\fR sections.
.TP 4
.B binary_name=
Overrides the binary name used to invoke the tool on PATH.
.TP 4
.B container_image=
Overrides the container image name used when running via a container backend.
.TP 4
.B lint_args=
Overrides the arguments passed to the tool in lint mode.
.TP 4
.B format_print_args=
Overrides the arguments passed to the tool in format:print mode. When unset,
the tool is skipped in format:print mode. Tools that support this mode use a
diff or dry-run flag (e.g. \fB--diff\fR) to show what would change without
modifying files.
.TP 4
.B format_args=
Overrides the arguments passed to the tool in format:fix mode. When unset, the
tool is skipped in format:fix mode.
.TP 4
.B env_vars=
Whitespace-separated \fIKEY=value\fR pairs set in the environment when running
the tool on PATH.
.TP 4
.B container_env_vars=
Whitespace-separated \fIKEY=value\fR pairs set in the environment when running
the tool via a container backend.
.SH SEE ALSO
.BR vlint (1)
"#;

/// # Errors
///
/// Returns an error if a man page directory cannot be created or the file cannot be written.
pub fn install() -> anyhow::Result<()> {
    let (man1_dir, man5_dir) = dirs();
    install_if_changed(&man1_dir.join("vlint.1"), VLINT_1)?;
    install_if_changed(&man5_dir.join("vlint.5"), VLINT_5)?;
    Ok(())
}

fn install_if_changed(path: &PathBuf, content: &[u8]) -> anyhow::Result<()> {
    if std::fs::read(path).ok().as_deref() == Some(content) {
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, content)?;
    Ok(())
}

fn dirs() -> (PathBuf, PathBuf) {
    let is_root = unsafe { libc::geteuid() } == 0;
    if is_root {
        (
            PathBuf::from("/usr/local/share/man/man1"),
            PathBuf::from("/usr/local/share/man/man5"),
        )
    } else {
        let base = std::env::var_os("XDG_DATA_HOME").map_or_else(
            || {
                let home = std::env::var_os("HOME").unwrap_or_default();
                PathBuf::from(home).join(".local/share")
            },
            PathBuf::from,
        );
        (base.join("man/man1"), base.join("man/man5"))
    }
}
