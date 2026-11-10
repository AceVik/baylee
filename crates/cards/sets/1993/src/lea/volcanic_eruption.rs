//! Volcanic Eruption — {X}{U}{U}{U} — Sorcery
//! Oracle: Destroy X target Mountains. Volcanic Eruption deals damage to each creature and each player equal to the number of Mountains put into a graveyard this way.
//! Set: 4ED #112 — Fourth Edition | Scryfall ID: 5828713a-edb3-4b11-b1f9-8f1bfc3c103f | Oracle ID: 4c15889e-3172-413b-b805-a2b7ad05f636
// IMPLEMENTED — the damage counts the targets a graveyard now holds
// (`Amount::TargetsPutIntoGraveyard`), so a regenerated Mountain is not one.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VOLCANIC_ERUPTION,
    oracle_id = "4c15889e-3172-413b-b805-a2b7ad05f636",
    scryfall_id = "5828713a-edb3-4b11-b1f9-8f1bfc3c103f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Volcanic Eruption",
        mana_cost = mana!("{X}{U}{U}{U}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[
            Effect::destroy(TargetSpec::Object(&Filter::HasSubtype(
                subtypes::land::MOUNTAIN
            ))),
            Effect::DealDamageEach {
                amount: Amount::TargetsPutIntoGraveyard,
                filter: &Filter::CREATURE
            },
            Effect::DealDamage {
                amount: Amount::TargetsPutIntoGraveyard,
                target: TargetSpec::Player(PlayerRel::EachPlayer)
            },
        ],
        targets = Some(TargetReq::x_targets(TargetSpec::Object(
            &Filter::HasSubtype(subtypes::land::MOUNTAIN)
        )))
    )],
);
