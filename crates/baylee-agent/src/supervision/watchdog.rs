//! Reads an owned child's private progress pipe and samples only that child's RSS.
use baylee_worker::{Budget, Failure, Limits};
use std::time::{Duration, Instant};
use tokio::io::AsyncReadExt as _;

const SAMPLE: Duration = Duration::from_millis(250);
const EXIT_GRACE: Duration = Duration::from_secs(1);

pub(super) async fn watch(
    reports: Option<tokio::process::ChildStdout>,
    pid: Option<u32>,
    limits: Limits,
) -> Failure {
    let (Some(mut reports), Some(pid)) = (reports, pid) else {
        return Failure::Progress;
    };
    let Ok(mut system) = sysinfo::System::new() else {
        return Failure::Monitor;
    };
    let pid = sysinfo::Pid::from_u32(pid);
    let born = Instant::now();
    let mut budget = Budget::new(limits, Duration::ZERO);
    let mut ticks = tokio::time::interval(SAMPLE);
    ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut bytes = [0; 64];
    let mut closed = None;
    loop {
        tokio::select! {
            biased;
            _ = ticks.tick() => {
                let now = born.elapsed();
                if closed.is_some_and(|at: Duration| now.saturating_sub(at) >= EXIT_GRACE) {
                    return Failure::Progress;
                }
                system.refresh_processes_specifics(
                    sysinfo::ProcessesToUpdate::Some(&[pid]),
                    true,
                    sysinfo::ProcessRefreshKind::nothing().with_memory(),
                );
                // The outer supervisor owns Child::wait, which also wins its
                // select before this future. Never signal a sampled PID.
                if let Some(process) = system.process(pid) {
                    if let Err(failure) = budget.check(now, process.memory()) {
                        return failure;
                    }
                } else if closed.is_none() {
                    closed = Some(now);
                }
            },
            read = reports.read(&mut bytes), if closed.is_none() => {
                match read {
                    Ok(0) => closed = Some(born.elapsed()),
                    Ok(count) => {
                        for &byte in &bytes[..count] {
                            if let Err(failure) = budget.observe(byte, born.elapsed()) {
                                return failure;
                            }
                        }
                    },
                    Err(_) => return Failure::Progress,
                }
            }
        }
    }
}
