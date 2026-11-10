//! Angelic Favor — {3}{W} — Instant
//! Oracle: If you control a Plains, you may tap an untapped creature you control rather than pay this spell's mana cost.
//! Oracle: Cast this spell only during combat.
//! Oracle: Create a 4/4 white Angel creature token with flying. Exile it at the beginning of the next end step.
//! Set: NEM #1 — Nemesis | Scryfall ID: 871ad2f3-1dd2-45ea-881d-529aad3b76ec | Oracle ID: 37162c6e-ac31-4386-9d1e-76aa66070b35
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ANGELIC_FAVOR,
    oracle_id = "37162c6e-ac31-4386-9d1e-76aa66070b35",
    scryfall_id = "871ad2f3-1dd2-45ea-881d-529aad3b76ec",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Angelic Favor",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
