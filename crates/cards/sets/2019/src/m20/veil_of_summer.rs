//! Veil of Summer — {G} — Instant
//! Oracle: Draw a card if an opponent has cast a blue or black spell this turn. Spells you control can't be countered this turn. You and permanents you control gain hexproof from blue and from black until end of turn. (You and they can't be the targets of blue or black spells or abilities your opponents control.)
//! Set: M20 #198 — Core Set 2020 | Scryfall ID: aa686c34-1c11-469f-93c2-f9891aea521f | Oracle ID: 002965be-a36f-4a09-9ce0-c6535bca1703
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VEIL_OF_SUMMER,
    oracle_id = "002965be-a36f-4a09-9ce0-c6535bca1703",
    scryfall_id = "aa686c34-1c11-469f-93c2-f9891aea521f",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Veil of Summer",
        mana_cost = mana!("{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
