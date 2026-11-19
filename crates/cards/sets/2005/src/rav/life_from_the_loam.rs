//! Life from the Loam — {1}{G} — Sorcery
//! Oracle: Return up to three target land cards from your graveyard to your hand.
//! Oracle: Dredge 3 (If you would draw a card, you may mill three cards instead. If you do, return this card from your graveyard to your hand.)
//! Set: TDC #96 — Tarkir: Dragonstorm Commander | Scryfall ID: cdb6432f-3f56-4c92-b09a-2a5b64620c04 | Oracle ID: ac8fba34-512c-4f24-999a-ab72f1ce4acb
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LIFE_FROM_THE_LOAM,
    oracle_id = "ac8fba34-512c-4f24-999a-ab72f1ce4acb",
    scryfall_id = "cdb6432f-3f56-4c92-b09a-2a5b64620c04",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Life from the Loam",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
