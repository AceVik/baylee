//! Saproling Burst — {4}{G} — Enchantment
//! Oracle: Fading 7 (This enchantment enters with seven fade counters on it. At the beginning of your upkeep, remove a fade counter from it. If you can't, sacrifice it.)
//! Oracle: Remove a fade counter from this enchantment: Create a green Saproling creature token. It has "This token's power and toughness are each equal to the number of fade counters on Saproling Burst."
//! Oracle: When this enchantment leaves the battlefield, destroy all tokens created with this enchantment. They can't be regenerated.
//! Set: VMA #230 — Vintage Masters | Scryfall ID: 1534b459-62eb-42ef-a3c6-2e57348b04be | Oracle ID: 7b31ecf8-5266-4376-ace0-8b873706c4ea
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SAPROLING_BURST,
    oracle_id = "7b31ecf8-5266-4376-ace0-8b873706c4ea",
    scryfall_id = "1534b459-62eb-42ef-a3c6-2e57348b04be",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Saproling Burst",
        mana_cost = mana!("{4}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
