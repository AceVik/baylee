//! Vesuvan Doppelganger — {3}{U}{U} — Creature — Shapeshifter
//! Oracle: You may have this creature enter as a copy of any creature on the battlefield, except it doesn't copy that creature's color and it has "At the beginning of your upkeep, you may have this creature become a copy of target creature, except it doesn't copy that creature's color and it has this ability."
//! Set: ME1 #54 — Masters Edition | Scryfall ID: 543c08bc-f8ce-4324-b78d-891c49f3a24a | Oracle ID: aeaccab9-3e2c-4a40-a483-52c4972b2014
// PARTIAL — composable copy text awaits independent and live acceptance.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// A finite definition: "this ability" is captured from the resolving stack
// entry, rather than a cyclic DSL reference (CR 707.9a).
static UPKEEP: AbilityDef = triggered!(
    Trigger::StepBegin {
        step: StepKind::Upkeep,
        whose: PlayerRel::You
    },
    &[Effect::MayDo {
        effects: &[Effect::BecomeCopyOfTarget {
            mods: &[CopyMod::KeepColor, CopyMod::KeepResolvingAbility],
        }],
    }],
    targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
);

card!(
    index = index::VESUVAN_DOPPELGANGER,
    oracle_id = "aeaccab9-3e2c-4a40-a483-52c4972b2014",
    scryfall_id = "543c08bc-f8ce-4324-b78d-891c49f3a24a",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial("composable copy text awaits independent and live acceptance"),
    faces = &[face!(
        name = "Vesuvan Doppelganger",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SHAPESHIFTER],
        power = Some(0),
        toughness = Some(0),
    ),],
    abilities = &[AbilityDef::CopyOnEnter {
        target: TargetSpec::Object(&Filter::CREATURE),
        mods: &[CopyMod::KeepColor, CopyMod::GrantAbility(&UPKEEP)]
    },],
);
