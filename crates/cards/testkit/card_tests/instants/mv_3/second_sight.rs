//! `cards/instants/mv_3/second_sight.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "1a4e948a-5a5e-4811-a6af-7ea93a051746"
fn second_sight() -> CardIndex {
    card_index("1a4e948a-5a5e-4811-a6af-7ea93a051746")
}

/// Second Sight ({2}{U}, Entwine {U}): Mode 1 reorders top 5 of your library;
/// Entwine ({2}{U}{U}) reorders BOTH target opponent's top 5 AND your own top 5.
#[test]
fn second_sight_reorders_own_library_and_both_libraries_when_entwined() {
    let mut engine = lk::game(SEED, &[], &[], &[second_sight(), second_sight()], &[]);

    // 1. Mode 1 (your library): reverse the top 5 so the 5th-from-top becomes top.
    let lib0_before = engine.state().zones.list(ZoneLocation::Library(P0)).clone();
    let expected_top0 = lib0_before[lib0_before.len() - 5];
    lk::float(&mut engine, P0, &[(ManaColor::Blue, 3)]);
    cast_with_floating(&mut engine, P0, second_sight());
    let m1 = choose_cast_kind(&engine, CastModeKind::Mode(1));
    engine.apply(P0, PlayerAction::ChooseMode(m1)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange { cards, .. } = engine.pending().clone() else {
        unreachable!();
    };
    assert_eq!(cards.len(), 5);
    let mut rev0 = cards.clone();
    rev0.reverse();
    engine
        .apply(P0, PlayerAction::Arrange { piles: vec![rev0] })
        .unwrap();
    lk::settle(&mut engine);
    assert_eq!(
        *engine
            .state()
            .zones
            .list(ZoneLocation::Library(P0))
            .last()
            .unwrap(),
        expected_top0,
        "reordered card is now on top of P0's library"
    );

    // 2. Entwine ({2}{U}{U}): reorders BOTH opponent's top 5 AND your own top 5.
    let lib1_before = engine.state().zones.list(ZoneLocation::Library(P1)).clone();
    let expected_top1 = lib1_before[lib1_before.len() - 5];
    lk::float(&mut engine, P0, &[(ManaColor::Blue, 4)]);
    cast_with_floating(&mut engine, P0, second_sight());
    let both = choose_cast_kind(&engine, CastModeKind::Modes(3));
    engine.apply(P0, PlayerAction::ChooseMode(both)).unwrap();
    match engine.pending().clone() {
        Pending::ChoosePlayer { .. } => {
            engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
        }
        Pending::ChooseTargets { .. } => {
            lk::aim(&mut engine, &[], &[P1]);
        }
        other => panic!("expected opponent target prompt, got {other:?}"),
    }

    let mut arranged = 0;
    for _ in 0..20 {
        if stack_is_empty(&engine) && matches!(engine.pending(), Pending::Priority { .. }) {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::Arrange { player, cards, .. } => {
                assert_eq!(player, P0);
                assert_eq!(cards.len(), 5);
                let mut rev = cards;
                rev.reverse();
                engine
                    .apply(P0, PlayerAction::Arrange { piles: vec![rev] })
                    .unwrap();
                arranged += 1;
            }
            other => panic!("unexpected during entwined Second Sight: {other:?}"),
        }
    }
    assert_eq!(arranged, 2, "entwined Second Sight arranged both libraries");
    assert_eq!(
        *engine
            .state()
            .zones
            .list(ZoneLocation::Library(P1))
            .last()
            .unwrap(),
        expected_top1,
        "opponent's top 5 was reordered"
    );
}
