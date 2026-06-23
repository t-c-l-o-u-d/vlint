// SPDX-License-Identifier: AGPL-3.0-or-later

mod backend;
mod catalog;
mod cli;
mod color;
mod config;
mod detect;
mod man;
mod output;
mod runner;
mod update;

use std::process::ExitCode;

use crate::config::FormatMode;
use crate::runner::{Invocation, Mode};

fn init_color(config: &config::Config) {
    let mode = config.color.as_ref().unwrap_or(&color::ColorMode::Auto);
    color::init(
        mode,
        config.pass_color.as_deref().unwrap_or("green"),
        config.fail_color.as_deref().unwrap_or("red"),
        config.skip_color.as_deref().unwrap_or("yellow"),
        config.tool_color.as_deref().unwrap_or("blue"),
    );
}

fn resolve_workspace(
    cwd: std::path::PathBuf,
    files: Vec<std::path::PathBuf>,
) -> (std::path::PathBuf, Option<Vec<std::path::PathBuf>>) {
    if files.is_empty() {
        (cwd, None)
    } else if files.len() == 1 && files[0].is_dir() {
        (
            std::fs::canonicalize(&files[0]).unwrap_or_else(|e| {
                eprintln!("error: cannot resolve path {}: {e}", files[0].display());
                std::process::exit(2);
            }),
            None,
        )
    } else {
        for path in &files {
            if !path.exists() {
                eprintln!("error: path does not exist: {}", path.display());
                std::process::exit(2);
            }
        }
        (cwd, Some(files))
    }
}

fn stdout_is_piped() -> bool {
    use std::io::IsTerminal;
    !std::io::stdout().is_terminal()
}

fn stderr_merged_with_stdout() -> bool {
    use std::io::IsTerminal;
    use std::os::fd::AsRawFd;
    if std::io::stderr().is_terminal() {
        return false;
    }
    let out_fd = std::io::stdout().as_raw_fd();
    let err_fd = std::io::stderr().as_raw_fd();
    // SAFETY: fstat on valid std descriptors; return code checked before read.
    unsafe {
        let mut a: libc::stat = std::mem::zeroed();
        let mut b: libc::stat = std::mem::zeroed();
        if libc::fstat(out_fd, &raw mut a) != 0 || libc::fstat(err_fd, &raw mut b) != 0 {
            return false;
        }
        a.st_dev == b.st_dev && a.st_ino == b.st_ino
    }
}

fn print_io_warnings() {
    if stdout_is_piped() {
        eprintln!(
            "WARNING: stdout is being piped or redirected. vlint is not designed to be piped."
        );
    }
    if stderr_merged_with_stdout() {
        eprintln!("WARNING: stderr is merged with stdout (e.g. `2>&1`). Run vlint without it.");
    }
}

fn filter_backend_chain(chain: &mut Vec<Box<dyn backend::Backend>>, name: &str) {
    let kind = match name {
        "auto" => None,
        "path" => Some(backend::BackendKind::Path),
        "docker" => Some(backend::BackendKind::Docker),
        "podman" => Some(backend::BackendKind::Podman),
        other => {
            eprintln!("vlint: unknown backend: {other}");
            std::process::exit(2);
        }
    };
    if let Some(kind) = kind {
        chain.retain(|b| b.kind() == kind);
    }
}

/// Print one pass's results: the failures, optionally the skipped linters (`--debug`), and the
/// failure hint. Returns the exit code from `print_results`.
fn report(out: &runner::LintOutput, verbose: bool, debug: bool) -> u8 {
    let code = output::print_results(out, verbose);
    if debug {
        print!("{}", output::render_skipped(out));
    }
    print!("{}", output::render_failure_hint(out, verbose));
    code
}

/// Loud, unconditional notice printed on every invocation: vlint is dead.
fn print_deprecation_notice() {
    eprintln!("============================================================");
    eprintln!("  WARNING: vlint is NO LONGER MAINTAINED.");
    eprintln!("  Final release: v0.1.2 (2026-06-23).");
    eprintln!("  Do not depend on it. Migrate to another tool immediately.");
    eprintln!("============================================================");
}

fn main() -> ExitCode {
    // Exit quietly on SIGPIPE (e.g. `vlint | head`) instead of panicking.
    #[cfg(unix)]
    // SAFETY: restoring the default disposition for SIGPIPE is a standard CLI idiom.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    print_deprecation_notice();

    let args = cli::CliArgs::parse();

    let cwd = std::env::current_dir().unwrap_or_else(|e| {
        eprintln!("vlint: cannot determine working directory: {e}");
        std::process::exit(2);
    });

    let (workspace, explicit_files) = resolve_workspace(cwd, args.files);

    let config = config::load_config(&workspace, args.config.as_deref());

    if config.man_page_install != Some(false) {
        let _ = man::install();
    }

    match args.subcommand {
        Some(cli::Subcommand::Help) => {
            cli::print_help();
            return ExitCode::SUCCESS;
        }
        Some(cli::Subcommand::Version) => {
            cli::print_version(args.verbose);
            return ExitCode::SUCCESS;
        }
        _ => {}
    }

    if config.auto_update != Some(false) {
        update::try_auto_update(args.verbose);
    }

    init_color(&config);

    // Build tool catalog: hardcoded defaults + config overrides
    let mut tools = catalog::build_owned_catalog();
    config::apply_tool_overrides(&mut tools, &config.tool_overrides);

    let registry = config.registry.as_deref();
    let image_prefix = config.image_prefix.as_deref().unwrap_or("");
    let tag = config.tag.as_deref().unwrap_or("latest");
    let mut chain = backend::discovery_chain(registry, image_prefix, tag);
    if let Some(name) = &config.backend {
        filter_backend_chain(&mut chain, name);
    }

    if args.subcommand == Some(cli::Subcommand::Tools) {
        output::print_tool_list(&tools, &chain, args.verbose);
        return ExitCode::SUCCESS;
    }

    let invocation = match &explicit_files {
        Some(files) if files.len() == 1 && files[0].is_file() => Invocation::SingleFile,
        _ => Invocation::Directory,
    };

    print_io_warnings();

    if args.debug {
        println!("Scanning {}...", workspace.display());
    }
    let detection = if let Some(files) = explicit_files {
        detect::detect_explicit(&workspace, &files, args.debug)
    } else {
        detect::detect_all(&workspace, args.debug)
    };

    if detection.file_assignments.values().all(Vec::is_empty) {
        println!("Nothing to lint.");
        print_io_warnings();
        return ExitCode::SUCCESS;
    }

    let exit_code = match config.format {
        Some(FormatMode::Check) => {
            let fmt = runner::format::format(
                &detection,
                &chain,
                &tools,
                &workspace,
                Mode::FormatPrint,
                invocation,
            );
            ExitCode::from(report(&fmt, args.verbose, args.debug))
        }
        Some(FormatMode::Apply) => {
            // Formatting is opt-in: apply it silently. vlint never reports what it reformatted,
            // and reformatting never affects the exit code -- only the lint pass is reported.
            let _ = runner::format::format(
                &detection,
                &chain,
                &tools,
                &workspace,
                Mode::Format,
                invocation,
            );
            let lint = runner::lint::lint(&detection, &chain, &tools, &workspace, invocation);
            ExitCode::from(report(&lint, args.verbose, args.debug))
        }
        None => {
            let results = runner::lint::lint(&detection, &chain, &tools, &workspace, invocation);
            ExitCode::from(report(&results, args.verbose, args.debug))
        }
    };

    print_io_warnings();
    exit_code
}
