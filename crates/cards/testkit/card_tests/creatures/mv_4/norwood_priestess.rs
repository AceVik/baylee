//! `cards/creatures/mv_4/norwood_priestess.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn norwood_priestess() -> CardIndex {
    card_index("902a7588-602c-456f-8250-c94f35e56d3a")
}

/// Before attackers are declared on your turn, `{T}` offers only green creature
/// cards in hand (`min == 0`) and puts the chosen one onto the battlefield untapped.
#[test]
fn norwood_priestess_puts_only_a_green_creature_from_hand_onto_the_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[norwood_priestess()])
        .hand(0, &[craw_wurm(), phantom_monster(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wurm = in_hand(&engine, p0, craw_wurm()).unwrap();
    let blue_creature = in_hand(&engine, p0, phantom_monster()).unwrap();
    let land = in_hand(&engine, p0, forest()).unwrap();
    let priestess = on_battlefield(&engine, p0, norwood_priestess()).unwrap();

    activate(&mut engine, p0, norwood_priestess(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        unreachable!("stopped on ChooseCards")
    };
    assert_eq!((min, max), (0, 1), "\"you may\" makes zero legal");
    assert!(options.contains(&wurm));
    assert!(
        !options.contains(&blue_creature),
        "non-green creature excluded"
    );
    assert!(!options.contains(&land), "green-producing land excluded");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let landed_wurm = on_battlefield(&engine, p0, craw_wurm()).expect("Craw Wurm entered");
    assert!(!is_tapped(&engine, landed_wurm));
    assert!(is_tapped(&engine, priestess));
    assert!(in_hand(&engine, p0, phantom_monster()).is_some());
}

/// Declining the optional choice leaves the green creature in hand, and the
/// ability cannot be activated once Declare Attackers begins or on opponent's turn.
#[test]
fn norwood_priestess_may_decline_and_is_restricted_to_own_turn_before_attackers() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[norwood_priestess(), norwood_priestess()])
        .hand(0, &[craw_wurm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let priestesses = all_on_battlefield(&engine, p0, norwood_priestess());
    let first = priestesses[0];
    let second = priestesses[1];

    // First Priestess activates in FirstMain and declines ("you may").
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: first,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_hand(&engine, p0, craw_wurm()).is_some());

    // Second Priestess is still offered in Beginning of Combat (before attackers).
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::CombatBegin && matches!(e.pending(), Pending::Priority { .. })
    });
    let Pending::Priority { legal, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(legal.abilities.contains(&(second, 0)));

    // Once attackers are declared (DeclareAttackers priority & SecondMain), withheld.
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && matches!(e.pending(), Pending::Priority { .. })
    });
    let Pending::Priority { legal, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(
        !legal.abilities.contains(&(second, 0)),
        "not after attackers"
    );

    // Opponent's main phase (even before opponent's attackers): withheld.
    reach_their_main_phase(&mut engine, p1);
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    let Pending::Priority { player, legal } = engine.pending() else {
        unreachable!()
    };
    assert_eq!(*player, p0);
    assert!(
        !legal.abilities.contains(&(second, 0)),
        "not on opponent's turn"
    );
}
