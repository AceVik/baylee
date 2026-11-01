//! Diabolic Edict — {1}{B} — Instant
//! Oracle: Target player sacrifices a creature of their choice.
//! Set: MH1 #87 — Modern Horizons | Scryfall ID: 4eabbed2-1399-4cf1-9eba-b53c56caced4 | Oracle ID: 058917c1-21ab-488a-9f9c-591c55f3c596
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DIABOLIC_EDICT,
    oracle_id = "058917c1-21ab-488a-9f9c-591c55f3c596",
    scryfall_id = "4eabbed2-1399-4cf1-9eba-b53c56caced4",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Diabolic Edict",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::SacrificeFilter {
            who: PlayerRel::Chosen,
            filter: &Filter::CREATURE,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
    )],
);
