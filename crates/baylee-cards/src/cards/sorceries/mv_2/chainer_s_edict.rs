//! Chainer's Edict — {1}{B} — Sorcery
//! Oracle: Target player sacrifices a creature of their choice.
//! Oracle: Flashback {5}{B}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: PLST #DMR-78 — The List | Scryfall ID: a1d194d7-dccf-4421-be8f-8efdcf9fbbeb | Oracle ID: 7975cb9d-d37e-44f8-9614-86d2ad453c35
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CHAINER_S_EDICT,
    oracle_id = "7975cb9d-d37e-44f8-9614-86d2ad453c35",
    scryfall_id = "a1d194d7-dccf-4421-be8f-8efdcf9fbbeb",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Chainer's Edict",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
