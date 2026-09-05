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
    if cost.mana != ManaCost::ZERO {
        return None;
    }
    let [
        Effect::AddMana {
            source,
            amount: Amount::Fixed(amount),
            restriction: None,
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
    Some(SimpleMana {
        colors,
        amount: u8::try_from(*amount).unwrap_or(u8::MAX),
    })
}
