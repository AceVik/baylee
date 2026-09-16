//! Dauthi Voidwalker — {B}{B} — Creature — Dauthi Rogue
//! Oracle: Shadow (This creature can block or be blocked by only creatures with shadow.)
//! Oracle: If a card would be put into an opponent's graveyard from anywhere, instead exile it with a void counter on it.
//! Oracle: {T}, Sacrifice this creature: Choose an exiled card an opponent owns with a void counter on it. You may play it this turn without paying its mana cost.
//! Set: TDC #176 — Tarkir: Dragonstorm Commander | Scryfall ID: 3573b9a2-7911-475c-8ae7-25bd0dbb7159 | Oracle ID: f1c2dbe2-fbe0-4058-bdf1-91d1b1832786
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DAUTHI_VOIDWALKER,
    oracle_id = "f1c2dbe2-fbe0-4058-bdf1-91d1b1832786",
    scryfall_id = "3573b9a2-7911-475c-8ae7-25bd0dbb7159",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Dauthi Voidwalker",
        mana_cost = mana!("{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DAUTHI, subtypes::creature::ROGUE],
        power = Some(3),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
