use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{TaskError, TaskResult};

const CARGO_MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");
const PRINT_PREFIX: &str = "[xtask]";
const MODULES_PATHS_ENV: &str = "FIMO_MODULES_PATHS";
const MODULES: &[&str] = &["fimo_actix", "fimo_core", "fimo_logging", "fimo_tasks"];

pub struct TaskHarness {
    root: PathBuf,
}

impl TaskHarness {
    pub fn new() -> TaskResult<Self> {
        let root = match Path::new(CARGO_MANIFEST_DIR).ancestors().nth(1) {
            Some(found_root) => found_root.to_path_buf(),
            None => return Err(TaskError::CouldNotDetermineRepositoryRoot),
        };
        Ok(Self { root })
    }

    fn cargo(&self, args: &'static str) -> TaskResult<()> {
        self.stdout(format!("running: cargo {args}"));

        let mut cmd = Command::new("cargo");
        match cmd
            .current_dir(&self.root)
            .args(args.trim().split(' '))
            .status()?
            .success()
        {
            true => Ok(()),
            false => Err(TaskError::CargoCommandFailed),
        }
    }

    fn cargo_with_envs(&self, args: &'static str, envs: HashMap<String, String>) -> TaskResult<()> {
        self.stdout(format!("running: cargo {args}, env {envs:?}"));

        let mut cmd = Command::new("cargo");
        match cmd
            .current_dir(&self.root)
            .args(args.trim().split(' '))
            .envs(envs)
            .status()?
            .success()
        {
            true => Ok(()),
            false => Err(TaskError::CargoCommandFailed),
        }
    }

    pub fn task_bloat(&self) -> TaskResult<()> {
        self.cargo("bloat --release")?;
        self.cargo("bloat --release --crates")
    }

    pub fn task_build(&self) -> TaskResult<()> {
        self.task_prepare()?;
        self.cargo("build --all-features --all-targets --workspace --exclude xtask")?;
        self.create_modules("debug")?;
        Ok(())
    }

    pub fn task_build_release(&self) -> TaskResult<()> {
        self.task_prepare()?;
        self.task_scan()?;
        self.cargo("build --all-features --release --workspace --exclude xtask")?;
        self.create_modules("release")?;
        Ok(())
    }

    pub fn task_ci(&self) -> TaskResult<()> {
        self.cargo("fmt --all -- --check")?;
        self.cargo("check --all-targets --all-features")?;
        self.cargo("clippy --all-targets --all-features --no-deps -- -D warnings")?;
        self.task_test()
    }

    pub fn task_test(&self) -> TaskResult<()> {
        self.cargo("build --all-features --workspace --exclude xtask")?;

        let modules = self.create_modules("debug")?;
        let envs = HashMap::from([(MODULES_PATHS_ENV.into(), modules)]);
        self.cargo_with_envs("nextest run", envs.clone())?;
        self.cargo_with_envs("test --doc -- --nocapture", envs)?;
        Ok(())
    }

    pub fn task_test_release(&self) -> TaskResult<()> {
        self.cargo("build --all-features --release --workspace --exclude xtask")?;

        let modules = self.create_modules("release")?;
        let envs = HashMap::from([(MODULES_PATHS_ENV.into(), modules)]);
        self.cargo_with_envs("nextest run --release", envs.clone())?;
        self.cargo_with_envs("test --doc --release -- --nocapture", envs)?;
        Ok(())
    }

    pub fn task_prepare(&self) -> TaskResult<()> {
        self.cargo("update")?;
        self.cargo("fmt")?;
        self.cargo("fix --edition-idioms --allow-dirty --allow-staged")?;
        self.cargo("clippy --all-features --all-targets --no-deps")
    }

    pub fn task_scan(&self) -> TaskResult<()> {
        self.cargo("+nightly udeps")?;
        self.cargo("audit")
    }

    fn create_modules(&self, opt: &str) -> TaskResult<String> {
        let module_bin_dir = self.root.join("target").join(opt);
        let modules_path = self.root.join("target").join(opt).join("modules");
        if modules_path.exists() {
            std::fs::remove_dir_all(&modules_path)?;
        }
        std::fs::create_dir_all(&modules_path)?;

        let mut modules_map = HashMap::<String, _>::new();
        for &module in MODULES {
            self.stdout(format!("creating: {module} module"));

            #[cfg(target_os = "windows")]
            let module_bin = format!("{module}.dll");
            #[cfg(target_os = "linux")]
            let module_bin = format!("lib{module}.so");
            #[cfg(target_os = "macos")]
            let module_bin = format!("lib{module}.dylib");

            let module_src_path = module_bin_dir.join(&module_bin);
            let module_dst_dir = modules_path.join(module);
            std::fs::create_dir_all(&module_dst_dir)?;
            std::fs::copy(module_src_path, module_dst_dir.join(&module_bin))?;
            std::fs::write(
                module_dst_dir.join("module.json"),
                format!("{{ \n\t\"schema\": \"0\", \n\t\"library_path\": {module_bin:?} \n}}"),
            )?;

            let module = module.strip_prefix("fimo_").unwrap();
            modules_map.insert(module.into(), module_dst_dir);
        }

        Ok(serde_json::to_string_pretty(&modules_map).unwrap())
    }

    pub fn stdout(&self, contents: impl AsRef<str>) {
        let contents = contents.as_ref();
        println!("{PRINT_PREFIX} {contents}")
    }

    #[allow(dead_code)]
    pub fn stderr(&self, contents: impl AsRef<str>) {
        let contents = contents.as_ref();
        eprintln!("{PRINT_PREFIX} {contents}")
    }
}
