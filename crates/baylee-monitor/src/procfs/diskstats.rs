//! `/proc/diskstats`: each block device's and partition's I/O counters.

/// The cumulative counters Pulse uses.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Counters {
    pub reads: u64,
    pub read_sectors: u64,
    pub writes: u64,
    pub write_sectors: u64,
    pub in_flight: u64,
    /// Milliseconds the device had I/O in progress.
    pub busy_ms: u64,
}

/// `/proc/diskstats` always counts in 512-byte sectors, whatever the device's.
pub(crate) const SECTOR: u64 = 512;

pub(crate) fn parse(text: &str) -> Vec<(String, Counters)> {
    text.lines().filter_map(parse_line).collect()
}

fn parse_line(line: &str) -> Option<(String, Counters)> {
    let mut fields = line.split_ascii_whitespace();
    let _major = fields.next()?;
    let _minor = fields.next()?;
    let name = fields.next()?.to_owned();
    let mut next = || fields.next()?.parse::<u64>().ok();
    let reads = next()?;
    let _reads_merged = next()?;
    let read_sectors = next()?;
    let _read_ms = next()?;
    let writes = next()?;
    let _writes_merged = next()?;
    let write_sectors = next()?;
    let _write_ms = next()?;
    Some((
        name,
        Counters {
            reads,
            read_sectors,
            writes,
            write_sectors,
            in_flight: next()?,
            busy_ms: next()?,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_disks_and_partitions_with_any_trailing_fields() {
        let text = concat!(
            " 259       5 nvme1n1 473942 84577 66509079 113414 281195 185915 22571322 792324 2 119626 986910 0 0 0 0 142273 81170\n",
            " 259       7 nvme1n1p2 405657 82919 49714490 60041 281192 185915 22571320 792316 0 125567 852357\n",
        );
        let disks = parse(text);
        assert_eq!(disks.len(), 2);
        assert_eq!(
            disks[0],
            (
                "nvme1n1".to_owned(),
                Counters {
                    reads: 473_942,
                    read_sectors: 66_509_079,
                    writes: 281_195,
                    write_sectors: 22_571_322,
                    in_flight: 2,
                    busy_ms: 119_626,
                }
            )
        );
        assert_eq!(disks[1].0, "nvme1n1p2");
    }

    #[test]
    fn short_lines_are_skipped() {
        assert!(parse("").is_empty());
        assert!(parse("  8 0 sda 1 2\n").is_empty());
    }
}
