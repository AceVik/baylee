//! Seasinger — {1}{U}{U} — Creature — Merfolk
//! Oracle: When you control no Islands, sacrifice this creature.
//! Oracle: You may choose not to untap this creature during your untap step.
//! Oracle: {T}: Gain control of target creature whose controller controls an Island for as long as you control this creature and this creature remains tapped.
//! Set: ME1 #49 — Masters Edition | Scryfall ID: 473c8a26-4971-4b17-bdef-4c8f8120360d | Oracle ID: 13745b92-1e30-4075-9733-5d00da11408c
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const ISLAND: Filter = Filter::HasSubtype(subtypes::land::ISLAND);
const NO_ISLANDS: Condition = Condition::ControlCountAtMost(&ISLAND, 0);
const CREATURE_WHOSE_CONTROLLER_CONTROLS_ISLAND: Filter =
    Filter::And(&[Filter::CREATURE, Filter::ControllerControls(&ISLAND)]);

card!(
    index = index::SEASINGER,
    oracle_id = "13745b92-1e30-4075-9733-5d00da11408c",
    scryfall_id = "473c8a26-4971-4b17-bdef-4c8f8120360d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Seasinger",
        mana_cost = mana!("{1}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::MERFOLK],
        power = Some(0),
        toughness = Some(1),
    ),],
    abilities = &[
        triggered!(Trigger::State(&NO_ISLANDS), &[Effect::SacrificeSelf]),
        static_ability!(Filter::This, Modifier::MayChooseNotToUntap),
        activated!(
            Cost::TAP,
            &[Effect::ModifyTargets {
                modifier: Modifier::GainControl,
                duration: Duration::WhileSourceTapped {
                    control: true,
                    power_limited: false,
                },
            }],
            target = Some(TargetSpec::Object(
                &CREATURE_WHOSE_CONTROLLER_CONTROLS_ISLAND
            )),
        ),
    ],
);
