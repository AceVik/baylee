//! `cards/creatures/mv_5/steam_catapult.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn steam_catapult() -> CardIndex {
    card_index("b6ea0c70-022a-43e4-aa40-3dc6a8b3b21e")
}

/// `{T}` destroys a tapped creature, rejects untapped creatures as targets,
/// and is offered only during your turn before attackers are declared.
#[test]
fn steam_catapult_destroys_only_tapped_creatures_before_attackers_on_your_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[steam_catapult(), steam_catapult()])
        .battlefield(1, &[grizzly_bears(), craw_wurm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let catapults = all_on_battlefield(&engine, p0, steam_catapult());
    let (first, second) = (catapults[0], catapults[1]);
    let untapped_bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let tapped_wurm = on_battlefield(&engine, p1, craw_wurm()).unwrap();
    engine
        .dev_state_mut(p0)
        .unwrap()
        .set_tapped(tapped_wurm, true);
    engine.refresh_offer();

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: first,
                ability_index: 0,
            },
        )
        .unwrap();
    let options = pass_until_targets(&mut engine, p0);
    assert_eq!(
        options,
        vec![tapped_wurm],
        "only the tapped creature is a legal target"
    );
    assert!(!options.contains(&untapped_bears));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![tapped_wurm],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, craw_wurm()).is_some());
    assert!(is_tapped(&engine, first));

    // Tap the remaining Grizzly Bears too, then verify timing restrictions on `second`.
    engine
        .dev_state_mut(p0)
        .unwrap()
        .set_tapped(untapped_bears, true);
    engine.refresh_offer();

    // Still offered in CombatBegin (before attackers are declared).
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::CombatBegin && matches!(e.pending(), Pending::Priority { .. })
    });
    let Pending::Priority { legal, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(legal.abilities.contains(&(second, 0)));

    // Withheld in SecondMain (after attackers) and on opponent's turn.
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && matches!(e.pending(), Pending::Priority { .. })
    });
    let Pending::Priority { legal, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(!legal.abilities.contains(&(second, 0)));

    reach_their_main_phase(&mut engine, p1);
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    let Pending::Priority { player, legal } = engine.pending() else {
        unreachable!()
    };
    assert_eq!(*player, p0);
    assert!(!legal.abilities.contains(&(second, 0)));
}
