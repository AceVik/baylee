//! Tart image preparation, one-job VMs, and their guaranteed cleanup.

use crate::github::{GitHub, Registration};
use crate::process::{Operations, Process, Spec, require_success};
use crate::settings::Settings;
use anyhow::{Result, bail};
use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use std::time::{Duration, Instant, SystemTime};

const RUNNER_DIR: &str = "/Users/admin/actions-runner";
const START_SCRIPT: &str = "set -eu; IFS= read -r ACTIONS_RUNNER_INPUT_JITCONFIG; export ACTIONS_RUNNER_INPUT_JITCONFIG; export PATH=/Users/admin/.cargo/bin:/opt/homebrew/bin:/opt/homebrew/sbin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin; cd /Users/admin/actions-runner; exec ./run.sh";

struct Vm<'a> {
    name: String,
    operations: &'a mut dyn Operations,
    tart: Option<Box<dyn Process>>,
    log: Option<PathBuf>,
    delete_on_drop: bool,
}

impl<'a> Vm<'a> {
    fn clone(operations: &'a mut dyn Operations, image: &str, name: &str) -> Result<Self> {
        let output = operations.output(
            Spec::new("tart", ["clone", image, name]).with_timeout(Duration::from_secs(10_800)),
        )?;
        require_success(&output, "tart clone")?;
        Ok(Self {
            name: name.to_owned(),
            operations,
            tart: None,
            log: None,
            delete_on_drop: true,
        })
    }

    fn start(&mut self, log: Option<PathBuf>) -> Result<()> {
        let mut spec = Spec::new(
            "tart",
            ["run", &self.name, "--no-graphics", "--net-softnet"],
        );
        if let Some(path) = log {
            spec = spec.with_log(path);
        }
        self.tart = Some(self.operations.spawn(spec)?);
        Ok(())
    }

    fn exec<I, S>(&self, args: I, timeout: Duration) -> Spec
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut all = vec!["exec".to_owned(), self.name.clone()];
        all.extend(args.into_iter().map(Into::into));
        Spec::new("tart", all).with_timeout(timeout)
    }

    fn checked_exec<I, S>(&mut self, args: I) -> Result<()>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let output = self
            .operations
            .output(self.exec(args, Duration::from_secs(600)))?;
        require_success(&output, "tart exec")
    }

    fn stop(&mut self) {
        let _ = self.operations.output(
            Spec::new("tart", ["stop", &self.name])
                .with_timeout(Duration::from_secs(30))
                .for_cleanup(),
        );
        if let Some(mut tart) = self.tart.take() {
            let _ = tart.wait(Duration::from_secs(60));
        }
    }

    fn keep(mut self) {
        self.delete_on_drop = false;
    }
}

