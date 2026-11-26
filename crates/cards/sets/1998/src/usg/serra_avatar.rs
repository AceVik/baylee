//! Serra Avatar — {4}{W}{W}{W} — Creature — Avatar
//! Oracle: Serra Avatar's power and toughness are each equal to your life total.
//! Oracle: When Serra Avatar is put into a graveyard from anywhere, shuffle it into its owner's library.
//! Set: DMR #26 — Dominaria Remastered | Scryfall ID: 5041dcfd-c1af-448c-91f7-75fd1d34e93e | Oracle ID: c363725f-15cf-4955-aaf1-536b1f8daaed
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SERRA_AVATAR,
    oracle_id = "c363725f-15cf-4955-aaf1-536b1f8daaed",
    scryfall_id = "5041dcfd-c1af-448c-91f7-75fd1d34e93e",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Serra Avatar",
        mana_cost = mana!("{4}{W}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::AVATAR],
        power = Some(0),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
