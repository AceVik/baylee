//! Necrite — {1}{B}{B} — Creature — Thrull
//! Oracle: Whenever this creature attacks and isn't blocked, you may sacrifice it. If you do, destroy target creature defending player controls. It can't be regenerated.
//! Set: ME2 #106 — Masters Edition II | Scryfall ID: 86a99edc-22d9-4653-8a7a-825f4b443d75 | Oracle ID: e6ab8a04-e225-4ff0-bb8b-c80d0e6ce50c
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const DEFENDERS_CREATURE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::ControlledByDefendingPlayer]);

card!(
    index = index::NECRITE,
    oracle_id = "e6ab8a04-e225-4ff0-bb8b-c80d0e6ce50c",
    scryfall_id = "86a99edc-22d9-4653-8a7a-825f4b443d75",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Necrite",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::THRULL],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::AttacksAndIsntBlocked(&Filter::This),
        &[Effect::MayDo {
            effects: &[Effect::SacrificeSelfThen {
                then: &[Effect::Destroy {
                    target: TargetSpec::Object(&DEFENDERS_CREATURE),
                    no_regen: true,
                }],
            }],
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&DEFENDERS_CREATURE))),
    ),],
);
