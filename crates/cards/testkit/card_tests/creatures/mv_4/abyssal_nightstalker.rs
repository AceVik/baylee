//! `cards/creatures/mv_4/abyssal_nightstalker.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn abyssal_nightstalker() -> CardIndex {
    card_index("10733767-1c97-4e2e-b02b-54bf346f6583")
}

/// Unblocked attack triggers defending player to discard one card, and still
/// deals its 2 combat damage; controller's hand is untouched.
#[test]
fn abyssal_nightstalker_makes_defending_player_discard_when_unblocked() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[abyssal_nightstalker(), swamp()])
        .hand(1, &[grizzly_bears(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, abyssal_nightstalker());
    pass_until(&mut engine, stack_is_empty);
    let stalker = on_battlefield(&engine, p0, abyssal_nightstalker()).expect("resolved");
    assert_eq!(pt(&engine, stalker), (2, 2));

    // Advance a full turn cycle so summoning sickness clears (CR 302.6).
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let p0_hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let p1_hand_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();
    assert!(p1_hand_before >= 2);

    let _ = attack_and_collect_blocks(&mut engine, stalker, p1);
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("stopped on ChooseCards")
    };
    assert_eq!((player, min, max), (p1, 1, 1));
    let discarded = in_hand(&engine, p1, grizzly_bears()).expect("Bears in p1 hand");
    assert!(options.contains(&discarded));
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![discarded],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        p1_hand_before - 1
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        p0_hand_before,
        "attacking controller does not discard"
    );
    assert_eq!(engine.state().players[1].life, 18);
}

/// Being blocked prevents "attacks and isn't blocked" from triggering.
#[test]
fn abyssal_nightstalker_does_not_trigger_when_blocked() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[abyssal_nightstalker()])
        .battlefield(1, &[grizzly_bears()])
        .hand(1, &[forest(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let stalker = on_battlefield(&engine, p0, abyssal_nightstalker()).unwrap();
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let p1_hand_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();

    let _ = attack_and_collect_blocks(&mut engine, stalker, p1);
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, stalker)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        p1_hand_before,
        "a blocked Abyssal Nightstalker never asks for a discard"
    );
    assert_eq!(engine.state().players[1].life, 20);
}
