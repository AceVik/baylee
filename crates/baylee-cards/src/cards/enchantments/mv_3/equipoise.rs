//! Equipoise — {2}{W} — Enchantment
//! Oracle: At the beginning of your upkeep, for each land target player controls in excess of the number you control, choose a land that player controls, then the chosen permanents phase out. Repeat this process for artifacts and creatures. (While they're phased out, they're treated as though they don't exist. They phase in before that player untaps during their next untap step.)
//! Set: VIS #3 — Visions | Scryfall ID: 53783312-3551-4361-ab02-c9651ce2a926 | Oracle ID: aab905ef-b945-4523-b26f-fb5e6932bb56
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EQUIPOISE,
    oracle_id = "aab905ef-b945-4523-b26f-fb5e6932bb56",
    scryfall_id = "53783312-3551-4361-ab02-c9651ce2a926",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Equipoise",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
