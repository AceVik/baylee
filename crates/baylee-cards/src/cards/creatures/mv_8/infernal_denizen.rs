//! Infernal Denizen — {7}{B} — Creature — Demon
//! Oracle: At the beginning of your upkeep, sacrifice two Swamps. If you can't, tap this creature, and an opponent may gain control of a creature you control of their choice for as long as this creature remains on the battlefield.
//! Oracle: {T}: Gain control of target creature for as long as this creature remains on the battlefield.
//! Set: ICE #136 — Ice Age | Scryfall ID: b63ac9a6-aaa5-4659-97d1-c5f6b0d5ccfe | Oracle ID: 2f33aced-c737-4b3c-ba25-aff7b340c2d7
// NOT SUPPORTED: when you can't sacrifice two Swamps during your upkeep, Infernal Denizen taps (and sacrifices a single Swamp if you control one) without offering an opponent the option to gain control of a creature you control of their choice.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static YOUR_SWAMP: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::land::SWAMP),
    Filter::ControlledByYou,
]);

card!(
    index = index::INFERNAL_DENIZEN,
    oracle_id = "2f33aced-c737-4b3c-ba25-aff7b340c2d7",
    scryfall_id = "b63ac9a6-aaa5-4659-97d1-c5f6b0d5ccfe",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "when you can't sacrifice two Swamps during your upkeep, Infernal Denizen taps (and sacrifices a single Swamp if you control one) without offering an opponent the option to gain control of a creature you control of their choice",
    ),
    faces = &[face!(
        name = "Infernal Denizen",
        mana_cost = mana!("{7}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DEMON],
        power = Some(5),
        toughness = Some(7),
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::IfCondition {
                condition: Condition::ControlCount(&YOUR_SWAMP, 2),
                then: &[
                    Effect::SacrificeFilter {
                        who: PlayerRel::You,
                        filter: &Filter::HasSubtype(subtypes::land::SWAMP),
                    },
                    Effect::SacrificeFilter {
                        who: PlayerRel::You,
                        filter: &Filter::HasSubtype(subtypes::land::SWAMP),
                    },
                ],
                otherwise: &[
                    Effect::SacrificeFilter {
                        who: PlayerRel::You,
                        filter: &Filter::HasSubtype(subtypes::land::SWAMP),
                    },
                    Effect::TapSelf,
                ],
            }],
        ),
        activated!(
            Cost::TAP,
            &[Effect::ModifyTargets {
                modifier: Modifier::GainControl,
                duration: Duration::WhileSourceOnBattlefield,
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
        ),
    ],
);
