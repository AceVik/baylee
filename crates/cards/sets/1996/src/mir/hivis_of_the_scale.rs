//! Hivis of the Scale — {3}{R}{R} — Legendary Creature — Lizard Shaman
//! Oracle: You may choose not to untap Hivis during your untap step.
//! Oracle: {T}: Gain control of target Dragon for as long as you control Hivis and Hivis remains tapped.
//! Set: MIR #182 — Mirage | Scryfall ID: ee84e61e-d99a-489b-a3b1-cb45fc81bce6 | Oracle ID: 797adc7e-03a9-4221-bdaf-4c46dcb95701
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HIVIS_OF_THE_SCALE,
    oracle_id = "797adc7e-03a9-4221-bdaf-4c46dcb95701",
    scryfall_id = "ee84e61e-d99a-489b-a3b1-cb45fc81bce6",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Hivis of the Scale",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::LIZARD, subtypes::creature::SHAMAN],
        power = Some(3),
        toughness = Some(4),
    ),],
    abilities = &[
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
            target = Some(TargetSpec::Object(&Filter::HasSubtype(
                subtypes::creature::DRAGON,
            ))),
        ),
    ],
);
