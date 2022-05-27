use clap::{Parser, Subcommand};
use std::{
    error::Error,
    ffi::OsString,
    path::{Path, PathBuf},
    process::ExitCode,
};

mod manifest;

const MANIFEST_NAME: &str = "Fimo.toml";

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[clap(about = "Builds the workspace")]
    Build {
        #[clap(flatten)]
        common: CommonArgs,
    },
    #[clap(about = "Runs the test contained in the workspace")]
    Test {
        #[clap(flatten)]
        common: CommonArgs,
    },
    #[clap(about = "Cleans up build artifacts")]
    Clean {
        #[clap(flatten)]
        common: CommonArgs,
    },
}

#[derive(Parser, Debug, Clone)]
struct CommonArgs {
    /// Path to the manifest file
    #[clap(long)]
    manifest_path: Option<PathBuf>,
    /// Path to the target directory
    #[clap(long)]
    target_dir: Option<PathBuf>,
    /// Path to the output directory
    #[clap(long)]
    out_dir: Option<PathBuf>,
    /// Architecture on which the command operates on
    #[clap(long)]
    target: Option<String>,
    /// Profile filter for the modules in the workspace
    #[clap(long)]
    profile: Option<String>,
    /// Switches to the release mode
    #[clap(long)]
    release: bool,
    /// Uses verbose output
    #[clap(short, long)]
    verbose: bool,
}

#[derive(Debug)]
struct CommonPaths {
    manifest_path: PathBuf,
    manifest_dir: PathBuf,
    target_dir: PathBuf,
    build_dir: PathBuf,
    out_dir: PathBuf,
}

impl TryFrom<&CommonArgs> for CommonPaths {
    type Error = Box<dyn Error>;

    fn try_from(args: &CommonArgs) -> Result<Self, Self::Error> {
        let manifest_path = args
            .manifest_path
            .clone()
            .unwrap_or(std::env::current_dir()?.join(MANIFEST_NAME));
        let manifest_dir = manifest_path.parent().unwrap().to_path_buf();

        let target_dir = args
            .target_dir
            .clone()
            .unwrap_or_else(|| manifest_dir.join("fimo_target"));
        let build_dir = target_dir.join("build");

        let mut out_dir = args
            .out_dir
            .clone()
            .unwrap_or_else(|| target_dir.join("modules"));
        out_dir = if args.release {
            out_dir.join("release")
        } else {
            out_dir.join("debug")
        };

        Ok(CommonPaths {
            manifest_path,
            manifest_dir,
            target_dir,
            build_dir,
            out_dir,
        })
    }
}

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().collect();
    let args_delimiter_pos = args.iter().position(|arg| *arg == "--");
    let (cli_args, extra_args) = if let Some(args_delimiter_pos) = args_delimiter_pos {
        (
            &args[..args_delimiter_pos],
            Some(&args[args_delimiter_pos + 1..]),
        )
    } else {
        (args.as_ref(), None)
    };

    let cli = Cli::parse_from(cli_args);

    let bin_dir = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    let result = match cli.command {
        Commands::Build { common } => build(common, &bin_dir, extra_args),
        Commands::Test { common } => test(common, &bin_dir, extra_args),
        Commands::Clean { common } => clean(common),
    };

    match result {
        Ok(e) => e,
        Err(err) => {
            eprintln!("{}", err);
            ExitCode::FAILURE
        }
    }
}

fn build(
    args: CommonArgs,
    bin_dir: &Path,
    extra_args: Option<&[OsString]>,
) -> Result<ExitCode, Box<dyn Error>> {
    let paths: CommonPaths = (&args).try_into()?;
    let workspace = manifest::load_fimo_manifest(&paths.manifest_path)?;

    let mut env = Vec::new();
    env.push(("FIMO_WORKSPACE", workspace.name.clone()));
    env.push((
        "FIMO_WORKSPACE_DIR",
        format!("{}", paths.manifest_dir.display()),
    ));
    env.push(("FIMO_TARGET_DIR", format!("{}", paths.target_dir.display())));
    env.push(("FIMO_BUILD_DIR", format!("{}", paths.build_dir.display())));
    env.push(("FIMO_OUT_DIR", format!("{}", paths.out_dir.display())));

    for (module_path, module) in workspace.modules {
        let profile = module.profile.iter().find(|p| {
            if let Some(profile) = &args.profile {
                p.name == *profile
            } else {
                p.default
            }
        });

        let profile = match profile {
            Some(profile) => profile,
            None => continue,
        };
        let builder_path = bin_dir.join(format!("fimo-build-{}", profile.builder.name));

        let build_dir = paths.build_dir.join(&module.module.name);
        let out_dir = paths.out_dir.join(&module.module.name);

        let mut module_env = Vec::new();
        module_env.push(("FIMO_MODULE_NAME", module.module.name));
        module_env.push(("FIMO_MODULE_VERSION", module.module.version));
        module_env.push(("FIMO_MODULE_DIR", format!("{}", module_path.display())));
        module_env.push(("FIMO_MODULE_BUILD_DIR", format!("{}", build_dir.display())));
        module_env.push(("FIMO_MODULE_OUT_DIR", format!("{}", out_dir.display())));

        let mut command = std::process::Command::new(builder_path);
        command
            .arg("--module-dir")
            .arg(module_path)
            .arg("--target-dir")
            .arg(build_dir)
            .arg("--out-dir")
            .arg(&out_dir);

        for (k, v) in &env {
            command.env(k, v);
        }

        for (k, v) in module_env {
            command.env(k, v);
        }

        if let Some(target) = &args.target {
            command.arg("--target").arg(target);
        }

        if args.release {
            command.arg("--release");
        }

        if args.verbose {
            command.arg("--verbose");
        }

        if !profile.builder.args.is_empty() {
            command.arg("--").args(&profile.builder.args);
        }

        if let Some(extra_args) = extra_args {
            command.arg("--").args(extra_args);
        }

        if !command.status()?.success() {
            return Ok(ExitCode::FAILURE);
        }

        for exclude in &profile.exclude {
            let path = out_dir.join(exclude);
            if path.exists() {
                if path.is_file() {
                    std::fs::remove_file(path)?
                } else if path.is_dir() {
                    std::fs::remove_dir_all(path)?
                }
            }
        }
    }

    Ok(ExitCode::SUCCESS)
}

