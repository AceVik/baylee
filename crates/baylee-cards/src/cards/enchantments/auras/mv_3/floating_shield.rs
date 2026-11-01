//! Floating Shield — {2}{W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: As this Aura enters, choose a color.
//! Oracle: Enchanted creature has protection from the chosen color. This effect doesn't remove this Aura.
//! Oracle: Sacrifice this Aura: Target creature gains protection from the chosen color until end of turn.
//! Set: TOR #5 — Torment | Scryfall ID: 4dbda39b-a998-4ad9-95df-72ed9556c390 | Oracle ID: 332716fb-cc1c-4a6f-b7d4-bd7f13cfa438
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FLOATING_SHIELD,
    oracle_id = "332716fb-cc1c-4a6f-b7d4-bd7f13cfa438",
    scryfall_id = "4dbda39b-a998-4ad9-95df-72ed9556c390",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Floating Shield",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
