//! Individual attack-group restrictions also constrain maximal attack requirements.
use super::*;
use baylee_cards_dsl::{Filter, Modifier};
use baylee_core::ids::Defender;
const SOLO: u32 = 7830;
const FIRST: u32 = 7831;
const SECOND: u32 = 7832;
static SOLO_RULES: &[AbilityDef] = &[baylee_cards_dsl::static_ability!(
    Filter::This,
    Modifier::AttacksAloneOnly
)];
static SOLO_REQUIRED: &[AbilityDef] = &[
    baylee_cards_dsl::static_ability!(Filter::This, Modifier::AttacksAloneOnly),
    baylee_cards_dsl::static_ability!(Filter::This, Modifier::AttacksEachCombat),
];
static REQUIRED: &[AbilityDef] = &[baylee_cards_dsl::static_ability!(
    Filter::This,
    Modifier::AttacksEachCombat
)];
fn fixture(required: bool) -> (Bench, [ObjectId; 3]) {
    let mut engine = bench(
        7830,
        vec![
            card(
                SOLO,
                creature_face("Lone fixture", "{0}", 2, 3),
                KeywordSet::EMPTY,
                if required { SOLO_REQUIRED } else { SOLO_RULES },
            ),
            card(
                FIRST,
                creature_face("First fixture", "{0}", 2, 3),
                KeywordSet::EMPTY,
                if required { REQUIRED } else { &[] },
            ),
            card(
                SECOND,
                creature_face("Second fixture", "{0}", 2, 3),
                KeywordSet::EMPTY,
                if required { REQUIRED } else { &[] },
            ),
        ],
        [Seat::with(&[SOLO, FIRST, SECOND]), Seat::with(&[])],
    );
    let ids = [SOLO, FIRST, SECOND].map(|card| the(&engine, ZoneLocation::Battlefield, card));
    to_main(&mut engine, me());
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    (engine, ids)
}
fn declare(ids: &[ObjectId]) -> PlayerAction {
    PlayerAction::DeclareAttackers {
        attackers: ids
            .iter()
            .map(|id| (*id, Defender::Player(them())))
            .collect(),
    }
}
#[test]
fn individual_alone_restriction_does_not_limit_unrelated_creatures() {
    let (mut engine, [solo, first, second]) = fixture(false);
    let Pending::ChooseAttackers {
        alone,
        minimum_required,
        ..
    } = engine.pending()
    else {
        panic!("attack declaration");
    };
    assert_eq!(alone, &[solo]);
    assert_eq!(*minimum_required, Some(0));
    assert!(engine.apply(me(), declare(&[solo, first])).is_err());
    engine.apply(me(), declare(&[first, second])).unwrap();
    let (mut engine, [solo, _, _]) = fixture(false);
    engine.apply(me(), declare(&[solo])).unwrap();
}
#[test]
fn required_attackers_maximize_requirements_without_impossible_alone_combination() {
    let (mut engine, [solo, first, second]) = fixture(true);
    let question = engine.pending().clone();
    let Pending::ChooseAttackers {
        required,
        minimum_required,
        ..
    } = &question
    else {
        panic!("attack declaration");
    };
    assert_eq!(required.len(), 3);
    assert_eq!(*minimum_required, Some(2));
    assert!(question.answer_fault(&declare(&[solo])).is_some());
    assert!(engine.apply(me(), declare(&[solo])).is_err());
    let fallback = crate::choice::timeout_answer(&question).unwrap();
    assert_eq!(question.answer_fault(&fallback), None);
    engine.apply(me(), fallback).unwrap();
    assert!(engine.state().combat.is_attacking(first));
    assert!(engine.state().combat.is_attacking(second));
    assert!(!engine.state().combat.is_attacking(solo));
}
