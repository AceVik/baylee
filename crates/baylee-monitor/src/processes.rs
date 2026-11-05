//! What sysinfo does not report about a process: its threads, its swap use and
//! its open sockets. A slower pass on its own thread, so that a process with
//! thousands of descriptors never delays the sampler.
//!
//! There is no honest per-process network traffic without privileges or eBPF:
//! the kernel's per-process counters do not exist in `/proc`. What can be read
//! is how many sockets a process holds, and only for this user's processes,
//! because `/proc/<pid>/fd` is closed to everyone else.

use crate::procfs::status;
use std::{collections::BTreeMap, fs, path::Path};

/// Descriptors looked at per pass. A pass past this leaves the remaining
/// processes uncounted (`None`) rather than slow.
const DESCRIPTOR_BUDGET: usize = 50_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Extra {
    pub threads: u32,
    pub swap: u64,
    /// `None`: not this user's process, or the pass ran out of budget.
    pub sockets: Option<u32>,
}

pub(crate) fn scan(proc: &Path) -> BTreeMap<u32, Extra> {
    let mut budget = DESCRIPTOR_BUDGET;
    fs::read_dir(proc)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let pid: u32 = entry.file_name().to_str()?.parse().ok()?;
            let text = fs::read_to_string(entry.path().join("status")).ok()?;
            let status = status::parse(&text);
            let sockets = count_sockets(&entry.path().join("fd"), &mut budget);
            Some((
                pid,
                Extra {
                    threads: status.threads,
                    swap: status.swap,
                    sockets,
                },
            ))
        })
        .collect()
}

fn count_sockets(descriptors: &Path, budget: &mut usize) -> Option<u32> {
    let mut sockets = 0;
    for entry in fs::read_dir(descriptors).ok()? {
        *budget = budget.checked_sub(1)?;
        let target = fs::read_link(entry.ok()?.path());
        // A descriptor closed since the listing is simply gone.
        if target.is_ok_and(|target| target.to_string_lossy().starts_with("socket:")) {
            sockets += 1;
        }
    }
    Some(sockets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Scratch;
    use std::os::unix::fs::symlink;

    fn fake_process(scratch: &Scratch, pid: u32, fds: &[&str]) {
        scratch.write(
            &format!("proc/{pid}/status"),
            "Name:\tx\nThreads:\t4\nVmSwap:\t  8 kB\n",
        );
        let dir = scratch.path(&format!("proc/{pid}/fd"));
        fs::create_dir_all(&dir).unwrap();
        for (at, target) in fds.iter().enumerate() {
            symlink(target, dir.join(at.to_string())).unwrap();
        }
    }

    #[test]
    fn counts_sockets_among_a_processes_descriptors() {
        let scratch = Scratch::new("fds");
        fake_process(
            &scratch,
            42,
            &[
                "socket:[1]",
                "/some/file",
                "pipe:[2]",
                "socket:[3]",
                "anon_inode:[x]",
            ],
        );
        scratch.write("proc/self/status", "Threads:\t1\n");
        scratch.write("proc/version", "not a process");
        let view = scan(&scratch.path("proc"));
        assert_eq!(view.len(), 1);
        assert_eq!(
            view[&42],
            Extra {
                threads: 4,
                swap: 8192,
                sockets: Some(2),
            }
        );
    }

    #[test]
    fn a_closed_descriptor_directory_means_unknown_not_zero() {
        let scratch = Scratch::new("closed");
        scratch.write("proc/7/status", "Threads:\t1\n");
        let view = scan(&scratch.path("proc"));
        assert_eq!(view[&7].sockets, None);
        assert_eq!(view[&7].threads, 1);
    }

    #[test]
    fn a_spent_budget_stops_counting_rather_than_guessing() {
        let scratch = Scratch::new("budget");
        fake_process(&scratch, 1, &["socket:[1]", "socket:[2]", "socket:[3]"]);
        let dir = scratch.path("proc/1/fd");
        let mut budget = 2;
        assert_eq!(count_sockets(&dir, &mut budget), None);
        let mut budget = 3;
        assert_eq!(count_sockets(&dir, &mut budget), Some(3));
    }
}
