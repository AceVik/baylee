//! What changed between two versions of a deck, row by row, in words a
//! player uses: a card added or cut, a count changed, a different printing,
//! a different finish, a different language, a note.
//!
//! A deck is stored as [`crate::deckrow`] strings, so the naive diff — the
//! whole row as the key — reads a foil change as `−1 Lightning Bolt` plus
//! `+1 Lightning Bolt *F*`. Here rows are keyed on the card **name** within
//! one zone, matched exactly first, and what is left of one name is paired
//! printing by printing, so the same change reads as one `Finish` row.
//!
//! The gateway answers each version's [`Delta`] from this (WG-6) and the
//! client draws its history sheet from the same [`diff_zone`], so the summary
//! on a version row and the rows under it can never disagree.

use crate::deckrow::{self, PrintChoice, Row};
use crate::preset::Finish;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One change to one card within one zone.
///
/// Every variant names the card and carries a printing to draw it with: the
/// right-hand (newer) side's, or the removed one's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    /// The name, or more copies of a printing, appears only on the newer side.
    Added {
        /// Card name as the row wrote it.
        name: String,
        /// Copies added.
        count: u32,
        /// The added printing.
        print: PrintChoice,
    },
    /// The name, or copies of a printing, appears only on the older side.
    Removed {
        /// Card name as the row wrote it.
        name: String,
        /// Copies removed.
        count: u32,
        /// The removed printing.
        print: PrintChoice,
    },
    /// The same printing with a different count.
    Count {
        /// Card name as the row wrote it.
        name: String,
        /// Copies before.
        from: u32,
        /// Copies after.
        to: u32,
        /// The printing both sides run.
        print: PrintChoice,
    },
    /// The same name in a different printing (set, number or exact id).
    Printing {
        /// Card name as the row wrote it.
        name: String,
        /// Copies that changed printing.
        count: u32,
        /// The older printing.
        from: PrintChoice,
        /// The newer printing.
        to: PrintChoice,
    },
    /// The same printing in a different finish.
    Finish {
        /// Card name as the row wrote it.
        name: String,
        /// Copies that changed finish.
        count: u32,
        /// The older finish.
        from: Finish,
        /// The newer finish.
        to: Finish,
        /// The newer printing.
        print: PrintChoice,
    },
    /// The same printing in a different language.
    Language {
        /// Card name as the row wrote it.
        name: String,
        /// Copies that changed language.
        count: u32,
        /// The older language code.
        from: String,
        /// The newer language code.
        to: String,
        /// The newer printing.
        print: PrintChoice,
    },
    /// The same printing with a different note.
    Note {
        /// Card name as the row wrote it.
        name: String,
        /// Copies the note is on.
        count: u32,
        /// The newer printing.
        print: PrintChoice,
    },
}

impl Change {
    /// The card's name.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Added { name, .. }
            | Self::Removed { name, .. }
            | Self::Count { name, .. }
            | Self::Printing { name, .. }
            | Self::Finish { name, .. }
            | Self::Language { name, .. }
            | Self::Note { name, .. } => name,
        }
    }

    /// The printing to draw the row with.
    #[must_use]
    pub fn print(&self) -> &PrintChoice {
        match self {
            Self::Printing { to, .. } => to,
            Self::Added { print, .. }
            | Self::Removed { print, .. }
            | Self::Count { print, .. }
            | Self::Finish { print, .. }
            | Self::Language { print, .. }
            | Self::Note { print, .. } => print,
        }
    }
}

/// Every change within one zone, ordered by card name.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ZoneDiff {
    /// The changes.
    pub changes: Vec<Change>,
}

/// How much changed, counted: copies for `added` / `removed`, rows for the
/// rest. What a version row in the history says in one line.
#[derive(Default, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Delta {
    /// Copies added (new names, and count increases).
    pub added: u32,
    /// Copies removed (cut names, and count decreases).
    pub removed: u32,
    /// Rows whose count changed.
    pub count: u32,
    /// Rows that changed printing.
    pub printing: u32,
    /// Rows that changed finish.
    pub finish: u32,
    /// Rows that changed language.
    pub language: u32,
    /// Rows whose note changed.
    pub note: u32,
}

