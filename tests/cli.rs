// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

use vlint::cli::{CliArgs, Subcommand};

fn parse(args: &[&str]) -> CliArgs {
    CliArgs::parse_from(args.iter().map(|s| s.to_string()))
}

#[test]
fn defaults_when_empty() {
    let a = parse(&[]);
    assert!(a.subcommand.is_none());
    assert!(!a.verbose);
    assert!(!a.debug);
    assert!(a.config.is_none());
    assert!(a.files.is_empty());
}

#[test]
fn debug_flag() {
    assert!(parse(&["-d"]).debug);
    assert!(parse(&["--debug"]).debug);
    assert!(!parse(&["-v"]).debug, "verbose does not imply debug");
}

#[test]
fn combined_verbose_and_debug() {
    let a = parse(&["-vd"]);
    assert!(a.verbose);
    assert!(a.debug);
}

#[test]
fn short_flags() {
    let a = parse(&["-t", "-v"]);
    assert_eq!(a.subcommand, Some(Subcommand::Tools));
    assert!(a.verbose);
}

#[test]
fn long_flags() {
    let a = parse(&["--tools", "--verbose"]);
    assert_eq!(a.subcommand, Some(Subcommand::Tools));
    assert!(a.verbose);
}

#[test]
fn config_short_separate() {
    let a = parse(&["-c", "/etc/vlint.ini"]);
    assert_eq!(a.config, Some(PathBuf::from("/etc/vlint.ini")));
}

#[test]
fn config_long_separate() {
    let a = parse(&["--config", "/etc/vlint.ini"]);
    assert_eq!(a.config, Some(PathBuf::from("/etc/vlint.ini")));
}

#[test]
fn config_long_equals() {
    let a = parse(&["--config=/etc/vlint.ini"]);
    assert_eq!(a.config, Some(PathBuf::from("/etc/vlint.ini")));
}

#[test]
fn positional_files() {
    let a = parse(&["src/", "README.md"]);
    assert_eq!(
        a.files,
        vec![PathBuf::from("src/"), PathBuf::from("README.md")]
    );
}

#[test]
fn double_dash_separator() {
    let a = parse(&["-v", "--", "--not-a-flag", "file.rs"]);
    assert!(a.verbose);
    assert_eq!(
        a.files,
        vec![PathBuf::from("--not-a-flag"), PathBuf::from("file.rs")]
    );
}

#[test]
fn flags_and_files_mixed() {
    let a = parse(&["-v", "src/", "-t", "--config", "vlint.ini"]);
    assert!(a.verbose);
    assert_eq!(a.subcommand, Some(Subcommand::Tools));
    assert_eq!(a.config, Some(PathBuf::from("vlint.ini")));
    assert_eq!(a.files, vec![PathBuf::from("src/")]);
}
