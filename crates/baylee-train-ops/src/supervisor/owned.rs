//! Keep direct children unreaped until their owned process groups are empty.
use crate::process;
use anyhow::{Context, Result, ensure};
use rustix::process::{Pid, Signal, WaitId, WaitIdOptions, kill_process_group, waitid};
use std::{
    process::Child,
    time::{Duration, Instant},
};
pub(super) struct Owned {
    pub child: Child,
    pub identity: process::Identity,
    reaped: bool,
}
impl Owned {
    pub fn new(child: Child) -> Result<Self> {
        // Establish RAII cleanup before reading procfs: even metadata failure
        // cannot discard an owned live group or its unreaped leader.
        let mut owned = Self {
            identity: process::Identity {
                pid: child.id(),
                start_ticks: 0,
                boot_id: String::new(),
            },
            child,
            reaped: false,
        };
        let stat = std::fs::read_to_string(format!("/proc/{}/stat", owned.child.id()))?;
        let fields = process::stat_fields(&stat)?;
        owned.identity.start_ticks = fields.get(19).context("child start time")?.parse()?;
        std::fs::read_to_string("/proc/sys/kernel/random/boot_id")?
            .trim()
            .clone_into(&mut owned.identity.boot_id);
        Ok(owned)
    }
    fn pid(&self) -> Result<Pid> {
        Pid::from_raw(i32::try_from(self.child.id())?).context("child PID")
    }
    pub fn exited(&self) -> Result<bool> {
        ensure!(!self.reaped, "owned child already reaped");
        Ok(waitid(
            WaitId::Pid(self.pid()?),
            WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT,
        )?
        .is_some())
    }
    pub fn alive(&self) -> Result<()> {
        ensure!(
            !self.exited()? && process::identity(self.child.id()).as_ref() == Some(&self.identity),
            "owned child exited or identity changed"
        );
        Ok(())
    }
    fn signal(&self, signal: Signal) -> Result<()> {
        self.exited()?; // A valid unreaped wait relationship pins leader/group identity.
        match kill_process_group(self.pid()?, signal) {
            Ok(()) | Err(rustix::io::Errno::SRCH) => Ok(()),
            Err(error) => Err(error.into()),
        }
    }
    pub fn cleanup(&mut self) -> Result<()> {
        if self.reaped {
            return Ok(());
        }
        for (signal, seconds) in [(Signal::TERM, 15), (Signal::KILL, 5)] {
            if !process::group_members(self.child.id())?.is_empty() {
                self.signal(signal)?;
            }
            let deadline = Instant::now() + Duration::from_secs(seconds);
            while Instant::now() < deadline {
                if self.exited()? && process::group_members(self.child.id())?.is_empty() {
                    self.child.wait()?;
                    self.reaped = true;
                    return Ok(());
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }
        anyhow::bail!("owned supervisor descendants remain; leader left unreaped")
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        if !self.reaped {
            let _ = self.cleanup();
        }
    }
}
