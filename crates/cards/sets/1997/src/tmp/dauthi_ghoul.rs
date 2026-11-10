//! Dauthi Ghoul — {1}{B} — Creature — Dauthi Zombie
//! Oracle: Shadow (This creature can block or be blocked by only creatures with shadow.)
//! Oracle: Whenever a creature with shadow dies, put a +1/+1 counter on this creature.
//! Set: TMP #121 — Tempest | Scryfall ID: 0a70778e-8171-4d97-b86c-d4d92b7e7f06 | Oracle ID: 5bf39f9e-4f06-4637-b0fc-5d21b926f6f4
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const SHADOW_CREATURE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::SHADOW)]);

card!(
    index = index::DAUTHI_GHOUL,
    oracle_id = "5bf39f9e-4f06-4637-b0fc-5d21b926f6f4",
    scryfall_id = "0a70778e-8171-4d97-b86c-d4d92b7e7f06",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::SHADOW,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Dauthi Ghoul",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DAUTHI, subtypes::creature::ZOMBIE],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::Dies(&SHADOW_CREATURE),
        &[Effect::AddCounterSource {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(1),
        }],
    )],
);
