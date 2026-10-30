//! The native macOS GitHub Actions runner.

mod github;
mod install;
pub mod process;
mod settings;
mod shutdown;
mod vm;

use crate::github::GitHub;
use crate::process::{Operations, RealOperations, Spec, require_success};
use crate::settings::Settings;
use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

/// The command's four modes.
#[derive(Subcommand)]
enum Mode {
    /// Build or refresh the image used for one-job VMs.
    Prepare,
    /// Poll GitHub and serve jobs under the Cargo lock.
    Serve,
    /// Run the queued batch, called by the lock command.
    #[command(hide = true)]
    Batch,
    /// Copy the binary and load the user's `LaunchAgent`.
    Install,
}

#[derive(Parser)]
#[command(
    name = "baylee-macos-runner",
    about = "Disposable macOS VMs for GitHub Actions"
)]
struct Cli {
    #[command(subcommand)]
    mode: Mode,
}

/// Parses the native command and runs it with the production process boundary.
///
/// # Errors
/// Returns an error when the requested operation cannot be completed.
pub fn main_entry() -> Result<()> {
    let cli = Cli::parse();
    shutdown::install()?;
    let settings = Settings::from_env();
    let mut operations = RealOperations;
    match cli.mode {
        Mode::Prepare => vm::prepare(&settings, &mut operations),
        Mode::Serve => serve(&settings, &mut operations),
        Mode::Batch => batch(&settings, &mut operations),
        Mode::Install => {
            let home = home()?;
            let current = std::env::current_exe().context("finding the current executable")?;
            install::install_agent(&mut operations, &home, &current)
        }
    }
}

fn home() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .filter(|home| !home.is_empty())
        .map(PathBuf::from)
        .context("HOME is not set")
}

fn serve(settings: &Settings, operations: &mut dyn Operations) -> Result<()> {
    let output = operations.output(
        Spec::new(
            "security",
            [
                "find-generic-password",
                "-s",
                &settings.keychain_service,
                "-w",
            ],
        )
        .with_timeout(Duration::from_secs(10)),
    )?;
    require_success(&output, "security find-generic-password")?;
    let token = String::from_utf8(output.stdout)
        .context("the keychain token is not UTF-8")?
        .trim()
        .to_owned();
    if token.is_empty() {
        bail!(
            "no token in the keychain under {}",
            settings.keychain_service
        );
    }
    crate::log(format!(
        "serving {} as {} from {}",
        settings.repo, settings.label, settings.image
    ));
    vm::sweep(settings, operations, &token);
    loop {
        if operations.cancelled() {
            return Ok(());
        }
        let queued = GitHub::new(operations, &settings.repo, &token).queued(&settings.label);
        if queued > 0 {
            crate::log("a job is queued; taking the cargo lock");
            locked_batch(settings, operations)?;
        }
        operations.sleep(settings.poll);
    }
}

fn locked_batch(settings: &Settings, operations: &mut dyn Operations) -> Result<()> {
    let current = std::env::current_exe().context("finding the current executable")?;
    let current = current.to_string_lossy().to_string();
    let lock = settings.lock.to_string_lossy().to_string();
    let spec = Spec::new(&lock, [current, "batch".to_owned()])
        .with_env("BAYLEE_SESSION", "macos-runner (CI jobs)")
        .with_prefix("batch:".to_owned())
        // A batch may contain more than one three-hour VM.
        .with_timeout(Duration::from_hours(24) + Duration::from_secs(180))
        .for_cleanup();
    let mut process = operations.spawn(spec)?;
    let started = Instant::now();
    let status = loop {
        if operations.cancelled() || started.elapsed() >= Duration::from_hours(24) {
            // The lock wrapper and batch share a group. Let the batch remove its
            // VM and registration before the wrapper releases the Cargo lock.
            process.terminate()?;
            break process.wait(Duration::from_secs(180))?;
        }
        if let Some(status) = process.try_wait()? {
            break status;
        }
        operations.sleep(Duration::from_millis(100));
    };
    if !status.success() {
        bail!("Cargo-locked batch failed with {status}");
    }
    Ok(())
}

fn batch(settings: &Settings, operations: &mut dyn Operations) -> Result<()> {
    let output = operations.output(
        Spec::new(
            "security",
            [
                "find-generic-password",
                "-s",
                &settings.keychain_service,
                "-w",
            ],
        )
        .with_timeout(Duration::from_secs(10)),
    )?;
    require_success(&output, "security find-generic-password")?;
    let token = String::from_utf8(output.stdout)
        .context("the keychain token is not UTF-8")?
        .trim()
        .to_owned();
    if token.is_empty() {
        bail!(
            "no token in the keychain under {}",
            settings.keychain_service
        );
    }
    let log_dir = home()?.join("Library/Logs");
    let mut failed_rounds = 0;
    loop {
        if operations.cancelled() {
            return Ok(());
        }
        let queued = GitHub::new(operations, &settings.repo, &token).queued(&settings.label);
        if queued == 0 {
            return Ok(());
        }
        let jobs = queued.min(settings.max_vms);
        crate::log(format!("{jobs} job(s) queued"));
        let names: Vec<_> = (1..=jobs)
            .map(|index| vm::job_name(settings, index))
            .collect();
        let served = vm::run_jobs(settings, operations, &names, &token, &log_dir);
        if served {
            failed_rounds = 0;
        } else {
            failed_rounds += 1;
            if failed_rounds >= 3 {
                crate::log("three rounds without a job; giving the lock back");
                return Ok(());
            }
        }
        operations.sleep(Duration::from_secs(10));
    }
}

