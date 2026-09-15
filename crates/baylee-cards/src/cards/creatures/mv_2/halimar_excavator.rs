//! Halimar Excavator — {1}{U} — Creature — Human Wizard Ally
//! Oracle: Whenever this creature or another Ally you control enters, target player mills X cards, where X is the number of Allies you control.
//! Set: WWK #29 — Worldwake | Scryfall ID: d147dce7-b2dd-426a-9ff7-843d50bb8b01 | Oracle ID: fd3e37c9-93bf-4f3e-a279-22afbffd8d43
// IMPLEMENTED — rally mill per Ally. "Target player" is a real target and
// the ability is not optional: with no other legal choice the controller
// has to point it at themselves.

use crate::filters::YOUR_ALLIES;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

static ALLIES_YOU: Filter =
    Filter::And(&[Filter::ControlledByYou, Filter::HasSubtype(creature::ALLY)]);

card!(
    index = index::HALIMAR_EXCAVATOR,
    oracle_id = "fd3e37c9-93bf-4f3e-a279-22afbffd8d43",
    scryfall_id = "d147dce7-b2dd-426a-9ff7-843d50bb8b01",
    faces = &[face!(
        name = "Halimar Excavator",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[creature::HUMAN, creature::WIZARD, creature::ALLY],
        power = Some(1),
        toughness = Some(3),
    )],
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::EntersBattlefield(&YOUR_ALLIES),
        &[Effect::Mill {
            amount: Amount::CountOf {
                filter: &ALLIES_YOU,
                zone: ZoneSel::Battlefield,
            },
            target: PlayerRel::Chosen,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer))
    )],
);
