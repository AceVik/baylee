//! Treacherous Werewolf — {2}{B} — Creature — Werewolf Minion
//! Oracle: Threshold — As long as there are seven or more cards in your graveyard, this creature gets +2/+2 and has "When this creature dies, you lose 4 life."
//! Set: JUD #76 — Judgment | Scryfall ID: c9647726-302b-4fc4-91d7-2aa0bba0b653 | Oracle ID: 59681998-c395-4d37-a419-2a78cf894506
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TREACHEROUS_WEREWOLF,
    oracle_id = "59681998-c395-4d37-a419-2a78cf894506",
    scryfall_id = "c9647726-302b-4fc4-91d7-2aa0bba0b653",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Treacherous Werewolf",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WEREWOLF, subtypes::creature::MINION],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
