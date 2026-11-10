//! Tempting Licid — {2}{G} — Creature — Licid
//! Oracle: {G}, {T}: This creature loses this ability and becomes an Aura enchantment with enchant creature. Attach it to target creature. You may pay {G} to end this effect.
//! Oracle: All creatures able to block enchanted creature do so.
//! Set: STH #122 — Stronghold | Scryfall ID: da7f3e0b-0600-4451-b621-e40c902a16cb | Oracle ID: 3b2206ed-55e9-4735-b969-59d9e089bb10
// PARTIAL — Licid Aura transformation, attachment, loss of the activation and payment to end the effect are not supported; only the creature body is implemented

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TEMPTING_LICID,
    oracle_id = "3b2206ed-55e9-4735-b969-59d9e089bb10",
    scryfall_id = "da7f3e0b-0600-4451-b621-e40c902a16cb",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "Licid Aura transformation, attachment, loss of the activation and payment to end the effect are not supported; only the creature body is implemented"
    ),
    faces = &[face!(
        name = "Tempting Licid",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LICID],
        power = Some(2),
        toughness = Some(2),
    ),],
    // NOT SUPPORTED: Licid Aura transformation, attachment, loss of the activation and payment to end the effect are not supported; only the creature body is implemented.
);
