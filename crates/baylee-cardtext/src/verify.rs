//! Whether the line in an Oracle line's position is the same ability.

use crate::align::Aligned;
use crate::text::{cost_colon, head, is_submultiset, sentences, symbols};

/// What the cost symbols say about a line [`align`](crate::align) placed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Verdict {
    /// The cost prints the Oracle's symbols, or the Oracle's cost prints
    /// none and there is nothing to compare.
    Positional,
    /// The printing's cost carries some of the Oracle's symbols and nothing
    /// else: Obelisk of Undoing's 5ed `6, {T}` for `{6}, {T}`, an old
    /// printing that braced only the tap.
    Subset,
    /// The printing swapped two abilities, and the Oracle's cost is printed
    /// on exactly one other line — that line, by its Oracle index. Granger
    /// Guildmage (mir) and Thornscape Apprentice (inv) print their `{W}` and
    /// `{R}` abilities in the other order.
    Permuted(usize),
    /// Nothing identifies the line: the row falls to the Oracle sentence.
    Refused,
}

/// Checks the printed line placed at Oracle line `line` by its cost.
///
/// The key is the multiset of symbols in each line's **cost part** — up to
/// its cost colon, or its lead when it has none — and never in the whole
/// line. The effect side is free to differ, and it does: an old printing
/// says `Erhöhe deinen Manavorrat um {1}` where the Oracle says `Add {C}`,
/// and Urza's Mine's German prints symbols in its reminder that the English
/// does not. Keyed over whole lines the same rule refused 67 German rows of
/// this pool; keyed over the cost part, 9 — and the brace repair rescues 8
/// of those (measured 2026-09-24).
///
/// A line with no symbols in its Oracle cost (a keyword, a trigger, a
/// loyalty ability) cannot be checked this way and is taken where it stands.
#[must_use]
pub fn verify(oracle: &str, aligned: &Aligned, line: usize) -> Verdict {
    let want: Vec<&str> = sentences(oracle).collect();
    let (Some(target), Some(Some(placed))) = (want.get(line), aligned.lines.get(line)) else {
        return Verdict::Refused;
    };
    let expected = symbols(head(target));
    let printed = symbols(head(placed));
    if expected.is_empty() || expected == printed {
        return Verdict::Positional;
    }
    // An old prose description without an activation cost is not evidence
    // for a current activated ability, even though its empty symbol set is
    // mathematically a subset. Leave it intact and let the caller use Oracle.
    let missing_activation = cost_colon(target).is_some() && cost_colon(placed).is_none();
    if !missing_activation && is_submultiset(&printed, &expected) {
        return Verdict::Subset;
    }
    let unique_in_oracle = want.iter().filter(|l| symbols(head(l)) == expected).count() == 1;
    if unique_in_oracle {
        let mut hits = aligned
            .lines
            .iter()
            .enumerate()
            .filter(|(_, l)| l.as_deref().is_some_and(|l| symbols(head(l)) == expected));
        if let (Some((at, _)), None) = (hits.next(), hits.next()) {
            return Verdict::Permuted(at);
        }
    }
    Verdict::Refused
}

/// The printed sentence that stands for Oracle line `line`, if one does.
///
/// [`verify`] applied: the line in position, the line it was swapped with,
/// or nothing — and nothing means the caller draws the Oracle sentence.
#[must_use]
pub fn localized<'a>(oracle: &str, aligned: &'a Aligned, line: usize) -> Option<&'a str> {
    let at = match verify(oracle, aligned, line) {
        Verdict::Positional | Verdict::Subset => line,
        Verdict::Permuted(at) => at,
        Verdict::Refused => return None,
    };
    aligned.lines.get(at)?.as_deref()
}

