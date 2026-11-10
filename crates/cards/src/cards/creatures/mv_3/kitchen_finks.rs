//! Kitchen Finks — {1}{G/W}{G/W} — Creature — Ouphe
//! Oracle: When this creature enters, you gain 2 life.
//! Oracle: Persist (When this creature dies, if it had no -1/-1 counters on it, return it to the battlefield under its owner's control with a -1/-1 counter on it.)
//! Set: UMA #216 — Ultimate Masters | Scryfall ID: a844d537-df73-422d-a154-f1c6b92d0469 | Oracle ID: 5470dcfa-4eff-43da-abf7-19922841f719

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KITCHEN_FINKS,
    oracle_id = "5470dcfa-4eff-43da-abf7-19922841f719",
    scryfall_id = "a844d537-df73-422d-a154-f1c6b92d0469",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Kitchen Finks",
        mana_cost = mana!("{1}{G/W}{G/W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::OUPHE],
        power = Some(3),
        toughness = Some(2),
        keywords = KeywordSet::PERSIST,
    ),],
    abilities = &[triggered!(Trigger::ETB, &[Effect::gain_life(2)])],
);
