//! Infinite Authority — {W}{W}{W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Whenever enchanted creature blocks or becomes blocked by a creature with toughness 3 or less, destroy the other creature at end of combat. At the beginning of the next end step, if that creature was destroyed this way, put a +1/+1 counter on the first creature.
//! Set: LEG #22 — Legends | Scryfall ID: dc60077f-d577-4a6c-a78f-697317024c40 | Oracle ID: b4faba1a-23db-4678-9ce6-a7816105f22a
// IMPLEMENTED — the Aura's own trigger destroys the other creature at end of combat and, if it died, puts a +1/+1 counter on the enchanted creature at the next end step.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::INFINITE_AUTHORITY,
    oracle_id = "b4faba1a-23db-4678-9ce6-a7816105f22a",
    scryfall_id = "dc60077f-d577-4a6c-a78f-697317024c40",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Infinite Authority",
        mana_cost = mana!("{W}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE)
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        // The Aura's own trigger (CR 113.1a, 603.3a): the other creature of the
        // pair is the event object, which `AtEndOfCombat` remembers for the
        // destroy (CR 603.7c); the enchanted ("first") creature is the trigger's
        // subject, which the end-step counter reads as the reference, so moving
        // the Aura afterwards changes nothing (2006-10-15 ruling).
        triggered!(
            Trigger::AttachedBlocksOrBecomesBlockedBy(&Filter::ToughnessAtMost(3)),
            &[Effect::AtEndOfCombat {
                about: TargetSpec::EventObject,
                effects: &[
                    Effect::destroy(TargetSpec::EventObject),
                    // "if that creature was destroyed this way": it died.
                    Effect::IfCondition {
                        condition: Condition::AmountAtLeast(
                            Amount::ThisWay(ThisWay::Died(&Filter::Any)),
                            1
                        ),
                        then: &[Effect::AtNextEndStepAboutSubject {
                            effects: &[Effect::AddCounterFilter {
                                filter: &Filter::Reference,
                                kind: CounterKind::P1P1,
                                amount: Amount::Fixed(1),
                            }],
                        }],
                        otherwise: &[],
                    },
                ],
            }]
        ),
    ],
);
