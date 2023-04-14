use std::env;

use crate::task_harness::TaskHarness;
use crate::{TaskError, TaskResult};

const HELP_TEXT: &str = "Available tasks:
  bloat          scan for potential bloat
  build          build all targets
  build-release  build all targets and scan  
  test           build and test
  test-release   build and test in release mode
  ci             run the ci suite
  prepare        run update, and baseline lints and checks
  scan           scan for vulnerabilities and unused dependencies";

pub struct Cli;

impl Cli {
    pub fn parse_and_run() -> TaskResult<()> {
        let maybe_argument = env::args()
            .nth(1)
            .as_ref()
            .map(|argument| argument.trim().to_lowercase());

        let harness = TaskHarness::new()?;

        match maybe_argument.as_deref() {
            Some("bloat") => harness.task_bloat(),
            Some("build") => harness.task_build(),
            Some("build-release") => harness.task_build_release(),
            Some("test") => harness.task_test(),
            Some("test-release") => harness.task_test_release(),
            Some("ci") => harness.task_ci(),
            Some("prepare") => harness.task_prepare(),
            Some("scan") => harness.task_scan(),
            Some(invalid_task) => Self::invalid(invalid_task),
            None => Self::help(),
        }
    }

    fn invalid(invalid_task: impl Into<String>) -> TaskResult<()> {
        eprintln!("{HELP_TEXT}");
        Err(TaskError::InvalidTaskProvided(invalid_task.into()))
    }

    fn help() -> TaskResult<()> {
        println!("{HELP_TEXT}");
        Ok(())
    }
}
