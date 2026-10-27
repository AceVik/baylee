//! Solar Blast — {3}{R} — Instant
//! Oracle: Solar Blast deals 3 damage to any target.
//! Oracle: Cycling {1}{R}{R} ({1}{R}{R}, Discard this card: Draw a card.)
//! Oracle: When you cycle this card, you may have it deal 1 damage to any target.
//! Set: DMR #140 — Dominaria Remastered | Scryfall ID: 87202a72-72aa-46d3-ba16-10a180276905 | Oracle ID: ece3e379-c586-48bd-8742-472b5deccc35
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SOLAR_BLAST,
    oracle_id = "ece3e379-c586-48bd-8742-472b5deccc35",
    scryfall_id = "87202a72-72aa-46d3-ba16-10a180276905",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Solar Blast",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
