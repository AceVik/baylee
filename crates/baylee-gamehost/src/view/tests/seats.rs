use super::*;

/// A seat in the game carries no loss; a seat that is out carries the
/// reason the engine recorded, and the view's `has_lost` reads it.
#[test]
fn a_lost_seat_carries_the_engines_reason_and_a_live_one_carries_none() {
    let preset = mixed_print_preset();
    let mut engine = Engine::new(&preset, Registry).expect("game starts");
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let view = player_view(engine.state(), me, 0, None, &SeatContext::default(), &[]);
    assert!(view.seats.iter().all(|s| s.loss.is_none() && !s.has_lost()));

    engine
        .apply(them, baylee_engine::choice::PlayerAction::Concede)
        .expect("a seated player may always concede");
    let view = player_view(engine.state(), me, 1, None, &SeatContext::default(), &[]);
    let seat = |p: PlayerId| view.seat(p).expect("seated");
    assert_eq!(seat(them).loss, Some(LossCause::Conceded));
    assert!(seat(them).has_lost());
    assert_eq!(seat(me).loss, None);
}

/// Each engine reason reaches the wire under its own name. The mapping is
/// an exhaustive match, so a new reason cannot be forgotten; this is what
/// catches two arms swapped.
#[test]
fn every_loss_reason_reaches_the_wire_as_itself() {
    for reason in [
        LossReason::Life,
        LossReason::EmptyDraw,
        LossReason::Poison,
        LossReason::CommanderDamage,
        LossReason::Conceded,
        LossReason::Effect,
    ] {
        assert_eq!(format!("{:?}", loss_cause(reason)), format!("{reason:?}"));
    }
}

/// Who answered for a seat is on that seat and no other, and a seat the
/// host's record does not reach reads as having answered itself.
#[test]
fn a_house_answer_is_on_the_seat_the_host_names_and_no_other() {
    let preset = mixed_print_preset();
    let engine = Engine::new(&preset, Registry).expect("game starts");
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let ctx = SeatContext::default();
    let view = player_view(
        engine.state(),
        me,
        0,
        None,
        &ctx,
        &[None, Some(HouseAnswer::Clock)],
    );
    assert_eq!(
        view.seat(them).map(|s| s.house_answered),
        Some(Some(HouseAnswer::Clock))
    );
    assert_eq!(view.seat(me).map(|s| s.house_answered), Some(None));

    let short = player_view(
        engine.state(),
        me,
        0,
        None,
        &ctx,
        &[Some(HouseAnswer::StandIn)],
    );
    assert_eq!(
        short.seat(me).map(|s| s.house_answered),
        Some(Some(HouseAnswer::StandIn))
    );
    assert_eq!(short.seat(them).map(|s| s.house_answered), Some(None));
}

/// What a seat's own policies answered for it is told to that seat and
/// no other (#234), for `priority_held`'s reason: the host hands every
/// seat's row in, and the view picks the one it is built for.
#[test]
fn a_seat_is_told_what_its_own_policies_answered_and_no_other_seats() {
    let preset = mixed_print_preset();
    let engine = Engine::new(&preset, Registry).expect("game starts");
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let act = |number, card| PolicyAct {
        number,
        ability: baylee_view::LogAbility {
            ability: Some(baylee_core::ids::AbilityRef::new(CardIndex::new(card), 0)),
            text: None,
            rules: None,
        },
        answer: baylee_view::PolicyAnswer::Passed,
    };
    let table = [vec![act(1, 7)], vec![act(1, 8), act(2, 8)]];
    let ctx = SeatContext {
        policy_acts: &table,
        ..SeatContext::default()
    };
    let told = |seat| player_view(engine.state(), seat, 0, None, &ctx, &[]).policy_acts;
    assert_eq!(told(me), [act(1, 7)]);
    assert_eq!(told(them), [act(1, 8), act(2, 8)]);

    let short = SeatContext {
        policy_acts: &table[..1],
        ..SeatContext::default()
    };
    let told = player_view(engine.state(), them, 0, None, &short, &[]).policy_acts;
    assert!(
        told.is_empty(),
        "a seat the table does not reach is told nothing"
    );
}
