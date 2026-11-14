use crate::Error;
use std::{path::Path, process::Stdio, time::Duration};
use tokio::process::{Child, Command};

mod progress;
mod stream;
#[cfg(test)]
mod tests;
pub(crate) use progress::progress;
pub(crate) use stream::{read, write};

pub(crate) fn command(path: &Path, mode: &str) -> Command {
    let mut command = Command::new(path);
    command
        .arg(mode)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    command
}

pub(crate) async fn reap(child: &mut Child, kill: bool) -> Result<(), Error> {
    if kill {
        let _ = child.start_kill();
    }
    // Retain the owned handle/capacity until wait confirms exit; never signal a PID.
    child.wait().await.map_err(|_| Error::Process)?;
    Ok(())
}

pub(crate) async fn memory(pid: u32, limit: u64) -> Error {
    let Ok(mut system) = sysinfo::System::new() else {
        return Error::Process;
    };
    let pid = sysinfo::Pid::from_u32(pid);
    let mut ticks = tokio::time::interval(Duration::from_millis(250));
    ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        ticks.tick().await;
        system.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::Some(&[pid]),
            true,
            sysinfo::ProcessRefreshKind::nothing().with_memory(),
        );
        if system
            .process(pid)
            .is_some_and(|process| process.memory() >= limit)
        {
            return Error::Budget;
        }
        // A quick child may already have exited. Its owned wait/output paths prove
        // completion; a missing sample neither fabricates a result nor signals it.
    }
}
