//! `cards/creatures/mv_2/mad_dog.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn mad_dog() -> CardIndex {
    card_index("097bce84-22b1-4d7b-b566-e7c23eb626b9")
}

/// Mad Dog survives its ETB turn (came under your control this turn) and
/// turns on which it attacked, but is sacrificed at your end step if it did neither.
#[test]
fn mad_dog_survives_etb_and_attack_turns_but_sacrifices_when_idle() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[mad_dog()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, mad_dog());
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(on_battlefield(&engine, p0, mad_dog()).is_some());

    advance_next_main(&mut engine, p1);
    assert!(
        on_battlefield(&engine, p0, mad_dog()).is_some(),
        "survives end step of turn it entered"
    );

    advance_next_main(&mut engine, p0);
    let dog = on_battlefield(&engine, p0, mad_dog()).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(dog, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    advance_next_main(&mut engine, p1);
    assert_eq!(lk::life(&engine, p1), 18);
    assert!(
        on_battlefield(&engine, p0, mad_dog()).is_some(),
        "survives end step because it attacked this turn"
    );

    advance_next_main(&mut engine, p0);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(p0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();
    advance_next_main(&mut engine, p1);
    assert!(
        on_battlefield(&engine, p0, mad_dog()).is_none(),
        "sacrificed at end step after an idle turn"
    );
    assert!(in_graveyard(&engine, p0, mad_dog()).is_some());
}

fn advance_next_main(engine: &mut Engine<RegistryLookup>, target: PlayerId) {
    let start_turn = engine.state().turn.number;
    loop {
        if engine.state().turn.number > start_turn
            && engine.state().turn.active == target
            && engine.state().turn.phase == Phase::FirstMain
            && at_rest(engine, target)
        {
            return;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected pending while advancing: {other:?}"),
        }
    }
}
