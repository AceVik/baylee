//! Baki's Curse — {2}{U}{U} — Sorcery
//! Oracle: Baki's Curse deals 2 damage to each creature for each Aura attached to that creature.
//! Set: HML #22 — Homelands | Scryfall ID: e3261b4c-7963-4ca0-875d-77b7c8571b3f | Oracle ID: 341a92b9-d6a0-499e-abb7-e2b7d7bc248c
// PARTIAL — per-creature damage equal to 2 per Aura attached to that creature is not in Effect; deals 2 damage to each enchanted creature.

use baylee_cards_dsl::prelude::*;

static ENCHANTED_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::Enchanted]);

card!(
    index = index::BAKI_S_CURSE,
    oracle_id = "341a92b9-d6a0-499e-abb7-e2b7d7bc248c",
    scryfall_id = "e3261b4c-7963-4ca0-875d-77b7c8571b3f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "per-creature damage equal to 2 per Aura attached to that creature not in Effect; deals 2 damage to each enchanted creature"
    ),
    faces = &[face!(
        name = "Baki's Curse",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::damage_each(2, &ENCHANTED_CREATURE)])],
);
