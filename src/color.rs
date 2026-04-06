// SPDX-License-Identifier: AGPL-3.0-or-later

use std::io::IsTerminal;
use std::sync::OnceLock;

static COLOR_STATE: OnceLock<ColorState> = OnceLock::new();

struct ColorState {
    enabled: bool,
    pass: &'static str,
    fail: &'static str,
    skip: &'static str,
    error: &'static str,
}

const RESET: &str = "\x1b[0m";

pub fn init(mode: &ColorMode, pass_color: &str, fail_color: &str, skip_color: &str) {
    let enabled = match mode {
        ColorMode::Always => true,
        ColorMode::Never => false,
        ColorMode::Auto => {
            std::env::var("TERM").as_deref() != Ok("dumb") && std::io::stdout().is_terminal()
        }
    };

    COLOR_STATE
        .set(ColorState {
            enabled,
            pass: resolve_ansi(pass_color, "\x1b[32m"), // default green
            fail: resolve_ansi(fail_color, "\x1b[31m"), // default red
            skip: resolve_ansi(skip_color, "\x1b[33m"), // default yellow
            error: resolve_ansi("yellow", "\x1b[33m"),
        })
        .ok();
}

fn state() -> &'static ColorState {
    COLOR_STATE.get_or_init(|| ColorState {
        enabled: false,
        pass: "\x1b[32m",
        fail: "\x1b[31m",
        skip: "\x1b[33m",
        error: "\x1b[33m",
    })
}

#[must_use]
pub fn pass(text: &str) -> String {
    colorize(text, state().pass)
}

#[must_use]
pub fn fail(text: &str) -> String {
    colorize(text, state().fail)
}

#[must_use]
pub fn skip(text: &str) -> String {
    colorize(text, state().skip)
}

#[must_use]
pub fn error(text: &str) -> String {
    colorize(text, state().error)
}

fn colorize(text: &str, ansi: &str) -> String {
    if state().enabled {
        format!("{ansi}{text}{RESET}")
    } else {
        text.to_string()
    }
}

fn resolve_ansi(name: &str, default: &'static str) -> &'static str {
    match name.to_lowercase().as_str() {
        "red" => "\x1b[31m",
        "green" => "\x1b[32m",
        "yellow" => "\x1b[33m",
        "blue" => "\x1b[34m",
        "magenta" => "\x1b[35m",
        "cyan" => "\x1b[36m",
        "white" => "\x1b[37m",
        "bright_red" | "bright-red" => "\x1b[91m",
        "bright_green" | "bright-green" => "\x1b[92m",
        "bright_yellow" | "bright-yellow" => "\x1b[93m",
        "bright_blue" | "bright-blue" => "\x1b[94m",
        "bright_magenta" | "bright-magenta" => "\x1b[95m",
        "bright_cyan" | "bright-cyan" => "\x1b[96m",
        "bright_white" | "bright-white" => "\x1b[97m",
        _ => default,
    }
}

#[derive(Debug, Clone, Default)]
pub enum ColorMode {
    #[default]
    Auto,
    Always,
    Never,
}

impl std::str::FromStr for ColorMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "always" => Ok(Self::Always),
            "never" => Ok(Self::Never),
            other => Err(format!(
                "invalid color mode: {other} (use auto, always, or never)"
            )),
        }
    }
}

impl std::fmt::Display for ColorMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auto => write!(f, "auto"),
            Self::Always => write!(f, "always"),
            Self::Never => write!(f, "never"),
        }
    }
}
