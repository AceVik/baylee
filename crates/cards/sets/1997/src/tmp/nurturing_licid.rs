//! Nurturing Licid — {1}{G} — Creature — Licid
//! Oracle: {G}, {T}: This creature loses this ability and becomes an Aura enchantment with enchant creature. Attach it to target creature. You may pay {G} to end this effect.
//! Oracle: {G}: Regenerate enchanted creature.
//! Set: TMP #242 — Tempest | Scryfall ID: 0bf53069-44ac-49c5-83bf-9c3c1274e407 | Oracle ID: 537a7af1-2b7e-4828-b645-7e34e86058d5

// PARTIAL — Licid Aura transformation, special action to end it, and regeneration of the enchanted creature are not supported; only the 1/1 body is played.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NURTURING_LICID,
    oracle_id = "537a7af1-2b7e-4828-b645-7e34e86058d5",
    scryfall_id = "0bf53069-44ac-49c5-83bf-9c3c1274e407",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "Licid Aura transformation, special action to end it, and regeneration of the enchanted creature are not supported; only the 1/1 body is played."
    ),
    faces = &[face!(
        name = "Nurturing Licid",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LICID],
        power = Some(1),
        toughness = Some(1),
    ),],
    // NOT SUPPORTED: Licid Aura transformation, special action to end it, and regeneration of the enchanted creature are not supported; only the 1/1 body is played.
);
