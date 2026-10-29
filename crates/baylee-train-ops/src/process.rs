//! Linux process identity; never infer ownership from a recycled PID alone.
use std::{fs, path::Path};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

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
    Ok(text.rsplit_once(')').context("malformed /proc stat")?.1.split_whitespace().collect())
}

/// Read a live identity from a Linux procfs mount; dead processes return `None`.
#[must_use]
pub fn identity_at(pid: u32, proc: &Path) -> Option<Identity> {
    let text = fs::read_to_string(proc.join(pid.to_string()).join("stat")).ok()?;
    let fields = stat_fields(&text).ok()?;
    if matches!(fields.first().copied(), Some("Z" | "X")) { return None; }
    Some(Identity { pid, start_ticks: fields.get(19)?.parse().ok()?, boot_id: fs::read_to_string(proc.join("sys/kernel/random/boot_id")).ok()?.trim().to_owned() })
}

/// Read a live local process identity.
#[must_use]
pub fn identity(pid: u32) -> Option<Identity> { identity_at(pid, Path::new("/proc")) }

/// Enumerate live members of a process group without reaping its leader.
/// # Errors
/// Returns procfs traversal and malformed process metadata errors.
pub fn group_members(group: u32) -> Result<Vec<u32>> {
    let mut result = Vec::new();
    for entry in fs::read_dir("/proc")? {
        let entry = entry?;
        let Some(pid) = entry.file_name().to_str().and_then(|v| v.parse().ok()) else { continue };
        let Ok(text) = fs::read_to_string(entry.path().join("stat")) else { continue };
        let fields = stat_fields(&text)?;
        if fields.get(2).and_then(|s| s.parse::<u32>().ok()) == Some(group) && !matches!(fields.first().copied(), Some("Z" | "X")) { result.push(pid); }
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
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound || identity(pid).is_none() => continue,
            Err(error) => return Err(error.into()),
        };
        if let Some(value) = text.lines().find_map(|line| line.strip_prefix("Pss:")) {
            total += value.split_whitespace().next().context("PSS value")?.parse::<u64>()? * 1024;
        }
    }
    Ok(total)
}
