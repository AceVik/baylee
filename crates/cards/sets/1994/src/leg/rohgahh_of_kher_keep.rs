//! Rohgahh of Kher Keep — {2}{B}{B}{R}{R} — Legendary Creature — Kobold
//! Oracle: At the beginning of your upkeep, you may pay {R}{R}{R}. If you don't, tap Rohgahh and all creatures named Kobolds of Kher Keep, then an opponent gains control of them.
//! Oracle: Creatures you control named Kobolds of Kher Keep get +2/+2.
//! Set: ME3 #172 — Masters Edition III | Scryfall ID: 1429cc3d-f6a2-4011-b716-5c3e0cf251d0 | Oracle ID: 44412804-aa69-4e62-b118-123741d91914
// IMPLEMENTED — upkeep tax; unpaid, taps Rohgahh and every Kobolds of Kher Keep, then a chosen opponent gains control of them. Anthem for your Kobolds.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static KOBOLDS_OF_KHER_KEEP: Filter = Filter::Named("Kobolds of Kher Keep");

static ROHGAHH_AND_KOBOLDS: Filter = Filter::Or(&[
    Filter::This,
    Filter::And(&[Filter::CREATURE, KOBOLDS_OF_KHER_KEEP]),
]);

card!(
    index = index::ROHGAHH_OF_KHER_KEEP,
    oracle_id = "44412804-aa69-4e62-b118-123741d91914",
    scryfall_id = "1429cc3d-f6a2-4011-b716-5c3e0cf251d0",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Rohgahh of Kher Keep",
        mana_cost = mana!("{2}{B}{B}{R}{R}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::KOBOLD],
        power = Some(5),
        toughness = Some(5),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::PlayerMayPayManaOr {
                player: PlayerRel::You,
                cost: mana!("{R}{R}{R}"),
                // Not targeted: the opponent is chosen on resolution, and the
                // creatures change hands whoever controls them or whether they
                // were tapped (2009-10-01 ruling).
                effect: &Effect::Sequence(&[
                    Effect::TapAll {
                        filter: &ROHGAHH_AND_KOBOLDS,
                    },
                    Effect::ChoosePlayerFrom {
                        among: PlayerRel::EachOpponent,
                    },
                    Effect::GainControlAll {
                        filter: &ROHGAHH_AND_KOBOLDS,
                        to: PlayerRel::Chosen,
                    },
                ]),
            }],
        ),
        static_ability!(
            Filter::And(&[Filter::YOUR_CREATURE, KOBOLDS_OF_KHER_KEEP]),
            Modifier::ModifyPT(2, 2)
        ),
    ],
);
