//! Fugitive Druid — {3}{G} — Creature — Human Druid
//! Oracle: Whenever this creature becomes the target of an Aura spell, you draw a card.
//! Set: TMP #229 — Tempest | Scryfall ID: afe165cd-8ef7-408e-ae56-3c6a0cc4e409 | Oracle ID: e05f3236-dc82-4353-87ab-d26ae7caa0e7
// PARTIAL

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FUGITIVE_DRUID,
    oracle_id = "e05f3236-dc82-4353-87ab-d26ae7caa0e7",
    scryfall_id = "afe165cd-8ef7-408e-ae56-3c6a0cc4e409",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "Triggers whenever this creature becomes the target of a spell or ability to draw a card; restricting Trigger::BecomesTarget specifically to Aura spells is not yet modeled."
    ),
    faces = &[face!(
        name = "Fugitive Druid",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::DRUID],
        power = Some(3),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(Trigger::BecomesTarget, &[Effect::draw(1)])],
);
