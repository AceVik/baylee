use super::{BTreeMap, CardIndex, Color, ColorSet};
use baylee_core::generated::index::CRYPTIC_SPIRES;
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

    let mut deck_colors = ColorSet::EMPTY;
    for &cid in main.iter().chain(commanders) {
        if let Some(def) = baylee_cards::by_index(cid) {
            deck_colors = deck_colors.union(def.color_identity);
        }
    }
    let spires_suffix = {
        let all = [
            Color::White,
            Color::Blue,
            Color::Black,
            Color::Red,
            Color::Green,
        ];
        let mut cols = all.into_iter().filter(|c| deck_colors.contains(*c));
        let first = cols.next().unwrap_or(Color::White);
        let second = cols
            .next()
            .unwrap_or_else(|| all.into_iter().find(|c| *c != first).unwrap());
        format!(" circles={}{}", first.symbol(), second.symbol())
    };

    let mut main_counts: BTreeMap<String, u32> = BTreeMap::new();
    for &cid in main {
        if let Some(def) = baylee_cards::by_index(cid) {
            let mut row = def.name().to_string();
            if cid == CRYPTIC_SPIRES {
                row.push_str(&spires_suffix);
            }
            *main_counts.entry(row).or_insert(0) += 1;
        }
    }
    for (cname, cnt) in main_counts {
        let _ = writeln!(out, "{cnt} {cname}");
    }

    if !side.is_empty() {
        let _ = writeln!(out, "\n[sideboard]");
        let mut side_counts: BTreeMap<String, u32> = BTreeMap::new();
        for &cid in side {
            if let Some(def) = baylee_cards::by_index(cid) {
                let mut row = def.name().to_string();
                if cid == CRYPTIC_SPIRES {
                    row.push_str(&spires_suffix);
                }
                *side_counts.entry(row).or_insert(0) += 1;
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
