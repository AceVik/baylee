//! Nonreaping child observation pins the process-group ID until cleanup ends.
use anyhow::{Context as _, ensure};
use rustix::process::{Pid, Signal, WaitId, WaitIdOptions, kill_process_group, waitid};
use std::{os::unix::process::ExitStatusExt as _, process::ExitStatus, time::Duration};
use tokio::process::Child;

fn owned_pid(child: &Child) -> anyhow::Result<Pid> {
    let raw = i32::try_from(child.id().context("Owned child was already reaped")?)?;
    Pid::from_raw(raw).context("Invalid owned child PID")
}

pub fn peek_exit(child: &mut Child) -> anyhow::Result<Option<ExitStatus>> {
    if child.id().is_none() {
        return Ok(child.try_wait()?);
    }
    let status = waitid(
        WaitId::Pid(owned_pid(child)?),
        WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT,
    )?;
    status
        .map(|status| {
            if let Some(code) = status.exit_status() {
                Ok(ExitStatus::from_raw(code << 8))
            } else {
                Ok(ExitStatus::from_raw(
                    status
                        .terminating_signal()
                        .context("Unknown child exit status")?,
                ))
            }
        })
        .transpose()
}

pub fn signal_owned(child: &Child, signal: Signal) -> anyhow::Result<()> {
    let pid = owned_pid(child)?;
    // ECHILD refuses signaling if another owner has reaped the leader.
    waitid(
        WaitId::Pid(pid),
        WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT,
    )?;
    match kill_process_group(pid, signal) {
        Ok(()) | Err(rustix::io::Errno::SRCH) => Ok(()),
        Err(error) => Err(error.into()),
    }
}

pub fn group_alive(child: &Child) -> anyhow::Result<bool> {
    let pid = child.id().context("Owned child was already reaped")?;
    Ok(!baylee_train_ops::process::group_members(pid)?.is_empty())
}

pub async fn cleanup(child: &mut Child) -> anyhow::Result<()> {
    owned_pid(child)?;
    for signal in [Signal::TERM, Signal::KILL] {
        if !group_alive(child)? {
            break;
        }
        signal_owned(child, signal)?;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        while group_alive(child)? && tokio::time::Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }
    ensure!(
        !group_alive(child)?,
        "Owned child group remains after SIGKILL; preserving leader identity"
    );
    tokio::time::timeout(Duration::from_secs(2), child.wait())
        .await
        .context("Owned child was not reaped before cleanup deadline")??;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::process::Command;

    struct Fixture(Child);
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = signal_owned(&self.0, Signal::TERM);
            let _ = signal_owned(&self.0, Signal::KILL);
        }
    }

    #[tokio::test]
    async fn observation_does_not_reap_and_reaped_ownership_refuses_signals() {
        let mut child = Command::new("true").process_group(0).spawn().unwrap();
        while peek_exit(&mut child).unwrap().is_none() {
            tokio::task::yield_now().await;
        }
        assert!(
            child.id().is_some(),
            "observation must pin the group leader"
        );
        cleanup(&mut child).await.unwrap();
        assert!(signal_owned(&child, Signal::KILL).is_err());
        assert!(peek_exit(&mut child).unwrap().unwrap().success());
    }

    #[test]
    #[ignore = "subprocess fixture; invoked only by owned-descendant cleanup test"]
    #[allow(
        clippy::zombie_processes,
        reason = "Child intentionally outlives its leader; the parent test kills and verifies it"
    )]
    fn exited_leader_fixture() {
        let path = std::env::var_os("BAYLEE_COLLECTOR_DESCENDANT_FIXTURE").unwrap();
        let child = std::process::Command::new("/bin/sleep")
            .arg("60")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        std::fs::write(path, child.id().to_string()).unwrap();
        std::process::exit(0);
    }

    #[tokio::test]
    async fn exited_leader_pins_group_until_remaining_descendant_is_dead() {
        let directory = crate::audit::tests::TestDir::new("owned-descendant");
        let path = directory.path().join("descendant.pid");
        let mut fixture = Fixture(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "processes::tests::exited_leader_fixture",
                    "--ignored",
                ])
                .env("BAYLEE_COLLECTOR_DESCENDANT_FIXTURE", &path)
                .process_group(0)
                .spawn()
                .unwrap(),
        );
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while peek_exit(&mut fixture.0).unwrap().is_none() {
            assert!(tokio::time::Instant::now() < deadline);
            tokio::task::yield_now().await;
        }
        let descendant: u32 = std::fs::read_to_string(path).unwrap().parse().unwrap();
        assert!(baylee_train_ops::process::identity(descendant).is_some());
        cleanup(&mut fixture.0).await.unwrap();
        assert!(baylee_train_ops::process::identity(descendant).is_none());
    }
}
