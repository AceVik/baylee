//! Treacherous Vampire — {4}{B} — Creature — Vampire
//! Oracle: Flying
//! Oracle: Whenever this creature attacks or blocks, sacrifice it unless you exile a card from your graveyard.
//! Oracle: Threshold — As long as there are seven or more cards in your graveyard, this creature gets +2/+2 and has "When this creature dies, you lose 6 life."
//! Set: JUD #75 — Judgment | Scryfall ID: a00562ae-b8b4-4f8f-8ea8-15d20568997d | Oracle ID: 1d33d49f-ba12-4bf6-878b-04cee5d5b48d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TREACHEROUS_VAMPIRE,
    oracle_id = "1d33d49f-ba12-4bf6-878b-04cee5d5b48d",
    scryfall_id = "a00562ae-b8b4-4f8f-8ea8-15d20568997d",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Treacherous Vampire",
        mana_cost = mana!("{4}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::VAMPIRE],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
