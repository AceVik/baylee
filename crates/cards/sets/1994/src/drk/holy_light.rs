//! Holy Light — {2}{W} — Instant
//! Oracle: Nonwhite creatures get -1/-1 until end of turn.
//! Set: ME1 #16 — Masters Edition | Scryfall ID: 34fbb723-2507-489a-acf1-22ac0dcbcd9c | Oracle ID: 8f7dd328-b367-4546-ba79-1efc1e97f2ab

use baylee_cards_dsl::prelude::*;

const NONWHITE_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasColor(ColorSet::of(Color::White))),
]);

card!(
    index = index::HOLY_LIGHT,
    oracle_id = "8f7dd328-b367-4546-ba79-1efc1e97f2ab",
    scryfall_id = "34fbb723-2507-489a-acf1-22ac0dcbcd9c",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Holy Light",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::CreateContinuousEffect {
        layer: Layer::PtModify,
        filter: &NONWHITE_CREATURE,
        modifier: Modifier::ModifyPT(-1, -1),
        duration: Duration::UntilEndOfTurn,
    }])],
);
