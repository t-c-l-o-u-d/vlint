// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fs;
use std::path::Path;
use std::sync::Mutex;

use tempfile::TempDir;
use vlint::catalog::linter::{ConfigLocation, ConfigPrecedence, WalkStop};
use vlint::config::resolve::resolve_config;

// Env var mutation is process-global. Tests that set HOME/XDG_* must run serially.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn make_precedence(locations: &'static [ConfigLocation]) -> ConfigPrecedence {
    ConfigPrecedence {
        locations,
        flag: "-c",
        cache_filename: "test.yaml",
        default: "extends: default\n",
    }
}

fn set_home(dir: &Path) {
    // SAFETY: caller must hold ENV_LOCK
    unsafe { std::env::set_var("HOME", dir) };
    unsafe { std::env::remove_var("XDG_CONFIG_HOME") };
}

fn set_cache(dir: &Path) {
    // SAFETY: caller must hold ENV_LOCK
    unsafe { std::env::set_var("XDG_CACHE_HOME", dir) };
}

#[test]
fn walk_finds_config_in_workspace() {
    let _lock = ENV_LOCK.lock().unwrap();
    let workspace = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    set_home(home.path());
    fs::write(workspace.path().join(".testrc"), "content").unwrap();

    static LOCS: &[ConfigLocation] = &[ConfigLocation::Walk {
        filename: ".testrc",
        stop_at: WalkStop::Root,
    }];
    let prec = make_precedence(LOCS);
    let result = resolve_config(&prec, workspace.path()).unwrap();
    assert!(!result.is_default);
    assert!(result.path.ends_with(".testrc"));
}

#[test]
fn walk_stops_at_home() {
    let _lock = ENV_LOCK.lock().unwrap();
    let root = TempDir::new().unwrap();
    let home = root.path().join("home");
    let project = home.join("project");
    fs::create_dir_all(&project).unwrap();
    fs::write(root.path().join(".testrc"), "above home").unwrap();
    set_home(&home);

    let cache = TempDir::new().unwrap();
    set_cache(cache.path());

    static LOCS: &[ConfigLocation] = &[ConfigLocation::Walk {
        filename: ".testrc",
        stop_at: WalkStop::Home,
    }];
    let prec = make_precedence(LOCS);
    let result = resolve_config(&prec, &project).unwrap();
    assert!(result.is_default, "config above $HOME must not be found");
}

#[test]
fn walk_finds_config_at_home_boundary() {
    let _lock = ENV_LOCK.lock().unwrap();
    let root = TempDir::new().unwrap();
    let home = root.path().join("home");
    let project = home.join("project");
    fs::create_dir_all(&project).unwrap();
    fs::write(home.join(".testrc"), "at home").unwrap();
    set_home(&home);

    static LOCS: &[ConfigLocation] = &[ConfigLocation::Walk {
        filename: ".testrc",
        stop_at: WalkStop::Home,
    }];
    let prec = make_precedence(LOCS);
    let result = resolve_config(&prec, &project).unwrap();
    assert!(!result.is_default);
    assert!(result.path.ends_with(".testrc"));
}

#[test]
fn xdg_config_found() {
    let _lock = ENV_LOCK.lock().unwrap();
    let home = TempDir::new().unwrap();
    let xdg_config = TempDir::new().unwrap();
    set_home(home.path());
    unsafe { std::env::set_var("XDG_CONFIG_HOME", xdg_config.path()) };
    let tool_dir = xdg_config.path().join("mytool");
    fs::create_dir_all(&tool_dir).unwrap();
    fs::write(tool_dir.join("config"), "xdg content").unwrap();

    let workspace = TempDir::new().unwrap();
    static LOCS: &[ConfigLocation] = &[ConfigLocation::Xdg("mytool/config")];
    let prec = make_precedence(LOCS);
    let result = resolve_config(&prec, workspace.path()).unwrap();
    assert!(!result.is_default);
    assert!(result.path.ends_with("mytool/config"));
}

#[test]
fn home_location_found() {
    let _lock = ENV_LOCK.lock().unwrap();
    let home = TempDir::new().unwrap();
    set_home(home.path());
    fs::write(home.path().join(".testrc"), "home content").unwrap();

    let workspace = TempDir::new().unwrap();
    let cache = TempDir::new().unwrap();
    set_cache(cache.path());

    static LOCS: &[ConfigLocation] = &[ConfigLocation::Home(".testrc")];
    let prec = make_precedence(LOCS);
    let result = resolve_config(&prec, workspace.path()).unwrap();
    assert!(!result.is_default);
    assert!(result.path.ends_with(".testrc"));
}

#[test]
fn env_var_location_found() {
    let _lock = ENV_LOCK.lock().unwrap();
    let config_file = TempDir::new().unwrap();
    let config_path = config_file.path().join("myconfig.yaml");
    fs::write(&config_path, "env content").unwrap();
    unsafe { std::env::set_var("VLINT_TEST_CFG", config_path.to_str().unwrap()) };

    let workspace = TempDir::new().unwrap();
    let cache = TempDir::new().unwrap();
    set_cache(cache.path());

    static LOCS: &[ConfigLocation] = &[ConfigLocation::EnvVar("VLINT_TEST_CFG")];
    let prec = make_precedence(LOCS);
    let result = resolve_config(&prec, workspace.path()).unwrap();
    assert!(!result.is_default);

    unsafe { std::env::remove_var("VLINT_TEST_CFG") };
}

#[test]
fn fallback_writes_default_to_cache() {
    let _lock = ENV_LOCK.lock().unwrap();
    let cache = TempDir::new().unwrap();
    set_cache(cache.path());
    let workspace = TempDir::new().unwrap();

    static LOCS: &[ConfigLocation] = &[];
    let prec = make_precedence(LOCS);
    let result = resolve_config(&prec, workspace.path()).unwrap();
    assert!(result.is_default);
    let on_disk = fs::read_to_string(&result.path).unwrap();
    assert_eq!(on_disk, "extends: default\n");
}

#[test]
fn cache_respects_xdg_cache_home() {
    let _lock = ENV_LOCK.lock().unwrap();
    let cache = TempDir::new().unwrap();
    set_cache(cache.path());
    let workspace = TempDir::new().unwrap();

    static LOCS: &[ConfigLocation] = &[];
    let prec = make_precedence(LOCS);
    let result = resolve_config(&prec, workspace.path()).unwrap();
    assert!(result.path.starts_with(cache.path().to_str().unwrap()));
}

#[test]
fn first_found_wins() {
    let _lock = ENV_LOCK.lock().unwrap();
    let workspace = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    set_home(home.path());
    fs::write(workspace.path().join("first.yaml"), "first").unwrap();

    static LOCS: &[ConfigLocation] = &[
        ConfigLocation::Walk {
            filename: "first.yaml",
            stop_at: WalkStop::Root,
        },
        ConfigLocation::Walk {
            filename: "second.yaml",
            stop_at: WalkStop::Root,
        },
    ];
    let prec = make_precedence(LOCS);
    let result = resolve_config(&prec, workspace.path()).unwrap();
    assert!(!result.is_default);
    assert!(result.path.ends_with("first.yaml"));
}
