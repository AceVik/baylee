//! Vitalizing Cascade — {X}{G}{W} — Instant
//! Oracle: You gain X plus 3 life.
//! Set: MIR #286 — Mirage | Scryfall ID: 5fe8a5b8-1a87-46f5-920f-fbbb05bfd563 | Oracle ID: aaf176ff-646f-4182-a546-fe648498587d
// IMPLEMENTED — you gain X plus 3 life.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VITALIZING_CASCADE,
    oracle_id = "aaf176ff-646f-4182-a546-fe648498587d",
    scryfall_id = "5fe8a5b8-1a87-46f5-920f-fbbb05bfd563",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Vitalizing Cascade",
        mana_cost = mana!("{X}{G}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::GainLife {
        amount: Amount::Plus {
            base: &Amount::X,
            offset: 3,
        },
    }])],
);
