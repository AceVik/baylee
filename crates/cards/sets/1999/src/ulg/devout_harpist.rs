//! Devout Harpist — {W} — Creature — Human
//! Oracle: {T}: Destroy target Aura attached to a creature.
//! Set: ULG #6 — Urza's Legacy | Scryfall ID: 985b5c60-8a5e-4473-ba43-583aef50f19e | Oracle ID: f57901c2-80c6-49d6-8de3-c96aaeb72bbf
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const AURA_ATTACHED_TO_CREATURE: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::enchantment::AURA),
    Filter::AttachedTo(&Filter::CREATURE),
]);

card!(
    index = index::DEVOUT_HARPIST,
    oracle_id = "f57901c2-80c6-49d6-8de3-c96aaeb72bbf",
    scryfall_id = "985b5c60-8a5e-4473-ba43-583aef50f19e",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Devout Harpist",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!(TapSelf),
        &[Effect::destroy(TargetSpec::Object(
            &AURA_ATTACHED_TO_CREATURE,
        ))],
        targets = Some(TargetReq::one(TargetSpec::Object(
            &AURA_ATTACHED_TO_CREATURE,
        ))),
    )],
);
