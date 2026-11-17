//! Which printing speaks for a card in a language.

use std::cmp::Ordering;

use crate::align::{Aligned, align, untranslated};
use crate::verify::{Verdict, verify};

/// One printing of a card in the language being asked for.
///
/// The catalog builds this from its rows and a client from Scryfall's
/// answer; both carry the same fields because both are Scryfall's.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Printing {
    /// Scryfall's printing id.
    pub scryfall_id: String,
    /// `YYYY-MM-DD`, or empty when Scryfall has no date for it.
    pub released_at: String,
    /// As printed: `263`, `263a`, `MOC-379`.
    pub collector_number: String,
    /// Scryfall's `layout` — `modal_dfc` is the one the rule reads.
    pub layout: String,
    /// Each face's printed text, in face order; `None` where Scryfall has
    /// none (`printed_text` is NULL).
    pub printed: Vec<Option<String>>,
}

/// The printing chosen for a card, with each face placed against the Oracle.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Layer<'p> {
    /// The printing whose text is drawn.
    pub printing: &'p Printing,
    /// For each Oracle face, its lines — or `None` for a face this printing
    /// did not translate or that does not line up, which is drawn from the
    /// Oracle.
    pub faces: Vec<Option<Aligned>>,
}

/// Picks the printing whose text a player reading `lang` is shown.
///
/// `oracle` is the English Oracle text of each face, in face order. The
/// answer is per **card**, with every face at once, so a double-faced card is
/// never drawn from two printings.
///
/// 1. A printing that translated none of the faces ([`untranslated`]) is not
///    a candidate: English is never served as another language.
/// 2. One whose every face is translated and lines up ([`align`]) and whose
///    placed lines [`verify`] refuses none of comes first; then one whose
///    every face lines up; then the rest. Spawning Pool's newest German
///    printing (10e) glues two lines together; its older one (ulg) lines
///    up, and an older wording is still the card's own German where the
///    newer one is a blank. A newer printing whose activated line lost its
///    cost (old prose) is passed over the same way for one that keeps it.
/// 3. Then the newest.
/// 4. Then the lowest collector number, which is the regular frame — a
///    borderless or showcase variant that drops its reminder line is
///    numbered after it — then the collector number as written, then the
///    printing id. 268 cards of this pool tie on the first three rules in
///    German (measured 2026-09-24), and a rule that leaves a tie to a
///    database's row order is not a rule.
///
/// `None` under `en`, where the Oracle is what is drawn — an English
/// printing's printed text is a promo's wording (`TAP: ADD G`, Secret Lair's
/// `Equip 1`) and never the card's — and `None` when no printing
/// translated anything.
#[must_use]
pub fn pick<'p>(lang: &str, oracle: &[&str], printings: &'p [Printing]) -> Option<Layer<'p>> {
    if lang == "en" {
        return None;
    }
    printings
        .iter()
        .filter_map(|printing| {
            let faces: Vec<Option<Aligned>> = oracle
                .iter()
                .enumerate()
                .map(|(face, oracle)| {
                    let printed = printing.printed.get(face).and_then(Option::as_deref);
                    if untranslated(oracle, printed) {
                        return None;
                    }
                    align(oracle, printed?, &printing.layout)
                })
                .collect();
            let translated = oracle.iter().enumerate().any(|(face, oracle)| {
                !untranslated(
                    oracle,
                    printing.printed.get(face).and_then(Option::as_deref),
                )
            });
            translated.then_some(Layer { printing, faces })
        })
        .map(|layer| (verified(oracle, &layer), layer))
        .max_by(|(va, a), (vb, b)| va.cmp(vb).then_with(|| order(a, b)))
        .map(|(_, layer)| layer)
}

/// Rule 2's first half: every face lines up, and [`verify`] refuses none of
/// the lines placed. A line set aside as reminder-only has nothing placed
/// and is not counted, so a borderless frame that drops a reminder is not
/// held below the regular one for it.
fn verified(oracle: &[&str], layer: &Layer<'_>) -> bool {
    layer.faces.len() == oracle.len()
        && layer.faces.iter().zip(oracle).all(|(face, oracle)| {
            face.as_ref().is_some_and(|aligned| {
                aligned.lines.iter().enumerate().all(|(line, placed)| {
                    placed.is_none() || verify(oracle, aligned, line) != Verdict::Refused
                })
            })
        })
}

/// Rules 2–4 of [`pick`] after [`verified`], as "greater is better".
fn order(a: &Layer<'_>, b: &Layer<'_>) -> Ordering {
    let complete = |l: &Layer<'_>| l.faces.iter().all(Option::is_some);
    let (pa, pb) = (a.printing, b.printing);
    complete(a)
        .cmp(&complete(b))
        .then_with(|| pa.released_at.cmp(&pb.released_at))
        .then_with(|| number(&pb.collector_number).cmp(&number(&pa.collector_number)))
        .then_with(|| pb.collector_number.cmp(&pa.collector_number))
        .then_with(|| pb.scryfall_id.cmp(&pa.scryfall_id))
}

