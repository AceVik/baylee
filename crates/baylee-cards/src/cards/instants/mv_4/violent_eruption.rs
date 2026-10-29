//! Violent Eruption — {1}{R}{R}{R} — Instant
//! Oracle: Violent Eruption deals 4 damage divided as you choose among any number of targets.
//! Oracle: Madness {1}{R}{R} (If you discard this card, discard it into exile. When you do, cast it for its madness cost or put it into your graveyard.)
//! Set: C19 #154 — Commander 2019 | Scryfall ID: 419e6697-89ab-49f1-a867-8f86f001c80c | Oracle ID: 842578ca-0a86-4e96-bfd5-45931488f7c1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VIOLENT_ERUPTION,
    oracle_id = "842578ca-0a86-4e96-bfd5-45931488f7c1",
    scryfall_id = "419e6697-89ab-49f1-a867-8f86f001c80c",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Violent Eruption",
        mana_cost = mana!("{1}{R}{R}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
