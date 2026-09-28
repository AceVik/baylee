//! Final Showdown — {W} — Instant
//! Oracle: Spree (Choose one or more additional costs.)
//! Oracle: + {1} — All creatures lose all abilities until end of turn.
//! Oracle: + {1} — Choose a creature you control. It gains indestructible until end of turn.
//! Oracle: + {3}{W}{W} — Destroy all creatures.
//! Set: OTJ #11 — Outlaws of Thunder Junction | Scryfall ID: 358968f9-45bd-4022-b6bc-f1f7e0adf0e7 | Oracle ID: 7e7ec3d6-a84f-4cc3-93f4-4d181d41e126
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FINAL_SHOWDOWN,
    oracle_id = "7e7ec3d6-a84f-4cc3-93f4-4d181d41e126",
    scryfall_id = "358968f9-45bd-4022-b6bc-f1f7e0adf0e7",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Final Showdown",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
