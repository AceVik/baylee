//! Marsh Gas — {B} — Instant
//! Oracle: All creatures get -2/-0 until end of turn.
//! Set: 4ED #146 — Fourth Edition | Scryfall ID: c8c65bf9-cbab-45ee-9c6e-f8ee832dbe61 | Oracle ID: 08ae9f91-48c8-4721-a82d-063defc1b727

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MARSH_GAS,
    oracle_id = "08ae9f91-48c8-4721-a82d-063defc1b727",
    scryfall_id = "c8c65bf9-cbab-45ee-9c6e-f8ee832dbe61",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Marsh Gas",
        mana_cost = mana!("{B}"),
        types = TypeSet::INSTANT,
        abilities = &[spell!(&[Effect::CreateContinuousEffect {
            layer: Layer::PtModify,
            filter: &Filter::CREATURE,
            modifier: Modifier::ModifyPT(-2, 0),
            duration: Duration::UntilEndOfTurn,
        }])],
    ),],
);