/// The number a collector number starts with; one with none sorts last.
fn number(collector_number: &str) -> u32 {
    let digits = collector_number.len()
        - collector_number
            .trim_start_matches(|c: char| c.is_ascii_digit())
            .len();
    collector_number[..digits].parse().unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn printing(id: &str, date: &str, number: &str, text: Option<&str>) -> Printing {
        Printing {
            scryfall_id: id.to_owned(),
            released_at: date.to_owned(),
            collector_number: number.to_owned(),
            layout: "normal".to_owned(),
            printed: vec![text.map(str::to_owned)],
        }
    }

    const ORACLE: [&str; 1] = ["{T}: Add {C}.\n{1}, {T}, Sacrifice this artifact: Draw a card."];

    fn picked(printings: &[Printing]) -> Option<&str> {
        pick("de", &ORACLE, printings).map(|l| l.printing.scryfall_id.as_str())
    }

    #[test]
    fn the_newest_translated_printing_speaks_for_the_card() {
        let ps = [
            printing(
                "old",
                "2007-07-13",
                "335",
                Some("{T}: Erhöhe um {1}.\n{1}, {T}, opfere: Ziehe."),
            ),
            printing(
                "new",
                "2025-06-13",
                "353",
                Some("{T}: Erzeuge {C}.\n{1}, {T}, opfere: Ziehe."),
            ),
        ];
        assert_eq!(picked(&ps), Some("new"));
    }

    /// Both branches of rule 1: English under another language's name, and
    /// no text at all, are never picked — even when newest.
    #[test]
    fn an_untranslated_printing_is_never_picked() {
        let ps = [
            printing("english", "2026-01-01", "1", Some(ORACLE[0])),
            printing("null", "2026-01-01", "2", None),
            printing(
                "german",
                "2020-01-01",
                "3",
                Some("{T}: Erzeuge {C}.\n{1}, {T}, opfere: Ziehe."),
            ),
        ];
        assert_eq!(picked(&ps), Some("german"));
        assert_eq!(picked(&ps[..2]), None);
    }

    #[test]
    fn a_printing_that_lines_up_beats_a_newer_one_that_does_not() {
        let ps = [
            printing(
                "glued",
                "2015-11-13",
                "259",
                Some("{T}: Erhöhe um {1}.{1}, {T}, opfere: Ziehe."),
            ),
            printing(
                "level",
                "2014-11-07",
                "250",
                Some("{T}: Erhöhe um {1}.\n{1}, {T}, opfere: Ziehe."),
            ),
        ];
        assert_eq!(picked(&ps), Some("level"));
        let layer = pick("de", &ORACLE, &ps[..1]).unwrap();
        assert_eq!(
            layer.faces,
            [None],
            "a face that does not line up is drawn from the Oracle"
        );
    }

    /// Lining up is not enough where a line in place is not the ability: a
    /// newer printing whose second line lost its cost is passed over for an
    /// older one whose lines verify, and still beats one that does not line
    /// up at all.
    #[test]
    fn a_printing_whose_lines_verify_beats_a_newer_one_that_only_lines_up() {
        let ps = [
            printing(
                "prose",
                "2016-01-01",
                "1",
                Some("{T}: Erzeuge {C}.\nOpfere den Stein und ziehe eine Karte."),
            ),
            printing(
                "costed",
                "2010-01-01",
                "2",
                Some("{T}: Erzeuge {C}.\n{1}, {T}, opfere: Ziehe."),
            ),
            printing(
                "glued",
                "2020-01-01",
                "3",
                Some("{T}: Erzeuge {C}.{1}, {T}, opfere: Ziehe."),
            ),
        ];
        assert_eq!(picked(&ps), Some("costed"));
        assert_eq!(picked(&[ps[0].clone(), ps[2].clone()]), Some("prose"));
    }

    #[test]
    fn a_tie_goes_to_the_lowest_collector_number_then_the_id() {
        let text = Some("{T}: Erzeuge {C}.\n{1}, {T}, opfere: Ziehe.");
        let ps = [
            printing("b", "2024-02-09", "327", text),
            printing("c", "2024-02-09", "263", text),
            printing("a", "2024-02-09", "263", text),
        ];
        assert_eq!(picked(&ps), Some("a"));
        let reversed: Vec<Printing> = ps.iter().rev().cloned().collect();
        assert_eq!(
            picked(&reversed),
            Some("a"),
            "the answer does not depend on row order"
        );
    }

    #[test]
    fn english_is_answered_by_the_oracle() {
        let ps = [printing("slz", "2026-09-02", "113", Some("Equip 1"))];
        assert_eq!(pick("en", &["Equip {1}"], &ps), None);
    }

    #[test]
    fn a_collector_number_is_ordered_by_the_number_it_starts_with() {
        assert_eq!(number("263"), 263);
        assert_eq!(number("263a"), 263);
        assert_eq!(number("MOC-379"), u32::MAX);
        assert_eq!(number(""), u32::MAX);
    }
}
