//! Natural Balance — {2}{G}{G} — Sorcery
//! Oracle: Each player who controls six or more lands chooses five lands they control and sacrifices the rest. Each player who controls four or fewer lands may search their library for up to X basic land cards and put them onto the battlefield, where X is five minus the number of lands they control. Then each player who searched their library this way shuffles.
//! Set: MIR #231 — Mirage | Scryfall ID: b0f7c47c-416e-4f73-89ec-024a29dfb5e9 | Oracle ID: 69acaa6f-3ba2-420c-9069-02958f844121
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NATURAL_BALANCE,
    oracle_id = "69acaa6f-3ba2-420c-9069-02958f844121",
    scryfall_id = "b0f7c47c-416e-4f73-89ec-024a29dfb5e9",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Natural Balance",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
