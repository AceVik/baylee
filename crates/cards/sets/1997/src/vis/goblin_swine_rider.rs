//! Goblin Swine-Rider — {R} — Creature — Goblin
//! Oracle: Whenever this creature becomes blocked, it deals 2 damage to each attacking creature and each blocking creature.
//! Set: VIS #81 — Visions | Scryfall ID: 49980982-d534-4204-bc15-3e6c4ffa1a53 | Oracle ID: 9ebe32fc-3b6e-4cb9-9af0-1a6c3c6ad885
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ATTACKING_OR_BLOCKING_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Or(&[Filter::Attacking, Filter::Blocking]),
]);

card!(
    index = index::GOBLIN_SWINE_RIDER,
    oracle_id = "9ebe32fc-3b6e-4cb9-9af0-1a6c3c6ad885",
    scryfall_id = "49980982-d534-4204-bc15-3e6c4ffa1a53",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Goblin Swine-Rider",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::BecomesBlocked,
        &[Effect::DealDamageEach {
            amount: Amount::Fixed(2),
            filter: &ATTACKING_OR_BLOCKING_CREATURE,
        }],
    )],
);
