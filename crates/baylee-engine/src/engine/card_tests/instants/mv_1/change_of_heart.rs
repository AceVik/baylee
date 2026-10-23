//! `cards/instants/mv_1/change_of_heart.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn change_of_heart() -> CardIndex {
    card_index("a8c0b41b-916c-4da9-9c39-a058c91a78f4")
}

/// Change of Heart (`a8c0b41b-916c-4da9-9c39-a058c91a78f4`) — {W} Instant (Partial):
/// "Target creature can't attack this turn."
#[test]
fn change_of_heart_prevents_target_creature_from_attacking() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), index::GRIZZLY_BEARS, index::SAVANNAH_LIONS])
        .hand(0, &[change_of_heart()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, P0, change_of_heart());
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, P0, change_of_heart()).is_some(),
        "base spell does not get free buyback"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        panic!("expected ChooseAttackers");
    };
    assert!(!attackers.contains(&bears));
    let lions = on_battlefield(&engine, P0, index::SAVANNAH_LIONS).unwrap();
    assert!(attackers.contains(&lions));
    engine
        .apply(P0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();
    reach_their_main_phase(&mut engine, lk::P1);
    reach_their_main_phase(&mut engine, P0);
    assert!(lk::attackers_offered(&mut engine).contains(&bears));
}