impl Delta {
    /// Whether nothing changed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    fn add(&mut self, other: Self) {
        self.added += other.added;
        self.removed += other.removed;
        self.count += other.count;
        self.printing += other.printing;
        self.finish += other.finish;
        self.language += other.language;
        self.note += other.note;
    }
}

impl ZoneDiff {
    /// The changes, counted.
    #[must_use]
    pub fn delta(&self) -> Delta {
        let mut delta = Delta::default();
        for change in &self.changes {
            match change {
                Change::Added { count, .. } => delta.added += count,
                Change::Removed { count, .. } => delta.removed += count,
                Change::Count { from, to, .. } => {
                    delta.count += 1;
                    if to > from {
                        delta.added += to - from;
                    } else {
                        delta.removed += from - to;
                    }
                }
                Change::Printing { .. } => delta.printing += 1,
                Change::Finish { .. } => delta.finish += 1,
                Change::Language { .. } => delta.language += 1,
                Change::Note { .. } => delta.note += 1,
            }
        }
        delta
    }
}

/// Reads a row; one this build cannot parse stands as one copy of its own
/// text, so a diff never loses a row it does not understand.
fn read(row: &str) -> Row {
    deckrow::parse(row).unwrap_or_else(|_| Row::plain(1, row.trim()))
}

/// The rows of one zone grouped by name, exact duplicates summed.
fn group(rows: &[String]) -> BTreeMap<String, Vec<Row>> {
    let mut by_name: BTreeMap<String, Vec<Row>> = BTreeMap::new();
    for row in rows.iter().map(|r| read(r)) {
        let same = by_name.entry(row.name.clone()).or_default();
        match same
            .iter_mut()
            .find(|r| r.print == row.print && r.note == row.note)
        {
            Some(existing) => existing.count += row.count,
            None => same.push(row),
        }
    }
    by_name
}

/// The printing itself, without finish and language: what makes two rows
/// "the same printing".
fn printing(print: &PrintChoice) -> (&Option<String>, &Option<String>, &Option<String>) {
    (&print.set, &print.collector_number, &print.scryfall_id)
}

/// How alike two rows of one name are, for pairing what is left over.
fn likeness(a: &Row, b: &Row) -> u8 {
    let mut score = 0;
    if printing(&a.print) == printing(&b.print) {
        score += 8;
    }
    if a.print.finish_or_default() == b.print.finish_or_default() {
        score += 4;
    }
    if a.print.lang_or_default() == b.print.lang_or_default() {
        score += 2;
    }
    if a.note == b.note {
        score += 1;
    }
    score
}

/// The changes between one paired older and newer row of one name.
fn pair(name: &str, old: &Row, new: &Row, out: &mut Vec<Change>) {
    let shared = old.count.min(new.count);
    let before = out.len();
    if printing(&old.print) == printing(&new.print) {
        let (from, to) = (old.print.finish_or_default(), new.print.finish_or_default());
        if from != to {
            out.push(Change::Finish {
                name: name.to_string(),
                count: shared,
                from,
                to,
                print: new.print.clone(),
            });
        }
        let (from, to) = (old.print.lang_or_default(), new.print.lang_or_default());
        if from != to {
            out.push(Change::Language {
                name: name.to_string(),
                count: shared,
                from: from.to_string(),
                to: to.to_string(),
                print: new.print.clone(),
            });
        }
        if old.note != new.note {
            out.push(Change::Note {
                name: name.to_string(),
                count: shared,
                print: new.print.clone(),
            });
        }
    } else {
        out.push(Change::Printing {
            name: name.to_string(),
            count: shared,
            from: old.print.clone(),
            to: new.print.clone(),
        });
    }
    if out.len() == before {
        // Spelled differently, meaning the same (`[en]` against nothing):
        // only the count can have changed.
        if old.count != new.count {
            out.push(Change::Count {
                name: name.to_string(),
                from: old.count,
                to: new.count,
                print: new.print.clone(),
            });
        }
        return;
    }
    if new.count > old.count {
        out.push(Change::Added {
            name: name.to_string(),
            count: new.count - old.count,
            print: new.print.clone(),
        });
    } else if old.count > new.count {
        out.push(Change::Removed {
            name: name.to_string(),
            count: old.count - new.count,
            print: old.print.clone(),
        });
    }
}

