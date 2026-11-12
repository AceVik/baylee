//! `cargo tools perf`: reading what a profiler exported.
//!
//! `gpu-intervals <table.xml> [part of a process name]` sums an exported
//! `metal-gpu-intervals` table per process, channel and depth. The table
//! comes from a Metal System Trace (`docs/perf-client.md`, "How it was
//! measured"):
//!
//! ```text
//! xcrun xctrace record --template 'Metal System Trace' --attach <pid> --time-limit 5s --output t.trace
//! xcrun xctrace export --input t.trace --xpath '/trace-toc/run[@number="1"]/data/table[@schema="metal-gpu-intervals"]' > t.xml
//! cargo tools perf gpu-intervals t.xml baylee-client
//! ```
//!
//! "Depth" is the lane an interval is drawn in where intervals overlap, not
//! a nesting: a process's GPU time is the sum over every channel and depth.
//!
//! xctrace writes a value once, with an `id`, and afterwards as a `ref`; ids
//! are unique over the document, so one table resolves every column.
//!
//! It was `scripts/perf/gpu-intervals.rs`, built by hand with `rustc`, until
//! the PM's review of 12.11.2026: a program outside the workspace is one no
//! gate builds or lints.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context as _, Result, bail};

pub fn run(_root: &Path, args: &[String]) -> Result<()> {
    match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["gpu-intervals", table, ref rest @ ..] => {
            let xml = std::fs::read_to_string(table).with_context(|| format!("reading {table}"))?;
            print!(
                "{}",
                report(&sum(&xml, rest.first().copied().unwrap_or("")))
            );
            Ok(())
        }
        _ => bail!("usage: cargo tools perf gpu-intervals <table.xml> [part of a process name]"),
    }
}

/// One process's intervals on one channel at one depth.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct Lane {
    intervals: u64,
    nanos: u64,
}

/// A table, summed.
#[derive(Debug, Default, PartialEq, Eq)]
struct Sums {
    /// From the first interval's start to the last one's end, over the rows
    /// that were counted, in nanoseconds.
    span: u64,
    /// By process, channel and depth.
    lanes: BTreeMap<(String, String, String), Lane>,
}

/// An attribute of an opening tag (the text between `<` and `>`).
fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    // With the space before it: `id` is not the end of `pid`.
    let key = format!(" {name}=\"");
    let at = tag.find(&key)? + key.len();
    let end = tag[at..].find('"')? + at;
    Some(&tag[at..end])
}

/// The opening tag of the first or last `<name …>` in `row`, and the text
/// that follows it up to the next tag.
fn column<'a>(row: &'a str, name: &str, last: bool) -> Option<(&'a str, &'a str)> {
    let open = format!("<{name} ");
    let at = if last {
        row.rfind(&open)?
    } else {
        row.find(&open)?
    };
    let close = row[at..].find('>')? + at;
    let text_end = row[close..].find('<').map_or(row.len(), |i| i + close);
    Some((&row[at + 1..close], &row[close + 1..text_end]))
}

/// Sums the intervals of every process whose name contains `want`.
fn sum(xml: &str, want: &str) -> Sums {
    // Every value that has an id: its `fmt` and its text.
    let mut known: HashMap<&str, (&str, &str)> = HashMap::new();
    let mut from = 0;
    while let Some(found) = xml[from..].find(" id=\"") {
        let at = from + found;
        let (Some(open), Some(close)) = (xml[..at].rfind('<'), xml[at..].find('>')) else {
            break;
        };
        let close = close + at;
        let tag = &xml[open + 1..close];
        let text_end = xml[close..].find('<').map_or(xml.len(), |i| i + close);
        if let Some(id) = attr(tag, "id") {
            known.insert(
                id,
                (attr(tag, "fmt").unwrap_or(""), &xml[close + 1..text_end]),
            );
        }
        from = close;
    }
    // A column's value: its own, or the one its `ref` names. `shown` asks
    // for the value as the trace prints it, which is a process's name.
    let value = |(tag, text): (&str, &str), shown: bool| -> String {
        let (fmt, text) = match attr(tag, "ref") {
            Some(id) => known.get(id).copied().unwrap_or(("?", "?")),
            None => (attr(tag, "fmt").unwrap_or(""), text),
        };
        (if shown { fmt } else { text }).to_owned()
    };
    let number = |row: &str, name: &str| -> Option<u64> {
        column(row, name, false).map(|found| value(found, false).parse().unwrap_or(0))
    };

    let mut sums = Sums::default();
    let (mut first, mut last) = (u64::MAX, 0u64);
    for row in xml.split("<row>").skip(1) {
        let (Some(start), Some(nanos)) = (number(row, "start-time"), number(row, "duration"))
        else {
            continue;
        };
        // The last `<process>` of the row: an interval names the process it
        // was submitted by before the one it ran for.
        let process = column(row, "process", true).map_or_else(String::new, |c| value(c, true));
        if !process.contains(want) {
            continue;
        }
        let text = |name: &str| {
            column(row, name, false).map_or_else(String::new, |found| value(found, false))
        };
        first = first.min(start);
        last = last.max(start.saturating_add(nanos));
        let lane = sums
            .lanes
            .entry((
                process,
                text("gpu-channel-name"),
                text("metal-nesting-level"),
            ))
            .or_default();
        lane.intervals += 1;
        lane.nanos += nanos;
    }
    sums.span = last.saturating_sub(first);
    sums
}

