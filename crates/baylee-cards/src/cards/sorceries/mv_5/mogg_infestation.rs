//! Mogg Infestation — {3}{R}{R} — Sorcery
//! Oracle: Destroy all creatures target player controls. For each creature that died this way, that player creates two 1/1 red Goblin creature tokens.
//! Set: TPR #146 — Tempest Remastered | Scryfall ID: bfeb1145-3729-481e-a314-c325ed2f2a35 | Oracle ID: cfd61b53-83c6-4886-8503-ddcb92dc7f85
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MOGG_INFESTATION,
    oracle_id = "cfd61b53-83c6-4886-8503-ddcb92dc7f85",
    scryfall_id = "bfeb1145-3729-481e-a314-c325ed2f2a35",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Mogg Infestation",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
