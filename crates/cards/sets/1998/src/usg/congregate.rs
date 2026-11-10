//! Congregate — {3}{W} — Instant
//! Oracle: Target player gains 2 life for each creature on the battlefield.
//! Set: PLST #DMR-4 — The List | Scryfall ID: 1e73f893-b8ab-4c1f-821a-c7dc8de99351 | Oracle ID: d139294c-36bb-4a1e-800d-50c4a4ae9877
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CONGREGATE,
    oracle_id = "d139294c-36bb-4a1e-800d-50c4a4ae9877",
    scryfall_id = "1e73f893-b8ab-4c1f-821a-c7dc8de99351",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Congregate",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::GainLifeFor {
            amount: Amount::Times {
                base: &Amount::CountOf {
                    filter: &Filter::CREATURE,
                    zone: ZoneSel::Battlefield,
                },
                factor: 2,
            },
            who: PlayerRel::Chosen,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
    )],
);
