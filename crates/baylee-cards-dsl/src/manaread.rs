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
    let colors = match source {
        ManaSource::Fixed(color) => vec![*color],
        ManaSource::Choice(colors) => colors.to_vec(),
        // Both depend on the rest of the board — a commander's identity, or
        // what someone else's lands can make. The engine knows; this does not.
        ManaSource::CommanderIdentity | ManaSource::LandColor { .. } => return None,
    };
    Some((
        SimpleMana {
            colors,
            amount: u8::try_from(*amount).unwrap_or(u8::MAX),
        },
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
    }
}
