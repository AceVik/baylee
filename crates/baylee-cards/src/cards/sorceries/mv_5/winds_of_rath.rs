//! Winds of Rath — {3}{W}{W} — Sorcery
//! Oracle: Destroy all creatures that aren't enchanted. They can't be regenerated.
//! Set: SOC #185 — Secrets of Strixhaven Commander | Scryfall ID: 8dee155b-1f06-4d74-afa8-a1244c254978 | Oracle ID: a6fd90dc-0ec3-4dce-a77a-4d04f5e254bf
// IMPLEMENTED — Destroys all creatures that aren't enchanted without regeneration.

use baylee_cards_dsl::prelude::*;

static UNENCHANTED_CREATURE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::Enchanted)]);

card!(
    index = index::WINDS_OF_RATH,
    oracle_id = "a6fd90dc-0ec3-4dce-a77a-4d04f5e254bf",
    scryfall_id = "8dee155b-1f06-4d74-afa8-a1244c254978",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Winds of Rath",
        mana_cost = mana!("{3}{W}{W}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::destroy_all_no_regen(
        &UNENCHANTED_CREATURE
    )])],
);
