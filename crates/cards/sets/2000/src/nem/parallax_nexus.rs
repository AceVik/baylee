//! Parallax Nexus — {2}{B} — Enchantment
//! Oracle: Fading 5 (This enchantment enters with five fade counters on it. At the beginning of your upkeep, remove a fade counter from it. If you can't, sacrifice it.)
//! Oracle: Remove a fade counter from this enchantment: Target opponent exiles a card from their hand. Activate only as a sorcery.
//! Oracle: When this enchantment leaves the battlefield, each player returns to their hand all cards they own exiled with it.
//! Set: NEM #63 — Nemesis | Scryfall ID: 862c50c7-0840-46e0-a653-5b660fdfd4bd | Oracle ID: 40fbcf4b-d79e-4e6b-bcec-1d1544b3c4a2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PARALLAX_NEXUS,
    oracle_id = "40fbcf4b-d79e-4e6b-bcec-1d1544b3c4a2",
    scryfall_id = "862c50c7-0840-46e0-a653-5b660fdfd4bd",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Parallax Nexus",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