fn test(
    args: CommonArgs,
    bin_dir: &Path,
    extra_args: Option<&[OsString]>,
) -> Result<ExitCode, Box<dyn Error>> {
    build(args.clone(), bin_dir, None)?;

    let paths: CommonPaths = (&args).try_into()?;
    let workspace = manifest::load_fimo_manifest(&paths.manifest_path)?;

    let mut env = Vec::new();
    env.push(("FIMO_WORKSPACE", workspace.name.clone()));
    env.push((
        "FIMO_WORKSPACE_DIR",
        format!("{}", paths.manifest_dir.display()),
    ));
    env.push(("FIMO_TARGET_DIR", format!("{}", paths.target_dir.display())));
    env.push(("FIMO_BUILD_DIR", format!("{}", paths.build_dir.display())));
    env.push(("FIMO_OUT_DIR", format!("{}", paths.out_dir.display())));

    for (module_path, module) in workspace.modules {
        let profile = module.profile.iter().find(|p| {
            if let Some(profile) = &args.profile {
                p.name == *profile
            } else {
                p.default
            }
        });

        let profile = match profile {
            Some(profile) => profile,
            None => continue,
        };
        let runner_path = bin_dir.join(format!("fimo-test-{}", profile.builder.name));

        let build_dir = paths.build_dir.join(&module.module.name);
        let out_dir = paths.out_dir.join(&module.module.name);

        let mut module_env = Vec::new();
        module_env.push(("FIMO_MODULE_NAME", module.module.name));
        module_env.push(("FIMO_MODULE_VERSION", module.module.version));
        module_env.push(("FIMO_MODULE_DIR", format!("{}", module_path.display())));
        module_env.push(("FIMO_MODULE_BUILD_DIR", format!("{}", build_dir.display())));
        module_env.push(("FIMO_MODULE_OUT_DIR", format!("{}", out_dir.display())));

        let mut command = std::process::Command::new(runner_path);
        command
            .arg("--module-dir")
            .arg(module_path)
            .arg("--target-dir")
            .arg(build_dir);

        for (k, v) in &env {
            command.env(k, v);
        }

        for (k, v) in module_env {
            command.env(k, v);
        }

        if let Some(target) = &args.target {
            command.arg("--target").arg(target);
        }

        if args.release {
            command.arg("--release");
        }

        if args.verbose {
            command.arg("--verbose");
        }

        if !profile.builder.args.is_empty() {
            command.arg("--").args(&profile.builder.args);
        }

        if let Some(extra_args) = extra_args {
            command.arg("--").args(extra_args);
        }

        if !command.status()?.success() {
            return Ok(ExitCode::FAILURE);
        }
    }

    if let Some(test) = workspace.test {
        let build_dir = paths.build_dir.join("__include");
        for path in test.include {
            let runner_path = bin_dir.join(format!("fimo-test-{}", test.runner));

            let mut command = std::process::Command::new(runner_path);
            command
                .arg("--module-dir")
                .arg(path)
                .arg("--target-dir")
                .arg(&build_dir);

            for (k, v) in &env {
                command.env(k, v);
            }

            if let Some(target) = &args.target {
                command.arg("--target").arg(target);
            }

            if args.release {
                command.arg("--release");
            }

            if args.verbose {
                command.arg("--verbose");
            }

            if let Some(extra_args) = extra_args {
                command.arg("--").args(extra_args);
            }

            if !command.status()?.success() {
                return Ok(ExitCode::FAILURE);
            }
        }
    }

    Ok(ExitCode::SUCCESS)
}

fn clean(args: CommonArgs) -> Result<ExitCode, Box<dyn Error>> {
    let paths: CommonPaths = (&args).try_into()?;

    if paths.build_dir.exists() {
        std::fs::remove_dir_all(&paths.build_dir)?;
    }
    if paths.out_dir.exists() {
        std::fs::remove_dir_all(&paths.out_dir)?;
    }
    if paths.target_dir.exists() {
        std::fs::remove_dir_all(&paths.target_dir)?;
    }

    Ok(ExitCode::SUCCESS)
}
