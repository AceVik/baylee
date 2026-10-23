//! `cards/creatures/mv_4/walking_dream.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn walking_dream() -> CardIndex {
    card_index("286e7ef1-d763-4b48-8603-a161dcf7efbc")
}

/// Walking Dream (`286e7ef1-d763-4b48-8603-a161dcf7efbc`) — {3}{U} Creature — Illusion (Implemented):
/// "This creature can't be blocked. This creature doesn't untap during your untap step if an opponent controls two or more creatures."
#[test]
fn walking_dream_is_unblockable_and_stays_tapped_when_opponent_has_two_creatures() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[walking_dream()])
        .battlefield(1, &[index::GRIZZLY_BEARS, index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let dream = on_battlefield(&engine, P0, walking_dream()).unwrap();
    kit::declare_attack(&mut engine, &[dream], P1);
    assert!(
        entered_tapped(&engine, dream),
        "Walking Dream tapped to attack"
    );

    // Advance to P0's next main phase
    kit::walk_to_next_own_main(&mut engine, P0);

    assert!(
        entered_tapped(&engine, dream),
        "Walking Dream should not untap when an opponent controls two or more creatures"
    );
}
