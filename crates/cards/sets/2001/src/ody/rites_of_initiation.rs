//! Rites of Initiation — {R} — Instant
//! Oracle: Discard any number of cards at random. Creatures you control get +1/+0 until end of turn for each card discarded this way.
//! Set: VMA #182 — Vintage Masters | Scryfall ID: f17d9c60-5f81-4b65-8af1-9c5bb757bd7d | Oracle ID: 77aa591d-efb5-48b5-98aa-5c8912ede11b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RITES_OF_INITIATION,
    oracle_id = "77aa591d-efb5-48b5-98aa-5c8912ede11b",
    scryfall_id = "f17d9c60-5f81-4b65-8af1-9c5bb757bd7d",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Rites of Initiation",
        mana_cost = mana!("{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
