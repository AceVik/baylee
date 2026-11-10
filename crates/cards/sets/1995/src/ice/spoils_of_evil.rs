//! Spoils of Evil — {2}{B} — Instant
//! Oracle: For each artifact or creature card in target opponent's graveyard, add {C} and you gain 1 life.
//! Set: ICE #163 — Ice Age | Scryfall ID: fd368eb6-72f0-42d4-afa5-3daa7de949ff | Oracle ID: a40c52ee-4b45-4b14-8fbc-6529b42bfc9d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SPOILS_OF_EVIL,
    oracle_id = "a40c52ee-4b45-4b14-8fbc-6529b42bfc9d",
    scryfall_id = "fd368eb6-72f0-42d4-afa5-3daa7de949ff",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Spoils of Evil",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
