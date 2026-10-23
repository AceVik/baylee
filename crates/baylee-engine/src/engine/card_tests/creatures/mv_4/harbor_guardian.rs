//! `cards/creatures/mv_4/harbor_guardian.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn harbor_guardian() -> CardIndex {
    card_index("d72b69aa-fb61-441e-9b62-4534d7a32a69")
}

/// Harbor Guardian (d72b69aa-fb61-441e-9b62-4534d7a32a69) — {2}{W}{U} 3/4:
/// "Reach. Whenever this creature attacks, defending player may draw a card."
#[test]
fn harbor_guardian_has_reach_and_lets_defending_player_draw_when_attacking() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[harbor_guardian()])
        .start();
    keep_mulligans(&mut engine);

    let guardian = on_battlefield(&engine, P0, harbor_guardian()).unwrap();
    assert!(
        engine
            .state()
            .object(guardian)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::REACH)
    );
    let before_hand = engine.state().zones.list(ZoneLocation::Hand(P1)).len();

    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(guardian, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P1)).len(),
        before_hand + 1
    );
}

/// P0 attacks with Harbor Guardian; the game stops on the question put to the
/// defending player. Returns the engine and the two hand sizes before it.
fn guardian_attacks() -> (Engine<RegistryLookup>, usize, usize) {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[harbor_guardian()])
        .start();
    keep_mulligans(&mut engine);

    let guardian = on_battlefield(&engine, P0, harbor_guardian()).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let mine = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    let theirs = engine.state().zones.list(ZoneLocation::Hand(P1)).len();
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(guardian, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    (engine, mine, theirs)
}

/// "defending player may draw": the question is put to P1, not the attacker,
/// and a yes draws one card for P1 and none for P0.
#[test]
fn harbor_guardian_lets_the_defending_player_draw_on_a_yes() {
    let (mut engine, mine, theirs) = guardian_attacks();
    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(player, P1);

    engine.apply(P1, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P1)).len(),
        theirs + 1
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        mine
    );
}

/// "may": a no draws nothing for anyone.
#[test]
fn harbor_guardian_draws_nothing_on_a_no() {
    let (mut engine, mine, theirs) = guardian_attacks();

    engine.apply(P1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P1)).len(),
        theirs
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        mine
    );
}
