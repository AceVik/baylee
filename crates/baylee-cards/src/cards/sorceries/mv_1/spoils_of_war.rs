//! Spoils of War — {X}{B} — Sorcery
//! Oracle: X is the number of artifact and/or creature cards in an opponent's graveyard as you cast this spell.
//! Oracle: Distribute X +1/+1 counters among any number of target creatures.
//! Set: ICE #164 — Ice Age | Scryfall ID: b38af8bd-d927-46d0-a1b1-fb437ea9ea66 | Oracle ID: f3240a86-28b6-4338-9a59-1fb6f795d713
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SPOILS_OF_WAR,
    oracle_id = "f3240a86-28b6-4338-9a59-1fb6f795d713",
    scryfall_id = "b38af8bd-d927-46d0-a1b1-fb437ea9ea66",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Spoils of War",
        mana_cost = mana!("{X}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
