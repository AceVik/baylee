//! Gliding Licid — {2}{U} — Creature — Licid
//! Oracle: {U}, {T}: This creature loses this ability and becomes an Aura enchantment with enchant creature. Attach it to target creature. You may pay {U} to end this effect.
//! Oracle: Enchanted creature has flying.
//! Set: STH #31 — Stronghold | Scryfall ID: 0148f3ca-4dba-4408-86d0-1190c387ee69 | Oracle ID: a07b8619-224d-46d0-a17a-38d04aba574e
// PARTIAL — Licid Aura transformation, attachment, loss of the activation and payment to end the effect are not supported; only the creature body is implemented

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GLIDING_LICID,
    oracle_id = "a07b8619-224d-46d0-a17a-38d04aba574e",
    scryfall_id = "0148f3ca-4dba-4408-86d0-1190c387ee69",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "Licid Aura transformation, attachment, loss of the activation and payment to end the effect are not supported; only the creature body is implemented"
    ),
    faces = &[face!(
        name = "Gliding Licid",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LICID],
        power = Some(2),
        toughness = Some(2),
    ),],
    // NOT SUPPORTED: Licid Aura transformation, attachment, loss of the activation and payment to end the effect are not supported; only the creature body is implemented.
);
