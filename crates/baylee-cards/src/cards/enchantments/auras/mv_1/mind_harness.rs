//! Mind Harness — {U} — Enchantment — Aura
//! Oracle: Enchant red or green creature
//! Oracle: Cumulative upkeep {1} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: You control enchanted creature.
//! Set: MIR #78 — Mirage | Scryfall ID: 5bf17780-801d-4ab8-91f4-a803ede51395 | Oracle ID: bba30360-c54a-4906-8b2e-84bb1209c43d
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ENCHANT1: Filter = Filter::Or(&[
    Filter::And(&[
        Filter::CREATURE,
        Filter::HasColor(ColorSet::from_slice(&[Color::Red])),
    ]),
    Filter::And(&[
        Filter::CREATURE,
        Filter::HasColor(ColorSet::from_slice(&[Color::Green])),
    ]),
]);

card!(
    index = index::MIND_HARNESS,
    oracle_id = "bba30360-c54a-4906-8b2e-84bb1209c43d",
    scryfall_id = "5bf17780-801d-4ab8-91f4-a803ede51395",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mind Harness",
        mana_cost = mana!("{U}"),
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
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You
            },
            &[
                Effect::AddCounter {
                    kind: counters::AGE,
                    amount: Amount::Fixed(1)
                },
                Effect::PlayerMayPayOr {
                    player: PlayerRel::You,
                    mana: Amount::CountersOnSource(counters::AGE),
                    effect: &Effect::SacrificeSelf
                }
            ],
            condition = Some(Condition::SourceMatches(&Filter::InZone(
                ZoneRef::Battlefield
            )))
        ),
        static_ability!(Filter::AttachedToBySource, Modifier::GainControl),
    ],
);
