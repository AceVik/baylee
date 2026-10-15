//! `cards/creatures/artifacts/mv_4/pierce_strider.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn pierce_strider() -> CardIndex {
    card_index("e2bbf963-794d-4600-bb92-cb9d1d00205a")
}

/// Pierce Strider — {4} 3/3: "When this creature enters, target opponent
/// loses 3 life." Cast off four Forests, the opponent goes from 20 to 17 and
/// its controller stays at 20.
#[test]
fn pierce_strider_makes_the_opponent_lose_three() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 4])
        .hand(0, &[pierce_strider()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, pierce_strider());
    pass_until(&mut engine, |e| {
        stack_is_empty(e) || matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    if let Pending::ChooseTargets { .. } = engine.pending() {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![p1],
                },
            )
            .expect("the opponent is a legal target");
    }
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, pierce_strider()).is_some());
    assert_eq!(engine.state().players[1].life, 17);
    assert_eq!(engine.state().players[0].life, 20);
}
