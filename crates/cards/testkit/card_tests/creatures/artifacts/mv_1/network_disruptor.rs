//! `cards/creatures/artifacts/mv_1/network_disruptor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn network_disruptor() -> CardIndex {
    card_index("0d64d150-1716-4da4-9cfe-ee7749b89918")
}

/// "Flying. When this creature enters, tap target permanent." Any permanent
/// is a target: a land of the opponent's is on the menu beside the creature,
/// and the one chosen ends tapped.
#[test]
fn network_disruptor_taps_a_target_permanent_when_it_enters() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island()])
        .battlefield(1, &[gray_ogre(), forest()])
        .hand(0, &[network_disruptor()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("ogre");
    let land = on_battlefield(&engine, p1, forest()).expect("forest");

    cast_from_hand(&mut engine, p0, network_disruptor());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let options = aim_at(&mut engine, p0, ogre);
    assert!(options.contains(&land), "a land is a permanent too");
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, ogre));
    assert!(!is_tapped(&engine, land), "only the target");
    let me = on_battlefield(&engine, p0, network_disruptor()).expect("seated");
    assert!(keywords(&engine, me).contains(KeywordSet::FLYING));
    assert_eq!(pt(&engine, me), (1, 1));
}
