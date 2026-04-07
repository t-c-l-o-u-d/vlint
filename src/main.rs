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
use crate::runner::Mode;

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

fn filter_backend_chain(chain: &mut Vec<Box<dyn backend::Backend>>, name: &str) {
    let kind = match name {
        "auto" => None,
        "path" => Some(backend::BackendKind::Path),
        "docker" => Some(backend::BackendKind::Docker),
        "podman" => Some(backend::BackendKind::Podman),
        "nspawn" => Some(backend::BackendKind::Nspawn),
        other => {
            eprintln!("vlint: unknown backend: {other}");
            std::process::exit(2);
        }
    };
    if let Some(kind) = kind {
        chain.retain(|b| b.kind() == kind);
    }
}

fn main() -> ExitCode {
    let args = cli::CliArgs::parse();

    let cwd = std::env::current_dir().unwrap_or_else(|e| {
        eprintln!("vlint: cannot determine working directory: {e}");
        std::process::exit(2);
    });

    let (workspace, explicit_files) = if args.files.is_empty() {
        (cwd, None)
    } else if args.files.len() == 1 && args.files[0].is_dir() {
        (args.files[0].clone(), None)
    } else {
        for path in &args.files {
            if !path.exists() {
                eprintln!("error: path does not exist: {}", path.display());
                std::process::exit(2);
            }
        }
        (cwd, Some(args.files))
    };

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
            println!("vlint {}", env!("CARGO_PKG_VERSION"));
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

    println!("Scanning {}...", workspace.display());
    let detection = if let Some(files) = explicit_files {
        detect::detect_explicit(&workspace, &files, args.verbose)
    } else {
        detect::detect_all(&workspace, args.verbose)
    };

    output::print_detection_summary(&detection);

    if detection.file_assignments.values().all(Vec::is_empty) {
        println!("Nothing to lint.");
        return ExitCode::SUCCESS;
    }

    match config.format {
        Some(FormatMode::Check) => {
            println!("Running format (check)...\n");
            let fmt = runner::format::format(
                &detection,
                &chain,
                &tools,
                &workspace,
                Mode::FormatPrint,
                args.verbose,
            );
            ExitCode::from(output::print_results(&fmt))
        }
        Some(FormatMode::Apply) => {
            println!("Running format...\n");
            let fmt = runner::format::format(
                &detection,
                &chain,
                &tools,
                &workspace,
                Mode::Format,
                args.verbose,
            );
            let fmt_code = output::print_results(&fmt);
            println!();
            println!("Running lint...\n");
            let lint = runner::lint::lint(&detection, &chain, &tools, &workspace, args.verbose);
            ExitCode::from(fmt_code.max(output::print_results(&lint)))
        }
        None => {
            println!("Running lint...\n");
            let results = runner::lint::lint(&detection, &chain, &tools, &workspace, args.verbose);
            ExitCode::from(output::print_results(&results))
        }
    }
}
