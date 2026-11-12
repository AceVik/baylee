//! Sums an exported `metal-gpu-intervals` table per process, channel and
//! nesting depth: `gpu-intervals <table.xml> [part of the process name]`.
//!
//! The table comes from a Metal System Trace (`docs/perf-client.md`, "How it
//! was measured"):
//!
//! ```text
//! xcrun xctrace record --template 'Metal System Trace' --attach <pid> --time-limit 5s --output t.trace
//! xcrun xctrace export --input t.trace --xpath '/trace-toc/run[@number="1"]/data/table[@schema="metal-gpu-intervals"]' > t.xml
//! rustc -O --edition 2024 scripts/perf/gpu-intervals.rs -o /tmp/gpu-intervals
//! /tmp/gpu-intervals t.xml baylee-client
//! ```
//!
//! "Depth" is the lane an interval is drawn in where intervals overlap, not
//! a nesting: a process's GPU time is the sum over every channel and depth.
//!
//! xctrace writes a value once, with an `id`, and afterwards as `ref`; ids
//! are unique over the document, so one table resolves every column.

use std::collections::{BTreeMap, HashMap};
use std::{env, fs};

fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let key = format!("{name}=\"");
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

fn main() {
    let path = env::args().nth(1).expect("an exported table");
    let want = env::args().nth(2).unwrap_or_default();
    let xml = fs::read_to_string(&path).expect("a readable file");

    // Every value that has an id: its `fmt` and its text.
    let mut known: HashMap<&str, (&str, &str)> = HashMap::new();
    let mut from = 0;
    while let Some(found) = xml[from..].find(" id=\"") {
        let at = from + found;
        let open = xml[..at].rfind('<').expect("an id inside a tag");
        let close = xml[at..].find('>').expect("a closed tag") + at;
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
    let value = |tag: &'_ str, text: &'_ str, fmt: bool| -> String {
        let (f, t) = match attr(tag, "ref") {
            Some(id) => known.get(id).copied().unwrap_or(("?", "?")),
            None => (attr(tag, "fmt").unwrap_or(""), text),
        };
        (if fmt { f } else { t }).to_owned()
    };

    let mut sums: BTreeMap<(String, String, String), (u64, u64)> = BTreeMap::new();
    let (mut first, mut last) = (u64::MAX, 0u64);
    for row in xml.split("<row>").skip(1) {
        let Some((tag, text)) = column(row, "start-time", false) else {
            continue;
        };
        let start: u64 = value(tag, text, false).parse().unwrap_or(0);
        let Some((tag, text)) = column(row, "duration", false) else {
            continue;
        };
        let ns: u64 = value(tag, text, false).parse().unwrap_or(0);
        let process =
            column(row, "process", true).map_or_else(String::new, |(t, x)| value(t, x, true));
        if !process.contains(&want) {
            continue;
        }
        let channel = column(row, "gpu-channel-name", false)
            .map_or_else(String::new, |(t, x)| value(t, x, false));
        let depth = column(row, "metal-nesting-level", false)
            .map_or_else(String::new, |(t, x)| value(t, x, false));
        first = first.min(start);
        last = last.max(start + ns);
        let entry = sums.entry((process, channel, depth)).or_default();
        entry.0 += 1;
        entry.1 += ns;
    }
    let wall = (last.saturating_sub(first)) as f64 / 1e9;
    println!("span {wall:.2} s");
    for ((process, channel, depth), (count, ns)) in &sums {
        let ms = *ns as f64 / 1e6;
        println!(
            "{process:28} {channel:10} depth {depth}: {count:6} intervals, {ms:9.1} ms, {:7.1} ms/s",
            ms / wall
        );
    }
}