/// The sums as the table that is printed: per lane its intervals, its GPU
/// time, and that time per second of the span.
fn report(sums: &Sums) -> String {
    let wall = Duration::from_nanos(sums.span).as_secs_f64();
    let mut out = format!("span {wall:.2} s\n");
    for ((process, channel, depth), lane) in &sums.lanes {
        let ms = Duration::from_nanos(lane.nanos).as_secs_f64() * 1e3;
        let per_second = if wall > 0.0 { ms / wall } else { 0.0 };
        let _ = writeln!(
            out,
            "{process:28} {channel:10} depth {depth}: {:6} intervals, {ms:9.1} ms, {per_second:7.1} ms/s",
            lane.intervals
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Four intervals as xctrace exports them: a value written once with an
    /// `id` and named by `ref` after that, a process inside its own tag, and
    /// a second process that a filter leaves out.
    const TABLE: &str = r#"<?xml version="1.0"?>
<trace-query-result><node xpath='//table[@schema="metal-gpu-intervals"]'><schema name="metal-gpu-intervals"/>
<row><start-time id="1" fmt="00:00.100.000">100000000</start-time><duration id="2" fmt="2.00 ms">2000000</duration><gpu-channel-name id="3" fmt="Vertex">Vertex</gpu-channel-name><metal-nesting-level id="4" fmt="0">0</metal-nesting-level><process id="5" fmt="baylee-client (4242)"><pid id="6" fmt="4242">4242</pid></process></row>
<row><start-time id="7" fmt="00:00.110.000">110000000</start-time><duration ref="2"/><gpu-channel-name ref="3"/><metal-nesting-level ref="4"/><process ref="5"/></row>
<row><start-time id="8" fmt="00:01.100.000">1100000000</start-time><duration id="9" fmt="1.00 ms">1000000</duration><gpu-channel-name id="10" fmt="Fragment">Fragment</gpu-channel-name><metal-nesting-level ref="4"/><process ref="5"/></row>
<row><start-time id="11" fmt="00:00.500.000">500000000</start-time><duration ref="9"/><gpu-channel-name ref="3"/><metal-nesting-level ref="4"/><process id="12" fmt="WindowServer (88)"><pid id="13" fmt="88">88</pid></process></row>
</node></trace-query-result>"#;

    fn lane(sums: &Sums, process: &str, channel: &str) -> Option<Lane> {
        sums.lanes
            .get(&(process.to_owned(), channel.to_owned(), "0".to_owned()))
            .copied()
    }

    #[test]
    fn a_table_is_summed_per_process_channel_and_depth() {
        let sums = sum(TABLE, "baylee");
        assert_eq!(sums.lanes.len(), 2, "{sums:?}");
        // The second interval's duration and channel are `ref`s to the first's.
        assert_eq!(
            lane(&sums, "baylee-client (4242)", "Vertex"),
            Some(Lane {
                intervals: 2,
                nanos: 4_000_000
            })
        );
        assert_eq!(
            lane(&sums, "baylee-client (4242)", "Fragment"),
            Some(Lane {
                intervals: 1,
                nanos: 1_000_000
            })
        );
        // From the first start to the last end, of the rows counted.
        assert_eq!(sums.span, 1_101_000_000 - 100_000_000);

        let text = report(&sums);
        assert!(text.starts_with("span 1.00 s\n"), "{text}");
        assert!(
            text.contains("Vertex     depth 0:      2 intervals,       4.0 ms,     4.0 ms/s"),
            "{text}"
        );
    }

    #[test]
    fn no_filter_counts_every_process_and_a_filter_that_matches_none_counts_nothing() {
        let all = sum(TABLE, "");
        assert_eq!(all.lanes.len(), 3);
        assert_eq!(
            lane(&all, "WindowServer (88)", "Vertex"),
            Some(Lane {
                intervals: 1,
                nanos: 1_000_000
            })
        );
        let none = sum(TABLE, "no such process");
        assert_eq!(none, Sums::default());
        assert_eq!(report(&none), "span 0.00 s\n");
    }

    /// `id` is looked up as an attribute of its own: a `pid` before it is
    /// not it.
    #[test]
    fn an_attribute_is_not_the_end_of_a_longer_one() {
        assert_eq!(attr(r#"thread pid="7" id="3""#, "id"), Some("3"));
        assert_eq!(attr(r#"thread pid="7""#, "id"), None);
    }
}
