//! Crested Craghorn — {4}{R} — Creature — Goat Beast
//! Oracle: Haste
//! Oracle: Provoke (Whenever this creature attacks, you may have target creature defending player controls untap and block it if able.)
//! Set: LGN #91 — Legions | Scryfall ID: aadb40c8-3d54-4705-82dc-54e8d6e315d5 | Oracle ID: 597c390e-deeb-4452-aeda-7cf9312a8b01
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CRESTED_CRAGHORN,
    oracle_id = "597c390e-deeb-4452-aeda-7cf9312a8b01",
    scryfall_id = "aadb40c8-3d54-4705-82dc-54e8d6e315d5",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Crested Craghorn",
        mana_cost = mana!("{4}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOAT, subtypes::creature::BEAST],
        power = Some(4),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