/// One face's sentence in the player's language for each Oracle line, from
/// several printings of it in that language: the owner's rule, "the
/// client's language first, English as the fallback" (17.11.2026).
///
/// `printings` are the face as each printing has it, placed by
/// [`align`](crate::align), in order of preference: the served printing,
/// then the others newest first. A face is read from **one printing where
/// one suffices**, the first whose every line [`localized`] keeps, so a
/// face never mixes two wordings it does not have to. Only when none of
/// them yields every line is each line taken from the first that yields
/// it. A `None` is a line no printing yields, which the caller draws from
/// the English Oracle.
#[must_use]
pub fn localized_lines(oracle: &str, printings: &[Aligned]) -> Vec<Option<String>> {
    let count = sentences(oracle).count();
    let said = |aligned: &Aligned, line| localized(oracle, aligned, line).map(str::to_owned);
    if let Some(whole) = printings
        .iter()
        .find(|aligned| (0..count).all(|line| localized(oracle, aligned, line).is_some()))
    {
        return (0..count).map(|line| said(whole, line)).collect();
    }
    (0..count)
        .map(|line| printings.iter().find_map(|aligned| said(aligned, line)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::align::align;

    const GUILDMAGE: &str = "{R}, {T}: This creature deals 1 damage to any target and 1 damage to you.\n\
                             {W}, {T}: Target creature gains first strike until end of turn.";

    #[test]
    fn a_line_printing_the_oracle_cost_is_where_it_stands() {
        let a = align(
            "{1}, {T}, Sacrifice this: Draw.",
            "{1}, {T}, opfere dies: Ziehe.",
            "normal",
        )
        .unwrap();
        assert_eq!(
            verify("{1}, {T}, Sacrifice this: Draw.", &a, 0),
            Verdict::Positional
        );
    }

    #[test]
    fn a_repaired_brace_is_the_same_cost() {
        let oracle = "{2}{B}{B}, {T}: Put counters.";
        let a = align(oracle, "{2BB}, {T}: Lege Marken.", "normal").unwrap();
        assert_eq!(verify(oracle, &a, 0), Verdict::Positional);
    }

    /// Both branches of the subset rule: part of the cost is an old
    /// printing; a symbol the Oracle does not print is not.
    #[test]
    fn part_of_the_cost_is_an_old_printing_and_a_foreign_symbol_is_not() {
        let oracle = "{6}, {T}: Return target permanent.";
        let a = align(oracle, "6, {T}: Bringe zurück.", "normal").unwrap();
        assert_eq!(verify(oracle, &a, 0), Verdict::Subset);
        let a = align(oracle, "{G}, {T}: Bringe zurück.", "normal").unwrap();
        assert_eq!(verify(oracle, &a, 0), Verdict::Refused);
    }

    #[test]
    fn historical_prose_without_the_activation_cost_uses_oracle() {
        let oracle = "{0}: The next 1 damage is dealt to its owner instead.";
        let historical =
            "Der Besitzer kann Teile oder den ganzen Schaden auf sich selber umleiten.";
        let aligned = align(oracle, historical, "normal").unwrap();
        assert_eq!(verify(oracle, &aligned, 0), Verdict::Refused);
        assert_eq!(localized(oracle, &aligned, 0), None);
        assert_eq!(aligned.lines[0].as_deref(), Some(historical));
        let current = align(
            oracle,
            "{0}: Der nächste 1 Schaden wird umgeleitet.",
            "normal",
        )
        .unwrap();
        assert_eq!(verify(oracle, &current, 0), Verdict::Positional);
    }

    #[test]
    fn two_swapped_abilities_are_found_by_their_costs() {
        let printed = "{W}, {T}: Erstschlag.\n{R}, {T}: 1 Schaden.";
        let a = align(GUILDMAGE, printed, "normal").unwrap();
        assert_eq!(verify(GUILDMAGE, &a, 0), Verdict::Permuted(1));
        assert_eq!(verify(GUILDMAGE, &a, 1), Verdict::Permuted(0));
        assert_eq!(localized(GUILDMAGE, &a, 0), Some("{R}, {T}: 1 Schaden."));
    }

    /// The other branch: a cost printed on two lines identifies neither.
    #[test]
    fn a_cost_printed_twice_permutes_nothing() {
        let printed = "{W}, {T}: Erstschlag.\n{W}, {T}: Noch einmal.";
        let a = align(GUILDMAGE, printed, "normal").unwrap();
        assert_eq!(verify(GUILDMAGE, &a, 0), Verdict::Refused);
        assert_eq!(localized(GUILDMAGE, &a, 0), None);
    }

    #[test]
    fn a_line_with_no_oracle_symbols_is_taken_where_it_stands() {
        let oracle = "Flying\nVigilance";
        let a = align(oracle, "Wachsamkeit\nFliegend", "normal").unwrap();
        assert_eq!(verify(oracle, &a, 0), Verdict::Positional);
    }

    const STONE: &str = "{T}: Add {C}.\n{1}, {T}, Sacrifice this artifact: Draw a card.";

    fn placed(printed: &str) -> Aligned {
        align(STONE, printed, "normal").expect("two lines")
    }

    fn owned(lines: &[Option<&str>]) -> Vec<Option<String>> {
        lines.iter().map(|l| l.map(str::to_owned)).collect()
    }

    /// A printing whose every line stands is read whole, even behind one
    /// that yields some of the lines: the face does not mix two wordings.
    #[test]
    fn one_printing_that_covers_the_face_is_read_whole() {
        let partial = placed("{T}: Erzeuge {C}.\n{G}: Ziehe.");
        let whole = placed("{T}: Erhöhe um {1}.\n{1}, {T}, opfere: Ziehe.");
        let newer = placed("{T}: Erzeuge {C}.\n{1}, {T}, opfere dies: Ziehe.");
        assert_eq!(
            localized_lines(STONE, &[partial.clone(), whole.clone(), newer.clone()]),
            owned(&[
                Some("{T}: Erhöhe um {1}."),
                Some("{1}, {T}, opfere: Ziehe.")
            ])
        );
        assert_eq!(
            localized_lines(STONE, &[newer, whole]),
            owned(&[
                Some("{T}: Erzeuge {C}."),
                Some("{1}, {T}, opfere dies: Ziehe.")
            ]),
            "and of two that cover it, the first"
        );
    }

    /// With no printing covering the face, each line comes from the first
    /// that yields it, a refused line falls to the next printing, and a line
    /// none yields is left to the English Oracle.
    #[test]
    fn otherwise_each_line_comes_from_the_first_printing_that_yields_it() {
        let first = placed("{T}: Erzeuge {C}.\n{G}: Ziehe.");
        let second = placed("{G}: Erzeuge.\n{1}, {T}, opfere: Ziehe.");
        assert_eq!(verify(STONE, &first, 1), Verdict::Refused);
        assert_eq!(
            localized_lines(STONE, &[first.clone(), second]),
            owned(&[Some("{T}: Erzeuge {C}."), Some("{1}, {T}, opfere: Ziehe.")])
        );
        let also_refused = placed("{T}: Erhöhe um {1}.\n{B}: Ziehe.");
        assert_eq!(
            localized_lines(STONE, &[first, also_refused]),
            owned(&[Some("{T}: Erzeuge {C}."), None])
        );
        assert_eq!(localized_lines(STONE, &[]), owned(&[None, None]));
    }

    #[test]
    fn an_oracle_reminder_set_aside_has_no_counterpart() {
        let oracle = "({T}: Add {B} or {G}.)\nAs this land enters, pay 2 life.";
        let a = align(
            oracle,
            "Sowie das Land ins Spiel kommt, 2 Lebenspunkte.",
            "normal",
        )
        .unwrap();
        assert_eq!(verify(oracle, &a, 0), Verdict::Refused);
        assert_eq!(verify(oracle, &a, 1), Verdict::Positional);
        assert_eq!(verify(oracle, &a, 7), Verdict::Refused);
    }
}
