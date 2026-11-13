//! Oracle headers belong to the exact printing named beside them.
use super::{Path, Pinned, PrintingTally, diff_lines, pinned_printing, printed_text, squash};

/// Compare the header with its pinned printing, preserving wording and forgiving
/// only whitespace. Reprints can have different reminder text without changing
/// rules identity; comparing a pinned header with today's default creates false
/// failures and lets `refresh-oracle` silently mix two printings.
pub(crate) fn check_oracle_matches_the_printing(
    root: &Path,
    slug: &str,
    content: &str,
    payload: &serde_json::Value,
    tally: &mut PrintingTally,
    problems: &mut usize,
) {
    let pinned = pinned_printing(root, content, payload);
    let printing = match &pinned {
        Pinned::Default(card) => *card,
        Pinned::Other(card) => card,
        Pinned::Missing => {
            println!(
                "{slug}: cannot check Oracle header without its pinned printing; run scryfall-cache"
            );
            *problems += 1;
            return;
        }
    };
    let header = content
        .lines()
        .filter_map(|line| line.strip_prefix("//! Oracle:"))
        .collect::<Vec<_>>()
        .join("\n");
    let printed = printed_text(printing);
    tally.oracle += 1;
    if squash(&header) == squash(&printed) {
        return;
    }
    println!("{slug}: the header's oracle text is not the printing's");
    for line in diff_lines(&header, &printed) {
        println!("    {line}");
    }
    *problems += 1;
}
