//! Restless Dreams — {B} — Sorcery
//! Oracle: As an additional cost to cast this spell, discard X cards.
//! Oracle: Return X target creature cards from your graveyard to your hand.
//! Set: TOR #79 — Torment | Scryfall ID: 63795680-5310-4c99-92c1-70e453391de9 | Oracle ID: 65f1dd09-0640-47f8-8bfd-7aceaa5646a6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RESTLESS_DREAMS,
    oracle_id = "65f1dd09-0640-47f8-8bfd-7aceaa5646a6",
    scryfall_id = "63795680-5310-4c99-92c1-70e453391de9",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Restless Dreams",
        mana_cost = mana!("{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
