//! Linux process identity; never infer ownership from a recycled PID alone.
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// PID bound to its boot and monotonic process start time.
pub struct Identity {
    /// Kernel process identifier.
    pub pid: u32,
    /// Kernel clock ticks at process creation.
    pub start_ticks: u64,
    /// Linux boot identity, preventing reuse after reboot.
    pub boot_id: String,
}

/// Parse fields after the parenthesized process name, including names with `)`.
/// # Errors
/// Rejects a missing closing parenthesis.
pub fn stat_fields(text: &str) -> Result<Vec<&str>> {
    Ok(text
        .rsplit_once(')')
        .context("malformed /proc stat")?
        .1
        .split_whitespace()
        .collect())
}

/// Read a live identity from a Linux procfs mount; dead processes return `None`.
#[must_use]
pub fn identity_at(pid: u32, proc: &Path) -> Option<Identity> {
    let text = fs::read_to_string(proc.join(pid.to_string()).join("stat")).ok()?;
    let fields = stat_fields(&text).ok()?;
    if matches!(fields.first().copied(), Some("Z" | "X")) {
        return None;
    }
    Some(Identity {
        pid,
        start_ticks: fields.get(19)?.parse().ok()?,
        boot_id: fs::read_to_string(proc.join("sys/kernel/random/boot_id"))
            .ok()?
            .trim()
            .to_owned(),
    })
}

/// Read a live local process identity.
#[must_use]
pub fn identity(pid: u32) -> Option<Identity> {
    identity_at(pid, Path::new("/proc"))
}

/// Enumerate live members of a process group without reaping its leader.
/// # Errors
/// Returns procfs traversal and malformed process metadata errors.
pub fn group_members(group: u32) -> Result<Vec<u32>> {
    let mut result = Vec::new();
    for entry in fs::read_dir("/proc")? {
        let entry = entry?;
        let Some(pid) = entry.file_name().to_str().and_then(|v| v.parse().ok()) else {
            continue;
        };
        let Ok(text) = fs::read_to_string(entry.path().join("stat")) else {
            continue;
        };
        let fields = stat_fields(&text)?;
        if fields.get(2).and_then(|s| s.parse::<u32>().ok()) == Some(group)
            && !matches!(fields.first().copied(), Some("Z" | "X"))
        {
            result.push(pid);
        }
    }
    Ok(result)
}

/// Sum proportional memory for all live members, avoiding shared-page double counting.
/// # Errors
/// Fails when live process memory accounting cannot be read or parsed.
pub fn group_pss_bytes(group: u32) -> Result<u64> {
    let mut total = 0_u64;
    for pid in group_members(group)? {
        let path = format!("/proc/{pid}/smaps_rollup");
        let Some(text) = accounting_text(pid, fs::read_to_string(path))? else {
            continue;
        };
        if let Some(value) = text.lines().find_map(|line| line.strip_prefix("Pss:")) {
            total += value
                .split_whitespace()
                .next()
                .context("PSS value")?
                .parse::<u64>()?
                * 1024;
        }
    }
    Ok(total)
}

fn accounting_text(pid: u32, read: std::io::Result<String>) -> Result<Option<String>> {
    match read {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => Err(error.into()),
        // Procfs can lose the task's mm before stat stops reporting it live.
        // In that interval smaps_rollup returns ESRCH, not ENOENT. It has no
        // address space left to account for; permission failures remain errors.
        Err(error)
            if error.kind() == std::io::ErrorKind::NotFound
                || error.raw_os_error() == Some(rustix::io::Errno::SRCH.raw_os_error())
                || identity(pid).is_none() =>
        {
            Ok(None)
        }
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::Read,
        process::{Child, Command},
    };

    struct NativeChild(Child);
    impl Drop for NativeChild {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    #[test]
    fn exited_native_task_smaps_esrch_is_not_a_memory_accounting_failure() {
        let mut child = NativeChild(Command::new("/usr/bin/sleep").arg("10").spawn().unwrap());
        let mut smaps = fs::File::open(format!("/proc/{}/smaps_rollup", child.0.id())).unwrap();
        child.0.kill().unwrap();
        child.0.wait().unwrap();
        let mut text = String::new();
        let error = smaps.read_to_string(&mut text).unwrap_err();
        assert_eq!(
            error.raw_os_error(),
            Some(rustix::io::Errno::SRCH.raw_os_error())
        );
        // Keep identity live to reproduce mm disappearing before the next stat
        // read notices exit, rather than passing via the old dead-PID fallback.
        assert!(identity(std::process::id()).is_some());
        assert!(
            accounting_text(std::process::id(), Err(error))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn live_process_permission_failure_is_retained() {
        for pid in [std::process::id(), u32::MAX] {
            let error = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
            let result = accounting_text(pid, Err(error)).unwrap_err();
            assert_eq!(
                result.downcast_ref::<std::io::Error>().unwrap().kind(),
                std::io::ErrorKind::PermissionDenied
            );
        }
    }
}
