//! `/proc/meminfo`, in bytes.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Meminfo {
    pub total: u64,
    pub free: u64,
    pub available: u64,
    pub buffers: u64,
    pub cached: u64,
    pub shared: u64,
    pub slab: u64,
    pub dirty: u64,
    pub writeback: u64,
    pub swap_total: u64,
    pub swap_free: u64,
    pub swap_cached: u64,
}

pub(crate) fn parse(text: &str) -> Meminfo {
    let mut info = Meminfo::default();
    for line in text.lines() {
        let Some((key, rest)) = line.split_once(':') else {
            continue;
        };
        let field = match key {
            "MemTotal" => &mut info.total,
            "MemFree" => &mut info.free,
            "MemAvailable" => &mut info.available,
            "Buffers" => &mut info.buffers,
            "Cached" => &mut info.cached,
            "Shmem" => &mut info.shared,
            "Slab" => &mut info.slab,
            "Dirty" => &mut info.dirty,
            "Writeback" => &mut info.writeback,
            "SwapTotal" => &mut info.swap_total,
            "SwapFree" => &mut info.swap_free,
            "SwapCached" => &mut info.swap_cached,
            _ => continue,
        };
        // Values are in KiB whatever the line's unit word says.
        let kib: u64 = rest
            .split_whitespace()
            .next()
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        *field = kib.saturating_mul(1024);
    }
    info
}

impl Meminfo {
    /// Memory in use by the OS's own accounting: not available for new work.
    pub fn used(&self) -> u64 {
        self.total.saturating_sub(self.available)
    }

    pub fn swap_used(&self) -> u64 {
        self.swap_total.saturating_sub(self.swap_free)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = "MemTotal:       63646720 kB\nMemFree:          405504 kB\n\
        MemAvailable:   49663000 kB\nBuffers:           10240 kB\nCached:         55000000 kB\n\
        SwapCached:           64 kB\nActive:         1 kB\nSwapTotal:      33554428 kB\n\
        SwapFree:       33554396 kB\nDirty:              1024 kB\nWriteback:             0 kB\n\
        Shmem:           6311936 kB\nSlab:            2097152 kB\nHugePages_Total:       0\n";

    #[test]
    fn values_are_bytes_and_used_follows_available() {
        let info = parse(FIXTURE);
        assert_eq!(info.total, 63_646_720 * 1024);
        assert_eq!(info.used(), (63_646_720 - 49_663_000) * 1024);
        assert_eq!(info.swap_used(), 32 * 1024);
        assert_eq!(info.swap_cached, 64 * 1024);
        assert_eq!(info.shared, 6_311_936 * 1024);
        assert_eq!(info.dirty, 1024 * 1024);
    }

    #[test]
    fn an_empty_file_gives_zeroes() {
        assert_eq!(parse(""), Meminfo::default());
        assert_eq!(parse("").used(), 0);
    }
}
