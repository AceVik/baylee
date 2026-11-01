//! `cards/instants/mv_3/crash.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn subject_card() -> CardIndex {
    card_index("562af5da-1080-46d2-8a38-71f06b7e8457")
}

/// Crash — {2}{R} Instant:
/// "Destroy target artifact." (Partial: sacrifice-a-Mountain alternative cost unimplemented).
#[test]
fn crash_destroys_target_artifact() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .battlefield(1, &[index::JAYEMDAE_TOME])
        .hand(0, &[subject_card()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let tome = on_battlefield(&engine, P1, index::JAYEMDAE_TOME).unwrap();
    cast_from_hand(&mut engine, P0, subject_card());
    aim_at(&mut engine, P0, tome);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, index::JAYEMDAE_TOME).is_none());
    assert!(in_graveyard(&engine, P1, index::JAYEMDAE_TOME).is_some());
}
