//! Infinite Authority — {W}{W}{W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Whenever enchanted creature blocks or becomes blocked by a creature with toughness 3 or less, destroy the other creature at end of combat. At the beginning of the next end step, if that creature was destroyed this way, put a +1/+1 counter on the first creature.
//! Set: LEG #22 — Legends | Scryfall ID: dc60077f-d577-4a6c-a78f-697317024c40 | Oracle ID: b4faba1a-23db-4678-9ce6-a7816105f22a
// PARTIAL — written and played correctly in every ordinary case, but the trigger
// is granted to the enchanted creature instead of being the Aura's own ability.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::INFINITE_AUTHORITY,
    oracle_id = "b4faba1a-23db-4678-9ce6-a7816105f22a",
    scryfall_id = "dc60077f-d577-4a6c-a78f-697317024c40",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "the trigger is granted to the enchanted creature instead of being the Aura's own: a lose-all-abilities effect on the creature silences it (Final Showdown, Oko) and the creature's controller controls it (CR 603.3a); needs a pair trigger about the enchanted creature that remembers it for the end-step counter"
    ),
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
        // NOT SUPPORTED: the trigger is the Aura's own ability (CR 113.1a), so
        // the Aura's controller controls it (CR 603.3a) and a lose-all-abilities
        // effect on the creature does not silence it. `Modifier::GrantTriggered`
        // gives it to the creature instead; the engine has no "enchanted creature
        // blocks or becomes blocked by" trigger that keeps the enchanted creature
        // as a recorded subject for the end-step counter.
        // The trigger belongs to the enchanted creature, so it is granted to
        // it: the other creature of the block pair is the trigger's event
        // object, and `AtEndOfCombat` remembers that exact object for its
        // destroy (CR 603.7c). The granted trigger's source is the enchanted
        // ("first") creature, so the delayed end-step trigger created inside
        // the destroy's resolution puts the counter on that creature; moving
        // the Aura afterwards changes nothing (2006-10-15 ruling).
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::GrantTriggered {
                trigger: Trigger::BlocksOrBecomesBlockedBy(&Filter::ToughnessAtMost(3)),
                effects: &[Effect::AtEndOfCombat {
                    about: TargetSpec::EventObject,
                    effects: &[
                        Effect::destroy(TargetSpec::EventObject),
                        // "if that creature was destroyed this way": it died.
                        Effect::IfCondition {
                            condition: Condition::AmountAtLeast(
                                Amount::ThisWay(ThisWay::Died(&Filter::Any)),
                                1
                            ),
                            then: &[Effect::AtNextEndStep {
                                effects: &[Effect::AddCounterSource {
                                    kind: CounterKind::P1P1,
                                    amount: Amount::Fixed(1),
                                }],
                            }],
                            otherwise: &[],
                        },
                    ],
                }],
                target: None,
            }
        ),
    ],
);
