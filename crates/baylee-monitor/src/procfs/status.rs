//! `/proc/<pid>/status`: only a process's thread count and swap use. The file
//! is world-readable and also names its owner's ids, which stay unparsed.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Status {
    pub threads: u32,
    /// Bytes this process has in swap; kernel threads have no such line.
    pub swap: u64,
}

pub(crate) fn parse(text: &str) -> Status {
    let mut status = Status::default();
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("Threads:") {
            status.threads = value.trim().parse().unwrap_or(0);
        } else if let Some(value) = line.strip_prefix("VmSwap:") {
            let kib: u64 = value
                .split_whitespace()
                .next()
                .and_then(|kib| kib.parse().ok())
                .unwrap_or(0);
            status.swap = kib.saturating_mul(1024);
        }
    }
    status
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_threads_and_swap_only() {
        let text = "Name:\tpython3\nUmask:\t0022\nState:\tS (sleeping)\nUid:\t1000\t1000\t1000\t1000\n\
                    VmRSS:\t  123456 kB\nVmSwap:\t    2048 kB\nThreads:\t17\nSigQ:\t0/255000\n";
        assert_eq!(
            parse(text),
            Status {
                threads: 17,
                swap: 2048 * 1024,
            }
        );
    }

    #[test]
    fn a_kernel_thread_has_no_swap_line() {
        assert_eq!(
            parse("Name:\tkworker/0:0\nThreads:\t1\n"),
            Status {
                threads: 1,
                swap: 0,
            }
        );
    }
}
