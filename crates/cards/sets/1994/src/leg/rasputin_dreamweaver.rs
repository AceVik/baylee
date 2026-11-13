//! Rasputin Dreamweaver — {4}{W}{U} — Legendary Creature — Human Wizard
//! Oracle: Rasputin enters with seven dream counters on it.
//! Oracle: Remove a dream counter from Rasputin: Add {C}.
//! Oracle: Remove a dream counter from Rasputin: Prevent the next 1 damage that would be dealt to Rasputin this turn.
//! Oracle: At the beginning of your upkeep, if Rasputin started the turn untapped, put a dream counter on it.
//! Oracle: Rasputin can't have more than seven dream counters on it.
//! Set: ME3 #170 — Masters Edition III | Scryfall ID: 78418809-f048-4611-88cb-369f427d9c44 | Oracle ID: fef452ee-9018-4175-bb79-476544e48433

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RASPUTIN_DREAMWEAVER,
    oracle_id = "fef452ee-9018-4175-bb79-476544e48433",
    scryfall_id = "78418809-f048-4611-88cb-369f427d9c44",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Rasputin Dreamweaver",
        mana_cost = mana!("{4}{W}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(4),
        toughness = Some(1),
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: counters::DREAM,
            amount: Amount::Fixed(7)
        }],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        activated!(
            cost!(RemoveCounterSelf {
                kind: counters::DREAM,
                n: 1
            }),
            &[Effect::mana(ManaColor::Colorless, 1)],
            mana_ability = true
        ),
        activated!(
            cost!(RemoveCounterSelf {
                kind: counters::DREAM,
                n: 1
            }),
            &[Effect::PreventNextDamage {
                target: TargetSpec::ThisObject,
                amount: Amount::Fixed(1)
            }]
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You
            },
            &[Effect::AddCounterSource {
                kind: counters::DREAM,
                amount: Amount::Fixed(1)
            }],
            condition = Some(Condition::SourceStartedTurnUntapped)
        ),
        static_ability!(
            Filter::This,
            Modifier::CounterMaximum {
                kind: counters::DREAM,
                maximum: 7
            }
        ),
    ],
);