impl Drop for Vm<'_> {
    fn drop(&mut self) {
        self.stop();
        if self.delete_on_drop {
            let _ = self.operations.output(
                Spec::new("tart", ["delete", &self.name])
                    .with_timeout(Duration::from_secs(60))
                    .for_cleanup(),
            );
        }
        if let Some(path) = &self.log {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn wait_for_agent(vm: &mut Vm<'_>) -> Result<()> {
    for _ in 0..90 {
        if vm.operations.cancelled() {
            bail!("runner shutdown requested");
        }
        if vm
            .operations
            .output(vm.exec(["true"], Duration::from_secs(10)))
            .is_ok_and(|output| output.status.success())
        {
            return Ok(());
        }
        vm.operations.sleep(Duration::from_secs(2));
    }
    bail!("{}: the guest agent never answered", vm.name)
}

fn wait_for_network(vm: &mut Vm<'_>) -> Result<()> {
    for _ in 0..30 {
        if vm.operations.cancelled() {
            bail!("runner shutdown requested");
        }
        if vm
            .operations
            .output(vm.exec(
                [
                    "/usr/bin/curl",
                    "-sS",
                    "-m",
                    "3",
                    "-o",
                    "/dev/null",
                    "https://github.com",
                ],
                Duration::from_secs(10),
            ))
            .is_ok_and(|output| output.status.success())
        {
            return Ok(());
        }
        vm.operations.sleep(Duration::from_secs(2));
    }
    bail!("{}: no way to github.com after 30 attempts", vm.name)
}

fn start_runner_spec(vm: &str, registration: &Registration, settings: &Settings) -> Spec {
    Spec::new("tart", ["exec", "-i", vm, "/bin/sh", "-c", START_SCRIPT])
        .with_stdin(format!("{}\n", registration.encoded).into_bytes())
        .with_prefix(format!("{vm}:"))
        .with_timeout(settings.job_cap + Duration::from_secs(60))
}

fn run_registered_job(
    vm: &mut Vm<'_>,
    settings: &Settings,
    token: &str,
    registration: &Registration,
) -> Result<ExitStatus> {
    let mut process = vm
        .operations
        .spawn(start_runner_spec(&vm.name, registration, settings))?;
    let started = Instant::now();
    let mut checked_busy = false;
    let status = loop {
        if vm.operations.cancelled() {
            bail!("runner shutdown requested");
        }
        if checked_busy && started.elapsed() >= settings.job_cap {
            crate::log(format!(
                "{}: still running after {}s, stopping",
                vm.name,
                settings.job_cap.as_secs()
            ));
            break None;
        }
        if !checked_busy && started.elapsed() >= settings.idle {
            checked_busy = true;
            let mut github = GitHub::new(vm.operations, &settings.repo, token);
            if github.busy(registration.id) == Some(false) {
                crate::log(format!(
                    "{}: no job after {}s, stopping",
                    vm.name,
                    settings.idle.as_secs()
                ));
                break None;
            }
        }
        if let Some(status) = process.try_wait()? {
            break Some(status);
        }
        vm.operations.sleep(Duration::from_millis(100));
    };
    let status = if let Some(status) = status {
        status
    } else {
        vm.stop();
        vm.operations.sleep(Duration::from_secs(30));
        process.kill()?;
        process.wait(Duration::from_secs(60))?
    };
    crate::log(format!("{}: runner exited ({status})", vm.name));
    Ok(status)
}

/// Runs one named VM to completion and then removes both it and its registration.
pub fn one_job_named(
    settings: &Settings,
    operations: &mut dyn Operations,
    name: &str,
    token: &str,
    log_dir: &Path,
) -> Result<ExitStatus> {
    let mut vm = Vm::clone(operations, &settings.image, name)?;
    vm.log = Some(log_dir.join(format!("{name}.log")));
    vm.start(vm.log.clone())?;
    wait_for_agent(&mut vm)?;
    wait_for_network(&mut vm)?;
    let registration = {
        let mut github = GitHub::new(vm.operations, &settings.repo, token);
        github.register(&vm.name, &settings.label)?
    };
    crate::log(format!(
        "{}: registered as runner {}",
        vm.name, registration.id
    ));
    let result = run_registered_job(&mut vm, settings, token, &registration);
    GitHub::new(vm.operations, &settings.repo, token).remove(registration.id);
    result
}

/// Gives a batch job a unique VM name without depending on a locale or `date`.
#[must_use]
pub fn job_name(settings: &Settings, index: usize) -> String {
    let seconds = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}-{seconds}-{index}", settings.label)
}

/// Builds or refreshes the image used by the one-job VMs.
pub fn prepare(settings: &Settings, operations: &mut dyn Operations) -> Result<()> {
    let output = operations.output(Spec::new("tart", ["list", "--quiet", "--source", "local"]))?;
    require_success(&output, "tart list")?;
    let existing = String::from_utf8_lossy(&output.stdout);
    if existing.lines().any(|line| line.trim() == settings.image) {
        bail!("{} exists; delete it first", settings.image);
    }

    let mut vm = Vm::clone(operations, &settings.base, &settings.image)?;
    let cpus = settings.vm_cpus.to_string();
    let memory = settings.vm_memory_mb.to_string();
    let disk = settings.vm_disk_gb.to_string();
    let output = vm.operations.output(Spec::new(
        "tart",
        [
            "set",
            &vm.name,
            "--cpu",
            &cpus,
            "--memory",
            &memory,
            "--disk-size",
            &disk,
        ],
    ))?;
    require_success(&output, "tart set")?;
    vm.start(None)?;
    wait_for_agent(&mut vm)?;

    vm.checked_exec([
        "/usr/bin/curl",
        "--proto",
        "=https",
        "--tlsv1.2",
        "-fsSLo",
        "/tmp/rustup-init.sh",
        "https://sh.rustup.rs",
    ])?;
    vm.checked_exec([
        "/bin/sh",
        "/tmp/rustup-init.sh",
        "-y",
        "--profile",
        "minimal",
        "--default-toolchain",
        "stable",
        "--target",
        "x86_64-apple-darwin",
    ])?;
    vm.checked_exec(["/bin/mkdir", "-p", RUNNER_DIR])?;
    let archive = format!(
        "https://github.com/actions/runner/releases/download/v{}/actions-runner-osx-arm64-{}.tar.gz",
        settings.runner_version, settings.runner_version
    );
    vm.checked_exec([
        "/usr/bin/curl",
        "-fsSLo",
        "/Users/admin/runner.tar.gz",
        &archive,
    ])?;
    let output = vm.operations.output(vm.exec(
        ["/usr/bin/shasum", "-a", "256", "/Users/admin/runner.tar.gz"],
        Duration::from_secs(60),
    ))?;
    require_success(&output, "shasum")?;
    let checksum = String::from_utf8_lossy(&output.stdout);
    if !checksum.starts_with(&settings.runner_sha256) {
        bail!("actions runner archive checksum mismatch");
    }
    vm.checked_exec([
        "/usr/bin/tar",
        "-xzf",
        "/Users/admin/runner.tar.gz",
        "-C",
        RUNNER_DIR,
    ])?;
    vm.checked_exec([
        "/bin/rm",
        "/Users/admin/runner.tar.gz",
        "/tmp/rustup-init.sh",
    ])?;
    vm.checked_exec(["/usr/bin/arch", "-x86_64", "/usr/bin/true"])?;
    vm.keep();
    crate::log(format!("{} is ready", settings.image));
    Ok(())
}

#[cfg(test)]
mod tests;
