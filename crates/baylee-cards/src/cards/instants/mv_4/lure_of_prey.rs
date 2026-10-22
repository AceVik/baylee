//! Lure of Prey — {2}{G}{G} — Instant
//! Oracle: Cast this spell only if an opponent cast a creature spell this turn.
//! Oracle: You may put a green creature card from your hand onto the battlefield.
//! Set: MIR #227 — Mirage | Scryfall ID: 7165f1d8-7b31-4a81-aab4-c6cd4ff2e67d | Oracle ID: 3c83ea11-7141-41a2-ba77-1ed2722dfe74
// NOT SUPPORTED: restricting casting to turns in which an opponent cast a creature spell is not in Condition.

use baylee_cards_dsl::prelude::*;

const GREEN_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::Green])),
]);

card!(
    index = index::LURE_OF_PREY,
    oracle_id = "3c83ea11-7141-41a2-ba77-1ed2722dfe74",
    scryfall_id = "7165f1d8-7b31-4a81-aab4-c6cd4ff2e67d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "lets you put a green creature card from your hand onto the battlefield; restricting casting to turns in which an opponent cast a creature spell is not in Condition",
    ),
    faces = &[face!(
        name = "Lure of Prey",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::PutFromHandOntoBattlefield {
        tapped: false,
        filter: &GREEN_CREATURE,
        mana_value: None,
        optional: true,
    }])],
);
