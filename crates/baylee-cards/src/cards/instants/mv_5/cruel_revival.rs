//! Cruel Revival — {4}{B} — Instant
//! Oracle: Destroy target non-Zombie creature. It can't be regenerated. Return up to one target Zombie card from your graveyard to your hand.
//! Set: GNT #28 — Game Night | Scryfall ID: 5f4b76ac-550e-4450-bade-66b176add109 | Oracle ID: 836d074a-8888-42a0-8dce-11ffce00d2f5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CRUEL_REVIVAL,
    oracle_id = "836d074a-8888-42a0-8dce-11ffce00d2f5",
    scryfall_id = "5f4b76ac-550e-4450-bade-66b176add109",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Cruel Revival",
        mana_cost = mana!("{4}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
