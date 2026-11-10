//! Necropotence — {B}{B}{B} — Enchantment
//! Oracle: Skip your draw step.
//! Oracle: Whenever you discard a card, exile that card from your graveyard.
//! Oracle: Pay 1 life: Exile the top card of your library face down. Put that card into your hand at the beginning of your next end step.
//! Set: IMA #98 — Iconic Masters | Scryfall ID: c89c6895-b0f8-444a-9c89-c6b4fd027b3e | Oracle ID: 94a844d2-0574-45a7-b347-e0e329767c42
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NECROPOTENCE,
    oracle_id = "94a844d2-0574-45a7-b347-e0e329767c42",
    scryfall_id = "c89c6895-b0f8-444a-9c89-c6b4fd027b3e",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Necropotence",
        mana_cost = mana!("{B}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