/// What changed in one zone from `before` to `after` (deck row strings).
#[must_use]
pub fn diff_zone(before: &[String], after: &[String]) -> ZoneDiff {
    let mut old = group(before);
    let mut new = group(after);
    let names: std::collections::BTreeSet<String> = old.keys().chain(new.keys()).cloned().collect();
    let mut changes = Vec::new();
    for name in names {
        let mut olds = old.remove(&name).unwrap_or_default();
        let mut news = new.remove(&name).unwrap_or_default();
        // Exact matches first: the same printing and note on both sides.
        olds.retain(|o| {
            let Some(at) = news
                .iter()
                .position(|n| n.print == o.print && n.note == o.note)
            else {
                return true;
            };
            let n = news.remove(at);
            if n.count != o.count {
                changes.push(Change::Count {
                    name: name.clone(),
                    from: o.count,
                    to: n.count,
                    print: n.print,
                });
            }
            false
        });
        // What is left pairs best match first, printing before finish
        // before language before note.
        while !olds.is_empty() && !news.is_empty() {
            let mut best = (0, 0, 0u8);
            for (i, o) in olds.iter().enumerate() {
                for (j, n) in news.iter().enumerate() {
                    let score = likeness(o, n);
                    if (i, j) == (0, 0) || score > best.2 {
                        best = (i, j, score);
                    }
                }
            }
            let o = olds.remove(best.0);
            let n = news.remove(best.1);
            pair(&name, &o, &n, &mut changes);
        }
        for o in olds {
            changes.push(Change::Removed {
                name: name.clone(),
                count: o.count,
                print: o.print,
            });
        }
        for n in news {
            changes.push(Change::Added {
                name: name.clone(),
                count: n.count,
                print: n.print,
            });
        }
    }
    ZoneDiff { changes }
}

/// The delta of a whole deck: main deck, sideboard and commanders summed.
#[must_use]
pub fn deck_delta(before: [&[String]; 3], after: [&[String]; 3]) -> Delta {
    let mut delta = Delta::default();
    for (b, a) in before.into_iter().zip(after) {
        delta.add(diff_zone(b, a).delta());
    }
    delta
}

