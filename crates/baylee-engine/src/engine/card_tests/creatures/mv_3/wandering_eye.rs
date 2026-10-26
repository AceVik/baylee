//! `cards/creatures/mv_3/wandering_eye.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn wandering_eye_reveals_all_hands_while_on_battlefield() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[index::WANDERING_EYE])
        .battlefield(0, &[island(); 3])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert!(!engine.state().hand_revealed(p0));
    assert!(!engine.state().hand_revealed(p1));

    cast_from_hand(&mut engine, p0, index::WANDERING_EYE);
    pass_until(&mut engine, stack_is_empty);

    let eye = on_battlefield(&engine, p0, index::WANDERING_EYE).unwrap();
    assert!(keywords(&engine, eye).contains(KeywordSet::FLYING));
    assert!(engine.state().hand_revealed(p0));
    assert!(engine.state().hand_revealed(p1));

    bury(&mut engine, &[eye]);
    assert!(!engine.state().hand_revealed(p0));
    assert!(!engine.state().hand_revealed(p1));
}
