//! Reading an ability as "n mana, of one of these colours".
//!
//! Two readers need this answer and must give the same one. A client's mana
//! planner asks it of a printed `{T}: Add {G}` so it knows what tapping a
//! Forest buys; `baylee-gamehost` asks it of the ability a Chromatic Lantern
//! *grants* a land, because the client cannot — a granted ability is not
//! printed on the card, so there is nothing in the registry to look up. Two
//! copies of the rule would be two answers, and the one that disagreed would
//! be a land the planner counts on and the engine refuses.
//!
//! The bar is deliberately high, and the reasons differ per clause. An
//! ability that costs mana to activate would make a plan recursive. An
//! ability that also does something else is one a player should decide about
//! themselves. Restricted mana is refused because what a Cavern of Souls'
//! mana may be spent on is a rules question, and answering it outside the
//! engine is exactly the guess this exists to avoid.

use crate::cost::Cost;
use crate::effect::{Amount, Effect, ManaSource};
use baylee_core::mana::{ManaColor, ManaCost};

/// What a simple mana ability makes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimpleMana {
    /// The colours it may make. More than one means the ability asks.
    pub colors: Vec<ManaColor>,
    /// How much, of whichever colour is chosen.
    pub amount: u8,
}

/// Reads a free, single-effect mana ability, or decides it is not one a
/// planner can use.
#[must_use]
pub fn simple_mana(cost: &Cost, effects: &[Effect]) -> Option<SimpleMana> {
    match mana_made(cost, effects) {
        Some((mana, false)) => Some(mana),
        // Restricted, and this is the caller that must refuse it — see the
        // module header, and [`mana_made`] for the caller that must not.
        _ => None,
    }
}

/// The same reading, restricted mana included, saying which it found.
///
/// The filter [`simple_mana`] applies on top of this is the entire difference
/// between two questions that look alike. A *planner* has to refuse restricted
/// mana: what a Cavern of Souls' mana may pay for is a rules question, and
/// answering it outside the engine is the guess this module exists to avoid.
/// A *label* must not refuse it — the button still has to say what tapping the
/// land does, and Jasmine Dragon Tea Shop shipped with its Ally ability drawn
/// as a bare "{T}" beside a "Tap for {C}", which is two offers a player cannot
/// tell apart and one of them is the reason the land is in the deck.
#[must_use]
pub fn mana_made(cost: &Cost, effects: &[Effect]) -> Option<(SimpleMana, bool)> {
    let (source, amount, restricted) = mana_shape(cost, effects)?;
    let colors = match source {
        ManaSource::Fixed(color) => vec![color],
        ManaSource::Choice(colors) => colors.to_vec(),
        // Both depend on the rest of the board — a commander's identity, or
        // what someone else's lands can make — so neither has an answer
        // here, where there is no board to read. A caller that *has* one
        // takes [`mana_shape`] and resolves them itself.
        ManaSource::CommanderIdentity | ManaSource::LandColor { .. } => return None,
    };
    Some((SimpleMana { colors, amount }, restricted))
}

