//! Explosive Growth — {G} — Instant
//! Oracle: Kicker {5} (You may pay an additional {5} as you cast this spell.)
//! Oracle: Target creature gets +2/+2 until end of turn. If this spell was kicked, that creature gets +5/+5 until end of turn instead.
//! Set: INV #187 — Invasion | Scryfall ID: eabc1e77-404c-436b-bde1-be1b21d00584 | Oracle ID: 96c65faf-859c-4436-a551-5538409a9792
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EXPLOSIVE_GROWTH,
    oracle_id = "96c65faf-859c-4436-a551-5538409a9792",
    scryfall_id = "eabc1e77-404c-436b-bde1-be1b21d00584",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Explosive Growth",
        mana_cost = mana!("{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
