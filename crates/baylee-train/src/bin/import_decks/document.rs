use super::{BTreeMap, CardIndex};
use std::fmt::Write as _;

/// Formats a normalized `[deck:Name]` text file from card indices and metadata.
pub(super) fn format_house_deck_text(
    name: &str,
    source_comment: &str,
    license_comment: &str,
    main: &[CardIndex],
    side: &[CardIndex],
    commanders: &[CardIndex],
) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# {source_comment}");
    let _ = writeln!(out, "# licence: {license_comment}");
    let _ = writeln!(out, "[deck:{name}]");

    let mut main_counts: BTreeMap<&'static str, u32> = BTreeMap::new();
    for &cid in main.iter().chain(commanders) {
        if let Some(def) = baylee_cards::by_index(cid) {
            *main_counts.entry(def.name()).or_insert(0) += 1;
        }
    }
    for (cname, cnt) in main_counts {
        let _ = writeln!(out, "{cnt} {cname}");
    }

    if !side.is_empty() {
        let _ = writeln!(out, "\n[sideboard]");
        let mut side_counts: BTreeMap<&'static str, u32> = BTreeMap::new();
        for &cid in side {
            if let Some(def) = baylee_cards::by_index(cid) {
                *side_counts.entry(def.name()).or_insert(0) += 1;
            }
        }
        for (cname, cnt) in side_counts {
            let _ = writeln!(out, "{cnt} {cname}");
        }
    }

    if !commanders.is_empty() {
        let _ = writeln!(out, "\n[commander]");
        for &cid in commanders {
            if let Some(def) = baylee_cards::by_index(cid) {
                let _ = writeln!(out, "{}", def.name());
            }
        }
    }
    out
}