/// The same reading one step earlier: the ability's own words, before any
/// board is consulted.
///
/// [`mana_made`] answers "which colours", and for two of the four sources
/// there is no answer without a game — a Command Tower's colours are its
/// controller's commanders' identity (CR 903.4), an Exotic Orchard's are
/// read off somebody else's lands. Both used to end the reading, so a client
/// holding a `PlayerView` — which carries a seat's commanders, and is the
/// same thing the engine reads — had no way to ask the question it *could*
/// answer. It got no source at all, and a five-colour deck's Command Tower
/// counted for nothing in the mana plan.
///
/// This is the reading it needs, and the split is the same one the module
/// header describes: the shape is the card's, the resolution is the board's,
/// and the two are not the same job. Everything that makes the bar high —
/// a free cost, one effect, a fixed amount — is still enforced here, so a
/// caller resolving the source itself cannot slip past any of it.
#[must_use]
pub fn mana_shape(cost: &Cost, effects: &[Effect]) -> Option<(ManaSource, u8, bool)> {
    if cost.mana != ManaCost::ZERO {
        return None;
    }
    let [
        Effect::AddMana {
            source,
            amount: Amount::Fixed(amount),
            restriction,
            ..
        },
    ] = effects
    else {
        return None;
    };
    Some((
        *source,
        u8::try_from(*amount).unwrap_or(u8::MAX),
        restriction.is_some(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cost::CostPart;
    use crate::effect::{ManaRestriction, SpendRider};
    use crate::filter::Filter;

    fn tap() -> Cost {
        Cost {
            mana: ManaCost::ZERO,
            parts: &[CostPart::TapSelf],
        }
    }

    #[test]
    fn a_plain_tap_for_one_colour_reads_the_same_through_both_doors() {
        let effects = [Effect::mana(ManaColor::Green, 1)];
        let expected = SimpleMana {
            colors: vec![ManaColor::Green],
            amount: 1,
        };
        assert_eq!(simple_mana(&tap(), &effects), Some(expected.clone()));
        assert_eq!(mana_made(&tap(), &effects), Some((expected, false)));
    }

    /// Jasmine Dragon Tea Shop's second ability, and the whole reason the two
    /// readings exist: the planner must refuse it, the button must not.
    #[test]
    fn restricted_mana_is_refused_by_the_planner_and_read_by_the_label() {
        static ALLY: Filter = Filter::CREATURE;
        let effects = [Effect::AddMana {
            source: crate::effect::ManaSource::Choice(&[
                ManaColor::White,
                ManaColor::Blue,
                ManaColor::Black,
                ManaColor::Red,
                ManaColor::Green,
            ]),
            amount: Amount::Fixed(1),
            combination: false,
            restriction: Some(ManaRestriction {
                filter: &ALLY,
                rider: SpendRider::None,
            }),
        }];
        assert_eq!(
            simple_mana(&tap(), &effects),
            None,
            "what restricted mana may pay for is the engine's question"
        );
        let (mana, restricted) = mana_made(&tap(), &effects).expect("the label can read it");
        assert!(restricted);
        assert_eq!(mana.colors.len(), 5, "five colours to choose between");
        assert_eq!(mana.amount, 1);
    }

    /// The bar stays high for everything else: an ability that costs mana
    /// would make a plan recursive, whichever door it is read through.
    #[test]
    fn an_ability_that_costs_mana_is_read_by_neither() {
        let cost = Cost {
            mana: baylee_core::mana!("{1}"),
            parts: &[CostPart::TapSelf],
        };
        let effects = [Effect::mana(ManaColor::Blue, 1)];
        assert_eq!(simple_mana(&cost, &effects), None);
        assert_eq!(mana_made(&cost, &effects), None);
        assert_eq!(mana_shape(&cost, &effects), None, "the bar is in the shape");
    }

    /// Command Tower: the two colour-answering doors have nothing to say,
    /// and the third hands the source back so a caller with a board can.
    #[test]
    fn a_commanders_identity_is_a_shape_without_being_a_colour() {
        let effects = [Effect::mana_commander_identity()];
        assert_eq!(simple_mana(&tap(), &effects), None);
        assert_eq!(mana_made(&tap(), &effects), None);
        assert_eq!(
            mana_shape(&tap(), &effects),
            Some((ManaSource::CommanderIdentity, 1, false)),
            "one unrestricted mana, of colours only a board can name"
        );
    }

    /// And the shape is not a way around the bar: Exotic Orchard's source
    /// comes back too, so a caller that cannot resolve it has to refuse it
    /// rather than never being told it was there.
    #[test]
    fn the_shape_names_the_source_it_cannot_answer_for() {
        let effects = [Effect::AddMana {
            source: ManaSource::LandColor { mine: false },
            amount: Amount::Fixed(1),
            combination: false,
            restriction: None,
        }];
        assert_eq!(
            mana_shape(&tap(), &effects),
            Some((ManaSource::LandColor { mine: false }, 1, false))
        );
    }
}
