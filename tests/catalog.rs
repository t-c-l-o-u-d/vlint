// SPDX-License-Identifier: AGPL-3.0-or-later

use vlint::catalog::build_owned_catalog;
use vlint::catalog::linter::ConfigLocation;

#[test]
fn cargo_deny_has_config_precedence() {
    let tools = build_owned_catalog();
    let deny = tools
        .iter()
        .find(|t| t.name == "cargo-deny")
        .expect("cargo-deny not in catalog");
    let prec = deny
        .config_precedence
        .as_ref()
        .expect("cargo-deny has no config_precedence");
    assert_eq!(prec.flag, "--config");
    assert!(!prec.default.is_empty());
}

#[test]
fn cargo_deny_precedence_includes_deny_toml() {
    let tools = build_owned_catalog();
    let deny = tools.iter().find(|t| t.name == "cargo-deny").unwrap();
    let prec = deny.config_precedence.as_ref().unwrap();
    let has_deny_toml = prec.locations.iter().any(|loc| {
        matches!(
            loc,
            ConfigLocation::Walk { filename, .. } if *filename == "deny.toml"
        )
    });
    assert!(
        has_deny_toml,
        "no deny.toml Walk entry in cargo-deny config_precedence"
    );
}

#[test]
fn all_tools_with_config_precedence_have_nonempty_default() {
    let tools = build_owned_catalog();
    for tool in &tools {
        if let Some(prec) = &tool.config_precedence {
            assert!(
                !prec.default.is_empty(),
                "{}: config_precedence.default is empty",
                tool.name
            );
        }
    }
}

#[test]
fn all_tools_with_config_precedence_have_flag() {
    let tools = build_owned_catalog();
    for tool in &tools {
        if let Some(prec) = &tool.config_precedence {
            assert!(
                !prec.flag.is_empty(),
                "{}: config_precedence.flag is empty",
                tool.name
            );
        }
    }
}

#[test]
fn all_tools_with_config_precedence_have_cache_filename() {
    let tools = build_owned_catalog();
    for tool in &tools {
        if let Some(prec) = &tool.config_precedence {
            assert!(
                !prec.cache_filename.is_empty(),
                "{}: config_precedence.cache_filename is empty",
                tool.name
            );
        }
    }
}