fn timestamp(now: SystemTime) -> String {
    let seconds = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let days = seconds.div_euclid(86_400);
    let day_seconds = seconds.rem_euclid(86_400);
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    };
    if month <= 2 {
        year += 1;
    }
    let hour = day_seconds / 3600;
    let minute = day_seconds % 3600 / 60;
    let second = day_seconds % 60;
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02} UTC")
}

pub(crate) fn log(message: impl AsRef<str>) {
    println!("{} {}", timestamp(SystemTime::now()), message.as_ref());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::test_support::{MockOperations, output};
    use std::path::Path;

    #[test]
    fn timestamps_are_utc_and_calendar_correct() {
        assert_eq!(timestamp(SystemTime::UNIX_EPOCH), "1970-01-01 00:00:00 UTC");
        assert_eq!(
            timestamp(SystemTime::UNIX_EPOCH + Duration::from_secs(951_827_696)),
            "2000-02-29 12:34:56 UTC"
        );
    }

    #[test]
    fn failed_keychain_command_cannot_supply_a_batch_token() {
        let settings = Settings::default();
        let mut operations = MockOperations::default();
        operations.push_output(
            Spec::new(
                "security",
                [
                    "find-generic-password",
                    "-s",
                    &settings.keychain_service,
                    "-w",
                ],
            )
            .with_timeout(Duration::from_secs(10)),
            output(1, "not-a-token"),
        );
        assert!(batch(&settings, &mut operations).is_err());
        operations.assert_done();
    }

    #[test]
    fn an_empty_batch_returns_immediately() {
        let settings = Settings::default();
        let mut operations = MockOperations::default();
        operations.push_output(
            Spec::new(
                "security",
                [
                    "find-generic-password",
                    "-s",
                    &settings.keychain_service,
                    "-w",
                ],
            )
            .with_timeout(Duration::from_secs(10)),
            output(0, "secret-token\n"),
        );
        for status in ["in_progress", "queued"] {
            operations.push_output(
                Spec::new(
                    "gh",
                    [
                        "api",
                        &format!(
                            "repos/{}/actions/runs?status={status}&per_page=20",
                            settings.repo
                        ),
                    ],
                )
                .with_env("GH_TOKEN", "secret-token")
                .with_timeout(Duration::from_secs(30)),
                output(0, r#"{"workflow_runs":[]}"#),
            );
        }
        batch(&settings, &mut operations).unwrap();
        operations.assert_done();
    }

    #[test]
    fn locked_batch_streams_output_and_requires_success() {
        let settings = Settings::default();
        let current = std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let lock = settings.lock.to_string_lossy().to_string();
        let spec = Spec::new(&lock, [current, "batch".to_owned()])
            .with_env("BAYLEE_SESSION", "macos-runner (CI jobs)")
            .with_prefix("batch:".to_owned())
            .with_timeout(Duration::from_hours(24) + Duration::from_secs(180))
            .for_cleanup();
        for code in [0, 1] {
            let mut operations = MockOperations::default();
            operations.push_process(
                spec.clone(),
                Box::new(crate::process::test_support::MockProcess::new(
                    crate::process::test_support::status(code),
                )),
            );
            let result = locked_batch(&settings, &mut operations);
            assert_eq!(result.is_ok(), code == 0);
            operations.assert_done();
        }
    }

    #[test]
    fn no_python_or_shell_scripts_remain_outside_the_exclusions() {
        fn visit(root: &Path, directory: &Path, scripts: &mut Vec<PathBuf>) {
            for entry in std::fs::read_dir(directory).expect("read repository directory") {
                let path = entry.expect("repository entry").path();
                let relative = path.strip_prefix(root).expect("path inside repository");
                if matches!(
                    relative.components().next().map(std::path::Component::as_os_str),
                    Some(part) if part == ".git" || part == "target" || part == ".agents" || part == ".claude" || part == ".codex"
                ) {
                    continue;
                }
                if relative.starts_with("tools/trainer") {
                    continue;
                }
                // An npm install's third-party packages, which ship their
                // own scripts and are none of this repository's.
                if path.file_name().is_some_and(|name| name == "node_modules") {
                    continue;
                }
                if path.is_dir() {
                    visit(root, &path, scripts);
                } else if matches!(
                    path.extension().and_then(|ext| ext.to_str()),
                    Some("py" | "sh")
                ) {
                    scripts.push(relative.to_path_buf());
                }
            }
        }
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("the repository root")
            .to_path_buf();
        let mut scripts = Vec::new();
        visit(&root, &root, &mut scripts);
        assert!(scripts.is_empty(), "scripts still to migrate: {scripts:?}");
    }
}
