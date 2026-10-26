//! ICE effect consumers preserve draw events and independent delayed subjects.

use super::*;
use crate::worth::{Aim, Origin};
use baylee_cards_dsl::{Amount, DelayedObservation, Effect, Filter, TargetSpec};

#[test]
fn discarding_drawn_cards_prices_the_net_gain_but_counts_every_draw() {
    let mut v = view(0, &[20, 20], vec![]);
    let origin = Origin {
        id: obj(1),
        object: None,
    };
    let reader = agent();
    let effects = [Effect::DrawThenDiscardDrawn {
        draw: Amount::Fixed(2),
        discard: 1,
    }];
    assert_eq!(
        reader.effects_worth(&v, origin, &effects, Aim::Source, 0),
        Some(400),
    );
    let meaning = crate::tactics::meaning(&effects, 0);
    assert_eq!(meaning.draw, 2, "discarding does not undo a draw event");
    assert_eq!(meaning.value, 400);

    v.seats[0].library_count = 1;
    assert!(
        reader
            .effects_worth(&v, origin, &effects, Aim::Source, 0)
            .is_some_and(|worth| worth < -50_000),
        "netting one card still attempts two draws from a one-card library",
    );
}

#[test]
fn the_drawn_only_discard_cannot_charge_existing_hand_cards() {
    let v = view(0, &[20, 20], vec![]);
    let effects = [Effect::DrawThenDiscardDrawn {
        draw: Amount::Fixed(1),
        discard: 3,
    }];
    assert_eq!(
        agent().effects_worth(
            &v,
            Origin {
                id: obj(1),
                object: None,
            },
            &effects,
            Aim::Source,
            0,
        ),
        Some(0),
        "only the single newly drawn card can be discarded",
    );
    assert_eq!(crate::tactics::meaning(&effects, 0).draw, 1);
    assert_eq!(crate::tactics::meaning(&effects, 0).value, 0);
}

#[test]
fn an_observer_does_not_value_its_future_subject_as_the_current_target() {
    static OBSERVER_EFFECTS: &[Effect] = &[Effect::destroy(TargetSpec::EventObject)];
    let v = view(0, &[20, 20], vec![permanent(obj(1), PlayerId::new(0), 2)]);
    let effects = [Effect::CreateDelayedObserver {
        watch: TargetSpec::ThisObject,
        when: DelayedObservation::YouLoseControl,
        about: TargetSpec::Object(&Filter::CREATURE),
        effects: OBSERVER_EFFECTS,
    }];
    assert_eq!(
        agent().effects_worth(&v, Origin::of(&v, obj(1)), &effects, Aim::Source, 0),
        None,
    );
    let meaning = crate::tactics::meaning(&effects, 0);
    assert!(!meaning.removal);
    assert!(!meaning.destroy);
    assert_eq!(meaning.benefit, 0);
}

#[test]
fn required_solo_attackers_are_alternatives_and_optional_solo_cannot_displace_requirements() {
    let defender = Defender::Player(PlayerId::new(1));
    let ids = [obj(1), obj(2), obj(3)];
    for (required, alone, minimum, chosen) in [
        (
            ids[..2].to_vec(),
            ids[..2].to_vec(),
            1,
            vec![(ids[0], defender), (ids[1], defender)],
        ),
        (
            ids[1..].to_vec(),
            ids[..1].to_vec(),
            2,
            vec![(ids[0], defender)],
        ),
    ] {
        let pending = Pending::ChooseAttackers {
            player: PlayerId::new(0),
            attackers: ids.to_vec(),
            defenders: vec![defender],
            required: required.clone(),
            alone: alone.clone(),
            minimum_required: Some(minimum),
            limits: vec![],
        };
        let answer = PlayerAction::DeclareAttackers {
            attackers: crate::combat::obey_attack_rules(
                chosen,
                &required,
                &alone,
                Some(minimum),
                &[],
                &[defender],
            ),
        };
        assert_eq!(pending.answer_fault(&answer), None);
    }
}

#[test]
fn computed_life_amount_rounds_before_valuation_and_unknown_second_targets_stay_unknown() {
    use baylee_cards_dsl::{PlayerRel, TargetSlot};
    let mut v = view(0, &[20, 20], vec![permanent(obj(1), PlayerId::new(1), 2)]);
    let reader = agent();
    let origin = Origin {
        id: obj(9),
        object: None,
    };
    let divided = [Effect::GainLife {
        amount: Amount::divided(&Amount::LifeTotal(PlayerRel::You), 3, true),
    }];
    assert_eq!(
        reader.effects_worth(&v, origin, &divided, Aim::Source, 0),
        reader.effects_worth(
            &v,
            origin,
            &[Effect::GainLife {
                amount: Amount::Fixed(7)
            }],
            Aim::Source,
            0
        )
    );
    v.seats[0].life = -2;
    assert_eq!(
        reader.effects_worth(&v, origin, &divided, Aim::Source, 0),
        Some(0)
    );
    assert_eq!(
        reader.effects_worth(
            &v,
            origin,
            &[Effect::DestroyTargetSlots {
                targets: &[TargetSlot::Second],
                no_regen: false
            }],
            Aim::Object(obj(1)),
            0
        ),
        None
    );
    assert_eq!(
        reader.effects_worth(
            &v,
            origin,
            &[Effect::GainLifeFor {
                who: PlayerRel::SecondTargetPlayerOrController,
                amount: Amount::Fixed(3)
            }],
            Aim::Player(PlayerId::new(1)),
            0
        ),
        None
    );
}
