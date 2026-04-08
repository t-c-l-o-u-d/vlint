// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq)]
pub enum Subcommand {
    Help,
    Version,
    Tools,
}

pub struct CliArgs {
    pub subcommand: Option<Subcommand>,
    pub verbose: bool,
    pub config: Option<PathBuf>,
    pub files: Vec<PathBuf>,
}

impl CliArgs {
    #[must_use]
    pub fn parse() -> Self {
        Self::parse_from(std::env::args().skip(1))
    }

    #[must_use]
    pub fn parse_from(args: impl Iterator<Item = String>) -> Self {
        let mut result = CliArgs {
            subcommand: None,
            verbose: false,
            config: None,
            files: Vec::new(),
        };
        // Expand combined short flags: -vV -> -v -V
        let expanded: Vec<String> = args
            .flat_map(|arg| {
                if !arg.starts_with("--") && arg.starts_with('-') && arg.len() > 2 {
                    arg[1..].chars().map(|c| format!("-{c}")).collect()
                } else {
                    vec![arg]
                }
            })
            .collect();
        let mut args = expanded.into_iter();
        let mut positional_only = false;

        while let Some(arg) = args.next() {
            if positional_only {
                result.files.push(PathBuf::from(arg));
                continue;
            }
            match arg.as_str() {
                "--" => positional_only = true,
                "-t" | "--tools" => result.subcommand = Some(Subcommand::Tools),
                "-v" | "--verbose" => result.verbose = true,
                "-c" | "--config" => {
                    if let Some(val) = args.next() {
                        result.config = Some(PathBuf::from(val));
                    } else {
                        eprintln!("error: flag '{arg}' requires a value");
                        std::process::exit(2);
                    }
                }
                "-h" | "--help" => result.subcommand = Some(Subcommand::Help),
                "-V" | "--version" => result.subcommand = Some(Subcommand::Version),
                _ if arg.starts_with("--config=") => {
                    result.config = Some(PathBuf::from(&arg["--config=".len()..]));
                }
                _ if arg.starts_with('-') => {
                    eprintln!("error: unexpected argument '{arg}'");
                    eprintln!("Try 'vlint --help' for more information.");
                    std::process::exit(2);
                }
                _ => result.files.push(PathBuf::from(arg)),
            }
        }

        result
    }
}

pub fn print_version(verbose: bool) {
    println!("vlint {}", env!("CARGO_PKG_VERSION"));
    if verbose {
        if let Some(date) = option_env!("BUILD_DATE") {
            println!("Build Date: {date}");
        }
        println!("Source: {}", env!("CARGO_PKG_REPOSITORY"));
    }
}

pub fn print_help() {
    println!(
        "Lint and format code using native tools or containers

Usage: vlint [OPTIONS] [FILE|DIR]...

Arguments:
  [FILE|DIR]...  Files or directories to lint (default: current directory)

Options:
  -t, --tools          List all tools and how each resolves
  -v, --verbose        Verbose output: detection scoring details and full tool output
  -c, --config <FILE>  Path to a vlint config file (default: $XDG_CONFIG_HOME/vlint/config.ini)
  -h, --help           Print help
  -V, --version        Print version"
    );
}
