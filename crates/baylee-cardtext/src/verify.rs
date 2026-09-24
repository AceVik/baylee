//! Whether the line in an Oracle line's position is the same ability.

use crate::align::Aligned;
use crate::text::{head, is_submultiset, sentences, symbols};

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
    if is_submultiset(&printed, &expected) {
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
