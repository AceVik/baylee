//! `cards/creatures/mv_5/cao_cao_lord_of_wei.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn cao_cao_lord_of_wei() -> CardIndex {
    card_index("81a98b14-e827-4241-a401-64890def51af")
}

/// Cao Cao, Lord of Wei (81a98b14-e827-4241-a401-64890def51af) — `{3}{B}{B}` 3/3
/// Legendary Creature — Human Soldier:
/// "`{T}`: Target opponent discards two cards. Activate only during your turn,
/// before attackers are declared."
#[test]
fn cao_cao_lord_of_wei_forces_target_opponent_to_discard_two_cards_before_attackers() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[cao_cao_lord_of_wei()])
        .hand(1, &[forest(), island(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, cao_cao_lord_of_wei());
    pass_until(&mut engine, stack_is_empty);

    let cao_cao = on_battlefield(&engine, P0, cao_cao_lord_of_wei()).expect("Cao Cao entered");
    assert_eq!(pt(&engine, cao_cao), (3, 3));

    reach_their_main_phase(&mut engine, P1);
    assert!(!ability_offered(&engine, cao_cao, 0));

    reach_their_main_phase(&mut engine, P0);
    let pitch_a = in_hand(&engine, P1, forest()).unwrap();
    let pitch_b = in_hand(&engine, P1, island()).unwrap();
    let keep = in_hand(&engine, P1, plains()).unwrap();

    activate(&mut engine, P0, cao_cao_lord_of_wei(), 0);
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
        panic!("expected ChooseCards, got {:?}", engine.pending());
    };
    assert_eq!(player, P1);
    assert_eq!((min, max), (2, 2));
    assert!(options.contains(&keep));

    engine
        .apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![pitch_a, pitch_b],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P1, forest()).is_some());
    assert!(in_graveyard(&engine, P1, island()).is_some());
    assert!(in_hand(&engine, P1, plains()).is_some());
}

#[test]
fn cao_cao_lord_of_wei_cannot_activate_in_second_main_phase() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[cao_cao_lord_of_wei()])
        .hand(1, &[forest(), island()])
        .start();
    keep_mulligans(&mut engine);
    let cao_cao = on_battlefield(&engine, P0, cao_cao_lord_of_wei()).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(P0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(!ability_offered(&engine, cao_cao, 0));
}
