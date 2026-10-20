//! Bone Shaman — {2}{R}{R} — Creature — Giant Shaman
//! Oracle: {B}: Until end of turn, this creature gains "Creatures dealt damage by this creature this turn can't be regenerated this turn."
//! Set: ICE #176 — Ice Age | Scryfall ID: 0a5e3d54-4dc4-482b-8ecc-bb819ba03d2c | Oracle ID: 690ba846-964e-44ba-8637-9a1ce62f5931

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: "Creatures dealt damage by this creature this turn can't be regenerated this turn." —
// Effect::TargetDealsUnregeneratableDamageThisTurn reads resolution targets rather than the source creature.
card!(
    index = index::BONE_SHAMAN,
    oracle_id = "690ba846-964e-44ba-8637-9a1ce62f5931",
    scryfall_id = "0a5e3d54-4dc4-482b-8ecc-bb819ba03d2c",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    coverage = Coverage::Partial("missing self-scoped unregeneratable damage effect"),
    faces = &[face!(
        name = "Bone Shaman",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GIANT, subtypes::creature::SHAMAN],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[activated!(cost!("{B}"), &[])],
);
