//! Where a printed cost ends.

use crate::text::{cost_colon, is_submultiset, symbols};

/// A sentence cut at its cost colon.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Split<'a> {
    /// What the ability costs, in the card's own words and symbols.
    pub head: &'a str,
    /// What it does.
    pub body: &'a str,
}

/// Cuts an activated ability's sentence into its cost and its effect.
///
/// At the first `:` or `：` before any reminder or quotation, trimmed on both
/// sides. `None` for a sentence with no such colon, which is drawn whole: a
/// keyword line (`Ausrüsten {1}`, whose only colon is in its reminder), a
/// chapter that grants an ability in quotes, or a sentence written around
/// its cost instead of in front of it.
///
/// There is no length limit on the cost. Gemstone Mine's `{T}, entferne eine
/// Minenmarke von der Edelsteinmine:` is fifty bytes of cost, and the 48-byte
/// cap this replaces drew it as an effect.
#[must_use]
pub fn split_cost(sentence: &str) -> Option<Split<'_>> {
    let (at, width) = cost_colon(sentence)?;
    let head = sentence[..at].trim_end();
    let body = sentence[at + width..].trim_start();
    (!head.is_empty()).then_some(Split { head, body })
}

/// Whether a printed cost may stand in the cost column of an ability that
/// costs `cost`.
///
/// `cost` is the ability's own cost as symbols in Scryfall's notation — its
/// mana cost followed by `{T}` or `{Q}` when it taps or untaps — and the
/// printed head is licensed when every symbol it prints is one of those.
/// Equal is the ordinary case; fewer is an old printing that braced part of
/// it; none is a cost printed in words (`Sacrifice a Forest`, `+2`). A head
/// printing a symbol the ability does not cost is some other line, and the
/// caller draws the sentence whole rather than put a wrong cost in the
/// column.
#[must_use]
pub fn licensed(head: &str, cost: &str) -> bool {
    is_submultiset(&symbols(head), &symbols(cost))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sentence_is_cut_at_its_cost_colon() {
        let got =
            split_cost("{T}, entferne eine Minenmarke von der Edelsteinmine: Erzeuge ein Mana.")
                .unwrap();
        assert_eq!(
            got.head,
            "{T}, entferne eine Minenmarke von der Edelsteinmine"
        );
        assert_eq!(got.body, "Erzeuge ein Mana.");
        let got = split_cost("{W}, {T} : Engagez la créature ciblée.").unwrap();
        assert_eq!(
            (got.head, got.body),
            ("{W}, {T}", "Engagez la créature ciblée.")
        );
        let got = split_cost("+2：プレイヤー１人を対象とする。").unwrap();
        assert_eq!((got.head, got.body), ("+2", "プレイヤー１人を対象とする。"));
    }

    #[test]
    fn a_sentence_with_no_cost_colon_is_drawn_whole() {
        assert_eq!(
            split_cost("Ausrüsten {1} ({1}: Lege diese Karte an.)"),
            None
        );
        assert_eq!(
            split_cost("I — Urzas Sage erhält „{T}: Erzeuge {C}.\""),
            None
        );
        assert_eq!(split_cost("Fliegend"), None);
    }

    /// Both branches: a head the cost accounts for, and one it does not.
    #[test]
    fn a_head_is_licensed_by_the_symbols_the_ability_costs() {
        assert!(licensed("{1}, {T}, opfere dieses Artefakt", "{1}{T}"));
        assert!(licensed("6, {T}", "{6}{T}"));
        assert!(licensed("Opfere einen Wald", ""));
        assert!(licensed("{2BB}, {T}", "{2}{B}{B}{T}"));
        assert!(!licensed("{R}, {T}", "{W}{T}"));
        assert!(!licensed("{T}", ""));
    }
}
