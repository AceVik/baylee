//! `cards/creatures/mv_5/gerrard_capashen.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn gerrard_capashen() -> CardIndex {
    card_index("27521e70-3a1f-4819-a111-c38216ffe7c3")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Gerrard Capashen (27521e70-3a1f-4819-a111-c38216ffe7c3) — `{3}{W}{W}` 3/4
/// Legendary Creature — Human Soldier:
/// - At the beginning of your upkeep, you gain 1 life for each card in target opponent's hand.
/// - `{3}{W}`: Tap target creature. Activate only if Gerrard Capashen is attacking.
#[test]
fn gerrard_capashen_gains_life_for_opponent_hand_size_at_upkeep_and_taps_while_attacking() {
    let lands = vec![plains(); 8];
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &lands)
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[gerrard_capashen()])
        .hand(1, &[forest(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, gerrard_capashen());
    pass_until(&mut engine, stack_is_empty);
    let gerrard = on_battlefield(&engine, P0, gerrard_capashen()).expect("Gerrard entered");
    let blocker = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    assert_eq!(pt(&engine, gerrard), (3, 4));

    // P1's turn draws 1 card (2 -> 3 cards in P1's hand).
    reach_their_main_phase(&mut engine, P1);

    // P0's next upkeep: target P1 -> gain 3 life (20 -> 23).
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    reach_main_phase(&mut engine, P0);
    assert_eq!(engine.state().players[0].life, 23);

    // `{3}{W}` tap ability (index 1) is NOT offered outside combat even with `{3}{W}` floating.
    let four_plains: Vec<ObjectId> = lands_of(&engine, P0).into_iter().take(4).collect();
    tap_mana_where(&mut engine, P0, |id| four_plains.contains(&id));
    assert!(
        !ability_offered(&engine, gerrard, 1),
        "cannot activate tap ability while Gerrard is not attacking"
    );

    // Declare Gerrard as attacker -> tap remaining 4 Plains for `{3}{W}` and tap P1's blocker.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(gerrard, Defender::Player(P1))],
            },
        )
        .unwrap();
    tap_all_mana(&mut engine, P0);
    assert!(ability_offered(&engine, gerrard, 1));
    activate(&mut engine, P0, gerrard_capashen(), 1);
    aim_at(&mut engine, P0, blocker);

    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(
        engine
            .state()
            .object(blocker)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
    assert_eq!(
        engine.state().players[1].life,
        17,
        "blocker was tapped before blockers"
    );
}
