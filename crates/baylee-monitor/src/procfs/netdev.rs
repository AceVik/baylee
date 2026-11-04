//! `/proc/net/dev`: each interface's counters since it appeared.

/// One interface's cumulative counters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Counters {
    pub rx_bytes: u64,
    pub rx_packets: u64,
    pub rx_errors: u64,
    pub rx_drops: u64,
    pub tx_bytes: u64,
    pub tx_packets: u64,
    pub tx_errors: u64,
    pub tx_drops: u64,
}

/// Interfaces in the file's order. A malformed line is skipped.
pub(crate) fn parse(text: &str) -> Vec<(String, Counters)> {
    text.lines().skip(2).filter_map(parse_line).collect()
}

fn parse_line(line: &str) -> Option<(String, Counters)> {
    // The last colon: counters are digits, and a name may hold a colon. The
    // first counter can follow the colon without a space.
    let (name, counters) = line.rsplit_once(':')?;
    let mut fields = counters.split_ascii_whitespace();
    let mut next = || fields.next()?.parse::<u64>().ok();
    let rx_bytes = next()?;
    let rx_packets = next()?;
    let rx_errors = next()?;
    let rx_drops = next()?;
    for _fifo_frame_compressed_multicast in 0..4 {
        next()?;
    }
    Some((
        name.trim().to_owned(),
        Counters {
            rx_bytes,
            rx_packets,
            rx_errors,
            rx_drops,
            tx_bytes: next()?,
            tx_packets: next()?,
            tx_errors: next()?,
            tx_drops: next()?,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = concat!(
        "Inter-|   Receive                                                |  Transmit\n",
        " face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed\n",
        "    lo: 553634461  200478    0    0    0     0          0         0 553634461  200478    0    0    0     0       0          0\n",
        "enp5s0: 203029623  181365    3 1877    0     0          0     12323 83762931   91075    1    2    0     0       0          0\n",
        "wlp8s0:0 0 0 0 0 0 0 0 7 1 0 0 0 0 0 0\n",
    );

    #[test]
    fn reads_every_interface_in_order() {
        let interfaces = parse(FIXTURE);
        assert_eq!(interfaces.len(), 3);
        assert_eq!(interfaces[0].0, "lo");
        assert_eq!(
            interfaces[1],
            (
                "enp5s0".to_owned(),
                Counters {
                    rx_bytes: 203_029_623,
                    rx_packets: 181_365,
                    rx_errors: 3,
                    rx_drops: 1877,
                    tx_bytes: 83_762_931,
                    tx_packets: 91_075,
                    tx_errors: 1,
                    tx_drops: 2,
                }
            )
        );
    }

    #[test]
    fn a_counter_may_follow_the_colon_directly() {
        let interfaces = parse(FIXTURE);
        assert_eq!(interfaces[2].0, "wlp8s0");
        assert_eq!(interfaces[2].1.tx_bytes, 7);
    }

    #[test]
    fn short_or_broken_lines_are_skipped() {
        assert!(parse("").is_empty());
        assert!(parse("a\nb\neth0: 1 2 3\nnot an interface\n").is_empty());
    }
}
