//! Nostalgic Dreams — {G}{G} — Sorcery
//! Oracle: As an additional cost to cast this spell, discard X cards.
//! Oracle: Return X target cards from your graveyard to your hand. Exile Nostalgic Dreams.
//! Set: VMA #222 — Vintage Masters | Scryfall ID: 299065ad-ee2e-4973-9b41-fd04c36d19a4 | Oracle ID: fabbc474-682c-4565-861c-d41de05abb8f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NOSTALGIC_DREAMS,
    oracle_id = "fabbc474-682c-4565-861c-d41de05abb8f",
    scryfall_id = "299065ad-ee2e-4973-9b41-fd04c36d19a4",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Nostalgic Dreams",
        mana_cost = mana!("{G}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
