//! Parallax Dementia — {1}{B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Fading 1 (This enchantment enters with one fade counter on it. At the beginning of your upkeep, remove a fade counter from it. If you can't, sacrifice it.)
//! Oracle: Enchanted creature gets +3/+2.
//! Oracle: When this Aura leaves the battlefield, destroy enchanted creature. That creature can't be regenerated.
//! Set: NEM #62 — Nemesis | Scryfall ID: 154789ac-bbea-467b-9655-76f378a53f40 | Oracle ID: 6ec7a7fb-0bf9-47fa-af2b-5b13b3af3947
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PARALLAX_DEMENTIA,
    oracle_id = "6ec7a7fb-0bf9-47fa-af2b-5b13b3af3947",
    scryfall_id = "154789ac-bbea-467b-9655-76f378a53f40",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Parallax Dementia",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
