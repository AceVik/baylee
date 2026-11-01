//! Tart image preparation, one-job VMs, and their guaranteed cleanup.

use crate::github::{GitHub, Registration};
use crate::process::{Operations, Process, Spec, require_success};
use crate::settings::Settings;
use anyhow::{Result, bail};
use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime};

const RUNNER_DIR: &str = "/Users/admin/actions-runner";
const GUARD: &str = "/Users/admin/job-guard.sh";

/// The job-started hook: only `repo`'s pushes to main and its dispatches run
/// in a VM. GitHub hands a just-in-time runner whichever queued job names its
/// label, and a fork's pull request writes its own `runs-on`; the fork policy
/// asks for approval, and this refuses the job even then (29.10.2026). A
/// hook that fails stops the job before its first step.
fn job_guard(repo: &str) -> String {
    format!(
        "#!/bin/sh\n\
         case \"$GITHUB_REPOSITORY:$GITHUB_EVENT_NAME:$GITHUB_REF\" in\n\
         {repo}:push:refs/heads/main | {repo}:workflow_dispatch:*) exit 0 ;;\n\
         esac\n\
         echo \"baylee-vm runs only main pushes and dispatches of {repo}, not $GITHUB_REPOSITORY $GITHUB_EVENT_NAME $GITHUB_REF\" >&2\n\
         exit 1\n"
    )
}

/// What the guest runs: the JIT configuration from stdin, the guard as the
/// job-started hook, then the runner.
fn start_script(settings: &Settings) -> String {
    format!(
        "set -eu; IFS= read -r ACTIONS_RUNNER_INPUT_JITCONFIG; export ACTIONS_RUNNER_INPUT_JITCONFIG; \
         export PATH=/Users/admin/.cargo/bin:/opt/homebrew/bin:/opt/homebrew/sbin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin; \
         cat > {GUARD} <<'GUARD'\n{guard}GUARD\n\
         chmod 755 {GUARD}; export ACTIONS_RUNNER_HOOK_JOB_STARTED={GUARD}; \
         cd {RUNNER_DIR}; exec ./run.sh",
        guard = job_guard(&settings.repo),
    )
}

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
    Spec::new(
        "tart",
        ["exec", "-i", vm, "/bin/sh", "-c", &start_script(settings)],
    )
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
///
/// `on_network` hears once the VM reached github.com; it is dropped unheard
/// when the VM gave up before that.
pub fn one_job_named(
    settings: &Settings,
    operations: &mut dyn Operations,
    name: &str,
    token: &str,
    log_dir: &Path,
    on_network: Option<mpsc::Sender<()>>,
) -> Result<ExitStatus> {
    let mut vm = Vm::clone(operations, &settings.image, name)?;
    vm.log = Some(log_dir.join(format!("{name}.log")));
    vm.start(vm.log.clone())?;
    wait_for_agent(&mut vm)?;
    wait_for_network(&mut vm)?;
    if let Some(on_network) = on_network {
        let _ = on_network.send(());
    }
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

/// Runs one-job VMs with these names side by side and says whether any of
/// them served a job. Each VM is started only once the one before it reached
/// the network or gave up: two VMs started together left the second one
/// without name resolution (22.10.2026). Without a worker boundary they run
/// one after the other.
pub fn run_jobs(
    settings: &Settings,
    operations: &mut dyn Operations,
    names: &[String],
    token: &str,
    log_dir: &Path,
) -> bool {
    let mut served = false;
    let mut report = |name: &str, result: Result<ExitStatus>| match result {
        Ok(status) if status.success() => served = true,
        Ok(status) => crate::log(format!("{name}: runner failed ({status})")),
        Err(error) => crate::log(format!("{name}: {error}")),
    };
    std::thread::scope(|scope| {
        let mut running = Vec::new();
        for name in names {
            if operations.cancelled() {
                break;
            }
            let worker = if names.len() > 1 {
                operations.worker()
            } else {
                None
            };
            let Some(mut worker) = worker else {
                let result = one_job_named(settings, operations, name, token, log_dir, None);
                report(name, result);
                continue;
            };
            let (on_network, network) = mpsc::channel();
            running.push((
                name,
                scope.spawn(move || {
                    one_job_named(
                        settings,
                        worker.as_mut(),
                        name,
                        token,
                        log_dir,
                        Some(on_network),
                    )
                }),
            ));
            // Either a message or the sender's drop ends the wait.
            let _ = network.recv();
        }
        for (name, job) in running {
            let result = job
                .join()
                .unwrap_or_else(|_| Err(anyhow::anyhow!("the job thread panicked")));
            report(name, result);
        }
    });
    served
}

/// Removes what a runner killed outright left behind: stopped VMs named for
/// this label and this label's offline registrations. `serve` calls it once,
/// before it serves anything; a running VM or an online runner is left alone.
pub fn sweep(settings: &Settings, operations: &mut dyn Operations, token: &str) {
    let prefix = format!("{}-", settings.label);
    let listed = operations.output(
        Spec::new("tart", ["list", "--source", "local", "--format", "json"])
            .with_timeout(Duration::from_secs(30)),
    );
    let vms: serde_json::Value = listed
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| serde_json::from_slice(&output.stdout).ok())
        .unwrap_or_default();
    for vm in vms.as_array().into_iter().flatten() {
        let Some(name) = vm.get("Name").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let stopped = vm.get("State").and_then(serde_json::Value::as_str) == Some("stopped");
        if name.starts_with(&prefix) && stopped {
            crate::log(format!(
                "{name}: left behind by an earlier runner, deleting"
            ));
            let _ = operations.output(
                Spec::new("tart", ["delete", name])
                    .with_timeout(Duration::from_secs(60))
                    .for_cleanup(),
            );
        }
    }
    let mut github = GitHub::new(operations, &settings.repo, token);
    for (id, name) in github.offline_runners(&prefix) {
        crate::log(format!(
            "{name}: registration left behind, removing runner {id}"
        ));
        github.remove(id);
    }
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
