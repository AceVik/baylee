//! Innkeeper's Talent — {1}{G} — Enchantment — Class
//! Oracle: (Gain the next level as a sorcery to add its ability.)
//! Oracle: At the beginning of combat on your turn, put a +1/+1 counter on target creature you control.
//! Oracle: {G}: Level 2
//! Oracle: Permanents you control with counters on them have ward {1}.
//! Oracle: {3}{G}: Level 3
//! Oracle: If you would put one or more counters on a permanent or player, put twice that many of each of those kinds of counters on that permanent or player instead.
//! Set: BLB #180 — Bloomburrow | Scryfall ID: 941b0afc-0e8f-45f2-ae7f-07595e164611 | Oracle ID: 915b75d1-b5e3-4aca-a54c-beaccef0a2e4
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::INNKEEPER_S_TALENT,
    oracle_id = "915b75d1-b5e3-4aca-a54c-beaccef0a2e4",
    scryfall_id = "941b0afc-0e8f-45f2-ae7f-07595e164611",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Innkeeper's Talent",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::CLASS],
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::CombatBegin,
                whose: PlayerRel::You,
            },
            &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE))),
        ),
        activated!(
            cost!("{G}"),
            &[Effect::AddCounter {
                kind: CounterKind::Level,
                amount: Amount::Fixed(1),
            }],
            timing = ActivationTiming::SorcerySpeed,
            condition = Some(Condition::CountersOnSelfExactly(CounterKind::Level, 0)),
        ),
        static_ability!(
            Filter::And(&[Filter::ControlledByYou, Filter::HasAnyCounter]),
            Modifier::GrantTriggered {
                trigger: Trigger::Ward,
                effects: &[Effect::PlayerMayPayOr {
                    player: PlayerRel::ControllerOfTarget,
                    mana: Amount::Fixed(1),
                    effect: &Effect::CounterTargetSpellOrAbility,
                }],
                target: None,
            },
            condition = Some(Condition::CountersOnSelf(CounterKind::Level, 1)),
        ),
        activated!(
            cost!("{3}{G}"),
            &[Effect::AddCounter {
                kind: CounterKind::Level,
                amount: Amount::Fixed(1),
            }],
            timing = ActivationTiming::SorcerySpeed,
            condition = Some(Condition::CountersOnSelfExactly(CounterKind::Level, 1)),
        ),
        static_ability!(
            Filter::Any,
            Modifier::DoubleCountersPlacedByYou,
            condition = Some(Condition::CountersOnSelf(CounterKind::Level, 2)),
        ),
    ],
);