/// How many cards the rows hold: the sum of their counts, not the number of
/// rows (`4 Lightning Bolt` is four).
#[must_use]
pub fn card_count(rows: &[String]) -> u32 {
    rows.iter().map(|r| read(r).count).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|l| (*l).to_string()).collect()
    }

    fn diff(before: &[&str], after: &[&str]) -> Vec<Change> {
        diff_zone(&rows(before), &rows(after)).changes
    }

    fn print(row: &str) -> PrintChoice {
        deckrow::parse(row).expect("fixture parses").print
    }

    #[test]
    fn an_identical_zone_has_no_changes() {
        let zone = ["4 Lightning Bolt (M11) 149", "1 Sol Ring # ramp"];
        assert!(diff(&zone, &zone).is_empty());
    }

    #[test]
    fn a_new_name_is_added_and_a_cut_one_removed() {
        assert_eq!(
            diff(&["2 Damnation"], &["2 Wrath of God"]),
            vec![
                Change::Removed {
                    name: "Damnation".into(),
                    count: 2,
                    print: PrintChoice::default()
                },
                Change::Added {
                    name: "Wrath of God".into(),
                    count: 2,
                    print: PrintChoice::default()
                },
            ]
        );
    }

    #[test]
    fn the_same_printing_with_another_count_is_one_count_row() {
        let changes = diff(
            &["4 Lightning Bolt (M11) 149"],
            &["3 Lightning Bolt (M11) 149"],
        );
        assert_eq!(
            changes,
            vec![Change::Count {
                name: "Lightning Bolt".into(),
                from: 4,
                to: 3,
                print: print("1 Lightning Bolt (M11) 149"),
            }]
        );
        let delta = ZoneDiff { changes }.delta();
        assert_eq!((delta.removed, delta.added, delta.count), (1, 0, 1));
    }

    #[test]
    fn a_foil_change_is_one_finish_row_never_a_removal_and_an_addition() {
        let changes = diff(&["1 Force of Will"], &["1 Force of Will *F*"]);
        assert_eq!(
            changes,
            vec![Change::Finish {
                name: "Force of Will".into(),
                count: 1,
                from: Finish::Normal,
                to: Finish::Foil,
                print: print("1 Force of Will *F*"),
            }]
        );
        let delta = ZoneDiff { changes }.delta();
        assert_eq!(
            delta,
            Delta {
                finish: 1,
                ..Delta::default()
            }
        );
    }

    #[test]
    fn another_printing_is_one_printing_row() {
        assert_eq!(
            diff(
                &["1 Snapcaster Mage (M11) 149"],
                &["1 Snapcaster Mage (MH2) 251"]
            ),
            vec![Change::Printing {
                name: "Snapcaster Mage".into(),
                count: 1,
                from: print("1 X (M11) 149"),
                to: print("1 X (MH2) 251"),
            }]
        );
    }

    #[test]
    fn another_language_is_one_language_row() {
        assert_eq!(
            diff(&["1 Gitaxian Probe"], &["1 Gitaxian Probe [ja]"]),
            vec![Change::Language {
                name: "Gitaxian Probe".into(),
                count: 1,
                from: "en".into(),
                to: "ja".into(),
                print: print("1 Gitaxian Probe [ja]"),
            }]
        );
    }

    #[test]
    fn another_note_is_one_note_row() {
        assert_eq!(
            diff(&["1 Sol Ring # ramp"], &["1 Sol Ring # fast mana"]),
            vec![Change::Note {
                name: "Sol Ring".into(),
                count: 1,
                print: PrintChoice::default(),
            }]
        );
    }

    #[test]
    fn a_spelled_out_default_is_no_change() {
        assert!(diff(&["1 Sol Ring"], &["1 Sol Ring [en]"]).is_empty());
    }

    #[test]
    fn a_finish_change_with_a_count_change_says_both() {
        let changes = diff(&["2 Force of Will"], &["3 Force of Will *F*"]);
        let delta = ZoneDiff { changes }.delta();
        assert_eq!(
            delta,
            Delta {
                added: 1,
                finish: 1,
                ..Delta::default()
            }
        );
    }

    #[test]
    fn two_printings_of_one_name_are_matched_printing_by_printing() {
        // One copy of each printing; only the M11 one goes foil.
        let changes = diff(
            &["1 Lightning Bolt (M11) 149", "1 Lightning Bolt (2XM) 123"],
            &[
                "1 Lightning Bolt (2XM) 123",
                "1 Lightning Bolt (M11) 149 *F*",
            ],
        );
        assert_eq!(
            changes,
            vec![Change::Finish {
                name: "Lightning Bolt".into(),
                count: 1,
                from: Finish::Normal,
                to: Finish::Foil,
                print: print("1 Lightning Bolt (M11) 149 *F*"),
            }]
        );
        // Both go foil: two finish rows, each on its own printing.
        let changes = diff(
            &["1 Lightning Bolt (M11) 149", "1 Lightning Bolt (2XM) 123"],
            &[
                "1 Lightning Bolt (2XM) 123 *F*",
                "1 Lightning Bolt (M11) 149 *F*",
            ],
        );
        assert_eq!(ZoneDiff { changes }.delta().finish, 2);
    }

    #[test]
    fn a_deck_delta_sums_its_zones() {
        let main_before = rows(&["4 Lightning Bolt", "2 Damnation"]);
        let main_after = rows(&["4 Lightning Bolt *F*", "3 Wrath of God"]);
        let side_before = rows(&["1 Duress"]);
        let side_after = rows(&["2 Duress"]);
        let none: Vec<String> = Vec::new();
        let delta = deck_delta(
            [&main_before, &side_before, &none],
            [&main_after, &side_after, &none],
        );
        assert_eq!(
            delta,
            Delta {
                added: 4,
                removed: 2,
                count: 1,
                finish: 1,
                ..Delta::default()
            }
        );
    }

    #[test]
    fn card_count_sums_counts_not_rows() {
        assert_eq!(card_count(&rows(&["4 Lightning Bolt", "1 Sol Ring"])), 5);
        assert_eq!(card_count(&[]), 0);
    }

    #[test]
    fn an_unreadable_row_stands_as_itself() {
        assert_eq!(
            diff(&[], &["Lightning Bolt"]),
            vec![Change::Added {
                name: "Lightning Bolt".into(),
                count: 1,
                print: PrintChoice::default(),
            }]
        );
    }
}
