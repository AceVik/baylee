//! `/proc/vmstat`: swap traffic, major page faults and OOM kills since boot.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Vmstat {
    /// Pages read from swap.
    pub swap_in: u64,
    /// Pages written to swap.
    pub swap_out: u64,
    pub major_faults: u64,
    /// Processes the kernel's OOM killer has ended.
    pub oom_kills: u64,
}

pub(crate) fn parse(text: &str) -> Vmstat {
    let mut vmstat = Vmstat::default();
    for line in text.lines() {
        let Some((key, value)) = line.split_once(' ') else {
            continue;
        };
        let field = match key {
            "pswpin" => &mut vmstat.swap_in,
            "pswpout" => &mut vmstat.swap_out,
            "pgmajfault" => &mut vmstat.major_faults,
            "oom_kill" => &mut vmstat.oom_kills,
            _ => continue,
        };
        *field = value.trim().parse().unwrap_or(0);
    }
    vmstat
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_swap_traffic_and_skips_everything_else() {
        let text = "nr_free_pages 123\npgpgin 499783818\npswpin 12\npswpout 8\n\
                    pgfault 342870362\npgmajfault 116396\noom_kill 2\nnew_counter x\n";
        assert_eq!(
            parse(text),
            Vmstat {
                swap_in: 12,
                swap_out: 8,
                major_faults: 116_396,
                oom_kills: 2,
            }
        );
    }

    #[test]
    fn missing_fields_stay_zero() {
        assert_eq!(parse(""), Vmstat::default());
        assert_eq!(parse("pswpin\npswpout nonsense\n"), Vmstat::default());
    }
}
