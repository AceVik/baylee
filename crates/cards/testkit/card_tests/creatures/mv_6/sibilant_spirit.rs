//! `cards/creatures/mv_6/sibilant_spirit.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn sibilant_spirit() -> CardIndex {
    card_index("9798d307-a44a-4f44-94c7-03e499b94962")
}

/// Sibilant Spirit (9798d307-a44a-4f44-94c7-03e499b94962) — {5}{U} 5/6 Creature — Spirit:
/// - "Flying"
/// - "Whenever this creature attacks, defending player may draw a card."
#[test]
fn sibilant_spirit_lets_defending_player_accept_or_decline_drawing_a_card_on_attack() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[sibilant_spirit()])
        .start();
    keep_mulligans(&mut engine);

    let spirit = on_battlefield(&engine, P0, sibilant_spirit()).unwrap();
    assert_eq!(pt(&engine, spirit), (5, 6));
    assert!(keywords(&engine, spirit).contains(KeywordSet::FLYING));

    let p0_hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    let p1_hand_before = engine.state().zones.list(ZoneLocation::Hand(P1)).len();

    // 1. First attack (turn 1): defending player (P1) accepts the draw -> P1 draws 1 card, P0 does not.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(spirit, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == P1),
    );
    engine.apply(P1, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P1)).len(),
        p1_hand_before + 1,
        "defending player drew 1 card when accepting"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        p0_hand_before,
        "attacking player did not draw from Sibilant Spirit"
    );
    assert_eq!(engine.state().players[1].life, 15);

    // 2. Second attack (turn 3): defending player (P1) declines the draw -> P1's hand size is unchanged.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3
            && matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0)
    });
    let p1_hand_turn3 = engine.state().zones.list(ZoneLocation::Hand(P1)).len();
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(spirit, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == P1),
    );
    engine.apply(P1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P1)).len(),
        p1_hand_turn3,
        "defending player drew no card when declining"
    );
    assert_eq!(engine.state().players[1].life, 10);
}
