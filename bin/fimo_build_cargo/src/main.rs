use clap::Parser;
use std::{path::PathBuf, process::ExitCode};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// Path to the module root
    #[clap(long)]
    module_dir: PathBuf,
    /// Path to the target directory
    #[clap(long)]
    target_dir: PathBuf,
    /// Path to the output directory
    #[clap(long)]
    out_dir: PathBuf,
    /// Architecture on which the command operates on
    #[clap(long)]
    target: Option<String>,
    /// Switches to the release mode
    #[clap(long)]
    release: bool,
    /// Uses verbose output
    #[clap(short, long)]
    verbose: bool,
}

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().collect();
    let args_delimiter_pos = args.iter().position(|arg| *arg == "--");
    let (cli_args, cargo_args) = if let Some(args_delimiter_pos) = args_delimiter_pos {
        (
            &args[..args_delimiter_pos],
            Some(&args[args_delimiter_pos + 1..]),
        )
    } else {
        (args.as_ref(), None)
    };

    let cli = Cli::parse_from(cli_args);

    let manifest_path = cli.module_dir.join("Cargo.toml");

    let mut command = std::process::Command::new("cargo");
    command
        .arg("build")
        .args(["-Z", "unstable-options"])
        .arg("--manifest-path")
        .arg(manifest_path)
        .arg("--target-dir")
        .arg(cli.target_dir)
        .arg("--out-dir")
        .arg(cli.out_dir);

    if let Some(target) = cli.target {
        command.arg("--target").arg(target);
    }

    if cli.release {
        command.arg("--release");
    }

    if cli.verbose {
        command.arg("--verbose");
    }

    if let Some(args) = cargo_args {
        command.args(args);
    }

    if command
        .status()
        .expect("could not execute process")
        .success()
    {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
