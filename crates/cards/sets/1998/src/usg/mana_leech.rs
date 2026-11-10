//! Mana Leech — {2}{B} — Creature — Leech
//! Oracle: You may choose not to untap this creature during your untap step.
//! Oracle: {T}: Tap target land. It doesn't untap during its controller's untap step for as long as this creature remains tapped.
//! Set: USG #141 — Urza's Saga | Scryfall ID: 8a288616-5350-43eb-b718-5bfeb5be4ed4 | Oracle ID: cad05528-917b-4296-9d23-ba9433612709
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MANA_LEECH,
    oracle_id = "cad05528-917b-4296-9d23-ba9433612709",
    scryfall_id = "8a288616-5350-43eb-b718-5bfeb5be4ed4",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mana Leech",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LEECH],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        static_ability!(Filter::This, Modifier::MayChooseNotToUntap),
        activated!(
            Cost::TAP,
            &[
                Effect::TapTarget,
                Effect::ModifyTargets {
                    modifier: Modifier::DoesNotUntap,
                    duration: Duration::WhileSourceTapped {
                        control: false,
                        power_limited: false,
                    },
                },
            ],
            target = Some(TargetSpec::Object(&Filter::LAND)),
        ),
    ],
);
