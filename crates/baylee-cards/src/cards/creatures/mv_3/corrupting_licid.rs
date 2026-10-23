//! Corrupting Licid — {2}{B} — Creature — Licid
//! Oracle: {B}, {T}: This creature loses this ability and becomes an Aura enchantment with enchant creature. Attach it to target creature. You may pay {B} to end this effect.
//! Oracle: Enchanted creature has fear. (It can't be blocked except by artifact creatures and/or black creatures.)
//! Set: STH #54 — Stronghold | Scryfall ID: c12c6548-20e6-4697-ba26-e711590f9e28 | Oracle ID: 503ddd47-4b0c-4a2a-b77a-fb321044b438
// PARTIAL — Licid Aura transformation, attachment, loss of the activation and payment to end the effect are not supported; only the creature body is implemented

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CORRUPTING_LICID,
    oracle_id = "503ddd47-4b0c-4a2a-b77a-fb321044b438",
    scryfall_id = "c12c6548-20e6-4697-ba26-e711590f9e28",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Licid Aura transformation, attachment, loss of the activation and payment to end the effect are not supported; only the creature body is implemented"
    ),
    faces = &[face!(
        name = "Corrupting Licid",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LICID],
        power = Some(2),
        toughness = Some(2),
    ),],
    // NOT SUPPORTED: Licid Aura transformation, attachment, loss of the activation and payment to end the effect are not supported; only the creature body is implemented.
);
