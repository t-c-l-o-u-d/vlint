// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Mutex;

use tempfile::TempDir;
use vlint::update::{cooldown_elapsed, cooldown_path, is_newer, record_cooldown};

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn newer_patch() {
    assert!(is_newer("1.0.1", "1.0.0"));
}

#[test]
fn newer_minor() {
    assert!(is_newer("1.1.0", "1.0.9"));
}

#[test]
fn newer_major() {
    assert!(is_newer("2.0.0", "1.9.9"));
}

#[test]
fn same_version_not_newer() {
    assert!(!is_newer("1.2.3", "1.2.3"));
}

#[test]
fn older_not_newer() {
    assert!(!is_newer("1.0.0", "1.0.1"));
}

#[test]
fn missing_patch_treated_as_zero() {
    assert!(!is_newer("1.0", "1.0.0"));
    assert!(is_newer("1.1", "1.0.0"));
}

#[test]
fn non_numeric_part_treated_as_zero() {
    assert!(!is_newer("1.0.x", "1.0.0"));
}

// --- cooldown ---

#[test]
fn cooldown_path_uses_xdg_cache_home() {
    let _lock = ENV_LOCK.lock().unwrap();
    let cache = TempDir::new().unwrap();
    unsafe { std::env::set_var("XDG_CACHE_HOME", cache.path()) };
    let path = cooldown_path().unwrap();
    assert!(path.starts_with(cache.path()));
    assert!(path.ends_with("last-update-check"));
}

#[test]
fn cooldown_path_falls_back_to_home() {
    let _lock = ENV_LOCK.lock().unwrap();
    let home = TempDir::new().unwrap();
    unsafe { std::env::remove_var("XDG_CACHE_HOME") };
    unsafe { std::env::set_var("HOME", home.path()) };
    let path = cooldown_path().unwrap();
    assert!(path.starts_with(home.path()));
    assert!(path.ends_with("last-update-check"));
}

#[test]
fn no_cooldown_file_means_elapsed() {
    let _lock = ENV_LOCK.lock().unwrap();
    let cache = TempDir::new().unwrap();
    unsafe { std::env::set_var("XDG_CACHE_HOME", cache.path()) };
    assert!(
        cooldown_elapsed(),
        "missing file should mean cooldown has elapsed"
    );
}

#[test]
fn fresh_cooldown_file_means_not_elapsed() {
    let _lock = ENV_LOCK.lock().unwrap();
    let cache = TempDir::new().unwrap();
    unsafe { std::env::set_var("XDG_CACHE_HOME", cache.path()) };
    record_cooldown();
    assert!(
        !cooldown_elapsed(),
        "freshly written file should not have elapsed"
    );
}
