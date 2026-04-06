// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::{Path, PathBuf};

use crate::catalog::linter::{ConfigLocation, ConfigPrecedence, WalkStop};

pub struct ResolvedConfig {
    pub path: String,
    pub is_default: bool,
}

/// Walk the precedence chain and return the first config found on the host.
/// Falls back to writing vlint's embedded default to the XDG cache directory.
/// Returns `None` only if the cache write also fails (e.g. no writable filesystem).
#[must_use]
pub fn resolve_config(precedence: &ConfigPrecedence, workspace: &Path) -> Option<ResolvedConfig> {
    for location in precedence.locations {
        if let Some(path) = check_location(location, workspace) {
            return Some(ResolvedConfig {
                path: path.to_string_lossy().into_owned(),
                is_default: false,
            });
        }
    }
    write_default_cache(precedence).map(|path| ResolvedConfig {
        path: path.to_string_lossy().into_owned(),
        is_default: true,
    })
}

fn check_location(location: &ConfigLocation, workspace: &Path) -> Option<PathBuf> {
    match location {
        ConfigLocation::Walk { filename, stop_at } => walk_for(filename, workspace, *stop_at),
        ConfigLocation::EnvVar(var) => {
            let val = std::env::var(var).ok().filter(|s| !s.is_empty())?;
            let path = PathBuf::from(val);
            path.is_file().then_some(path)
        }
        ConfigLocation::Xdg(rel) => {
            let path = xdg_config_home()?.join(rel);
            path.is_file().then_some(path)
        }
        ConfigLocation::Home(rel) => {
            let path = home_dir()?.join(rel);
            path.is_file().then_some(path)
        }
    }
}

fn walk_for(filename: &str, start: &Path, stop_at: WalkStop) -> Option<PathBuf> {
    let home = match stop_at {
        WalkStop::Home => home_dir(),
        WalkStop::Root => None,
    };

    let mut dir = start.to_path_buf();
    loop {
        let candidate = dir.join(filename);
        if candidate.is_file() {
            return Some(candidate);
        }
        if matches!(stop_at, WalkStop::Home) && home.as_ref().is_some_and(|h| dir == *h) {
            break;
        }
        match dir.parent() {
            Some(parent) => dir = parent.to_path_buf(),
            None => break,
        }
    }
    None
}

fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
}

fn xdg_config_home() -> Option<PathBuf> {
    std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|h| h.join(".config")))
}

fn write_default_cache(precedence: &ConfigPrecedence) -> Option<PathBuf> {
    let cache_dir = std::env::var("XDG_CACHE_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|h| h.join(".cache")))?
        .join("vlint");

    std::fs::create_dir_all(&cache_dir).ok()?;
    let path = cache_dir.join(precedence.cache_filename);
    std::fs::write(&path, precedence.default).ok()?;
    Some(path)
}
