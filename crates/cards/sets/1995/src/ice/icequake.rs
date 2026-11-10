//! Icequake — {1}{B}{B} — Sorcery
//! Oracle: Destroy target land. If that land was a snow land, Icequake deals 1 damage to that land's controller.
//! Set: ME2 #94 — Masters Edition II | Scryfall ID: 550880ba-26a0-463d-9da2-0160d224cda1 | Oracle ID: 8c0a6ac1-b1cc-405a-95fb-f06b7fecbb54
use baylee_cards_dsl::prelude::*;

static SNOW_LAND: Filter = Filter::And(&[Filter::LAND, Filter::HasSupertype(SupertypeSet::SNOW)]);

card!(
    index = index::ICEQUAKE,
    oracle_id = "8c0a6ac1-b1cc-405a-95fb-f06b7fecbb54",
    scryfall_id = "550880ba-26a0-463d-9da2-0160d224cda1",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Icequake",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::IfTargetMatchesElse {
            filter: &SNOW_LAND,
            then: &[
                Effect::destroy(TargetSpec::Object(&Filter::LAND)),
                Effect::DealDamageToTargetController {
                    amount: Amount::Fixed(1),
                },
            ],
            otherwise: &[Effect::destroy(TargetSpec::Object(&Filter::LAND))],
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::LAND)))
    )],
);
