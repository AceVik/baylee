//! Calming Licid — {2}{W} — Creature — Licid
//! Oracle: {W}, {T}: This creature loses this ability and becomes an Aura enchantment with enchant creature. Attach it to target creature. You may pay {W} to end this effect.
//! Oracle: Enchanted creature can't attack.
//! Set: STH #2 — Stronghold | Scryfall ID: 155fc239-c2eb-41cc-9a0a-2414da3c3d7b | Oracle ID: 93d20859-249f-47ae-8f6d-095228e7b6e9
// PARTIAL — Licid Aura transformation, attachment, loss of the activation and payment to end the effect are not supported; only the creature body is implemented

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CALMING_LICID,
    oracle_id = "93d20859-249f-47ae-8f6d-095228e7b6e9",
    scryfall_id = "155fc239-c2eb-41cc-9a0a-2414da3c3d7b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "Licid Aura transformation, attachment, loss of the activation and payment to end the effect are not supported; only the creature body is implemented"
    ),
    faces = &[face!(
        name = "Calming Licid",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LICID],
        power = Some(2),
        toughness = Some(2),
    ),],
    // NOT SUPPORTED: Licid Aura transformation, attachment, loss of the activation and payment to end the effect are not supported; only the creature body is implemented.
);
