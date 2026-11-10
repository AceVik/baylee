//! Decomposition — {1}{G} — Enchantment — Aura
//! Oracle: Enchant black creature
//! Oracle: Enchanted creature has "Cumulative upkeep—Pay 1 life." (At the beginning of its controller's upkeep, that player puts an age counter on it, then sacrifices it unless they pay its upkeep cost for each age counter on it.)
//! Oracle: When enchanted creature dies, its controller loses 2 life.
//! Set: MIR #212 — Mirage | Scryfall ID: b0ea46a1-ea65-4802-812c-4f0e0f3088d2 | Oracle ID: 62817ef5-9b8d-4bca-bdd4-ab3bc4e063d0
// IMPLEMENTED — enchants a black creature, grants cumulative upkeep—Pay 1 life, and its controller loses 2 life when it dies.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ENCHANT1: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::Black])),
]);

card!(
    index = index::DECOMPOSITION,
    oracle_id = "62817ef5-9b8d-4bca-bdd4-ab3bc4e063d0",
    scryfall_id = "b0ea46a1-ea65-4802-812c-4f0e0f3088d2",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Decomposition",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&ENCHANT1)
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&ENCHANT1)))
        ),
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::GrantTriggered {
                trigger: Trigger::StepBegin {
                    step: StepKind::Upkeep,
                    whose: PlayerRel::You
                },
                effects: &[
                    Effect::AddCounter {
                        kind: counters::AGE,
                        amount: Amount::Fixed(1)
                    },
                    Effect::PlayerMayPayLifeOr {
                        player: PlayerRel::You,
                        life: Amount::CountersOnSource(counters::AGE),
                        effect: &Effect::SacrificeSelf
                    }
                ],
                target: None
            }
        ),
        triggered!(
            Trigger::Dies(&Filter::AttachedToBySource),
            &[Effect::LoseLife {
                amount: Amount::Fixed(2),
                target: PlayerRel::ControllerOfEvent
            }]
        ),
    ],
);
