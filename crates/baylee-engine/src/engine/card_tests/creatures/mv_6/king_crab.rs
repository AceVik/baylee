#[allow(clippy::wildcard_imports)]
use super::*;

fn king_crab() -> CardIndex {
    card_index("24982bfe-5276-44a8-99a5-43b497ebf51f")
}

#[test]
fn king_crab_keeps_its_body_without_the_unsupported_library_ability() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[king_crab()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let crab = on_battlefield(&engine, p0, king_crab()).unwrap();
    if let Pending::Priority { legal, .. } = engine.pending() {
        assert!(
            !legal.abilities.iter().any(|(source, _)| *source == crab),
            "unsupported ability is unavailable"
        );
    }
    let _ = attack_and_collect_blocks(&mut engine, crab, p1);
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 16);
}
