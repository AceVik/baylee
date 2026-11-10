//! Thermokarst — {1}{G}{G} — Sorcery
//! Oracle: Destroy target land. If that land was a snow land, you gain 1 life.
//! Set: ME2 #183 — Masters Edition II | Scryfall ID: 74e907ed-76f7-476c-b128-bb6bfd892e06 | Oracle ID: f3b071a1-7501-4424-979a-780fa90eecb8

use baylee_cards_dsl::prelude::*;

static SNOW_LAND: Filter = Filter::And(&[Filter::LAND, Filter::HasSupertype(SupertypeSet::SNOW)]);

card!(
    index = index::THERMOKARST,
    oracle_id = "f3b071a1-7501-4424-979a-780fa90eecb8",
    scryfall_id = "74e907ed-76f7-476c-b128-bb6bfd892e06",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Thermokarst",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::IfTargetMatchesElse {
            filter: &SNOW_LAND,
            then: &[
                Effect::destroy(TargetSpec::Object(&Filter::LAND)),
                Effect::gain_life(1),
            ],
            otherwise: &[Effect::destroy(TargetSpec::Object(&Filter::LAND))],
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::LAND)))
    )],
);
