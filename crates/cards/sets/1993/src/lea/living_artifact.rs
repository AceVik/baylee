//! Living Artifact — {G} — Enchantment — Aura
//! Oracle: Enchant artifact
//! Oracle: Whenever you're dealt damage, put that many vitality counters on this Aura.
//! Oracle: At the beginning of your upkeep, you may remove a vitality counter from this Aura. If you do, you gain 1 life.
//! Set: 5ED #311 — Fifth Edition | Scryfall ID: 8af097b1-9eae-4bd6-8ee6-582cae57e970 | Oracle ID: 4ff9af56-ac18-4966-9e48-183e1ca1c2d0
// IMPLEMENTED — the damage trigger fires once for a step's combat damage and
// counts all of it (Trigger::PlayerDealtDamage, Amount::EventAmount); the
// upkeep's "you may remove … If you do" is the removal as a cost at the head
// of a MayDo (CR 118.12), asked only while a counter is there to remove.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LIVING_ARTIFACT,
    oracle_id = "4ff9af56-ac18-4966-9e48-183e1ca1c2d0",
    scryfall_id = "8af097b1-9eae-4bd6-8ee6-582cae57e970",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Living Artifact",
        mana_cost = mana!("{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::ARTIFACT)
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::ARTIFACT)))
        ),
        triggered!(
            Trigger::PlayerDealtDamage(PlayerRel::You),
            &[Effect::AddCounter {
                kind: counters::VITALITY,
                amount: Amount::EventAmount,
            }],
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::MayDo {
                effects: &[
                    Effect::RemoveCounterSelf {
                        kind: counters::VITALITY,
                        n: 1,
                    },
                    Effect::GainLife {
                        amount: Amount::Fixed(1),
                    },
                ],
            }],
        ),
    ],
);
