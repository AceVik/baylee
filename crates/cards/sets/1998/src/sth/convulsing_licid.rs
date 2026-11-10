//! Convulsing Licid — {2}{R} — Creature — Licid
//! Oracle: {R}, {T}: This creature loses this ability and becomes an Aura enchantment with enchant creature. Attach it to target creature. You may pay {R} to end this effect.
//! Oracle: Enchanted creature can't block.
//! Set: STH #77 — Stronghold | Scryfall ID: d5e50a1d-b9f5-4a03-a1c2-ca45ace53a52 | Oracle ID: 893c88cb-72ba-4f54-809c-bfac4d6efb74
// PARTIAL — Licid Aura transformation, attachment, loss of the activation and payment to end the effect are not supported; only the creature body is implemented

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CONVULSING_LICID,
    oracle_id = "893c88cb-72ba-4f54-809c-bfac4d6efb74",
    scryfall_id = "d5e50a1d-b9f5-4a03-a1c2-ca45ace53a52",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "Licid Aura transformation, attachment, loss of the activation and payment to end the effect are not supported; only the creature body is implemented"
    ),
    faces = &[face!(
        name = "Convulsing Licid",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LICID],
        power = Some(2),
        toughness = Some(2),
    ),],
    // NOT SUPPORTED: Licid Aura transformation, attachment, loss of the activation and payment to end the effect are not supported; only the creature body is implemented.
);
