//! Sivvi's Valor — {2}{W} — Instant
//! Oracle: If you control a Plains, you may tap an untapped creature you control rather than pay this spell's mana cost.
//! Oracle: All damage that would be dealt to target creature this turn is dealt to you instead.
//! Set: NEM #22 — Nemesis | Scryfall ID: 9d15f7b5-5070-4742-a05c-623822d874fb | Oracle ID: e02164ba-34f8-4a5f-a05b-dd3ef3f8ceae
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SIVVI_S_VALOR,
    oracle_id = "e02164ba-34f8-4a5f-a05b-dd3ef3f8ceae",
    scryfall_id = "9d15f7b5-5070-4742-a05c-623822d874fb",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Sivvi's Valor",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
