//! Quagmire Lamprey — {2}{B} — Creature — Fish
//! Oracle: Whenever this creature becomes blocked by a creature, put a -1/-1 counter on that creature.
//! Set: MMQ #154 — Mercadian Masques | Scryfall ID: 3c91c44e-6bfc-4595-9cdb-17d73f912c09 | Oracle ID: f5b374d8-bf22-44f7-a18a-18c096a84640
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::QUAGMIRE_LAMPREY,
    oracle_id = "f5b374d8-bf22-44f7-a18a-18c096a84640",
    scryfall_id = "3c91c44e-6bfc-4595-9cdb-17d73f912c09",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Quagmire Lamprey",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::FISH],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::BecomesBlockedBy(&Filter::CREATURE),
        &[Effect::AddCounterFilter {
            filter: &Filter::Reference,
            kind: CounterKind::M1M1,
            amount: Amount::Fixed(1),
        }]
    ),],
);
