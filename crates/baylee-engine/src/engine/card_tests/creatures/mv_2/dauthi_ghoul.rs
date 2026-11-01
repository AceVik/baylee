//! `cards/creatures/mv_2/dauthi_ghoul.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn dauthi_ghoul() -> CardIndex {
    card_index("5bf39f9e-4f06-4637-b0fc-5d21b926f6f4")
}

/// Dauthi Ghoul (`5bf39f9e-4f06-4637-b0fc-5d21b926f6f4`) — {1}{B} Creature — Dauthi Zombie:
/// "Shadow. Whenever a creature with shadow dies, put a +1/+1 counter on this creature."
#[test]
fn dauthi_ghoul_grows_only_when_a_shadow_creature_dies() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                dauthi_ghoul(),
                index::DAUTHI_MINDRIPPER,
                index::GRIZZLY_BEARS,
                index::MOUNTAIN,
                index::MOUNTAIN,
            ],
        )
        .hand(0, &[index::LIGHTNING_BOLT, index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let ghoul = on_battlefield(&engine, P0, dauthi_ghoul()).unwrap();
    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let mindripper = on_battlefield(&engine, P0, index::DAUTHI_MINDRIPPER).unwrap();

    // Killing Grizzly Bears (no shadow) does not put a +1/+1 counter on Dauthi Ghoul.
    cast_from_hand(&mut engine, P0, index::LIGHTNING_BOLT);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(ghoul)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0
    );
    assert_eq!(pt(&engine, ghoul), (1, 1));

    // Killing Dauthi Mindripper (which has shadow) puts a +1/+1 counter on Dauthi Ghoul.
    cast_from_hand(&mut engine, P0, index::LIGHTNING_BOLT);
    aim_at(&mut engine, P0, mindripper);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(ghoul)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1
    );
    assert_eq!(pt(&engine, ghoul), (2, 2));
}

#[test]
fn dauthi_ghoul_grows_for_opponents_shadow_creatures_and_shadow_limits_blocks() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut e = lk::game(
        SEED,
        &[dauthi_ghoul()],
        &[index::DAUTHI_MINDRIPPER, index::GRIZZLY_BEARS],
        &[index::LIGHTNING_BOLT],
        &[],
    );
    let ghoul = lk::obj(&e, P0, dauthi_ghoul());
    let shadow = lk::obj(&e, lk::P1, index::DAUTHI_MINDRIPPER);
    let bears = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);
    let menu = lk::attack(&mut e, &[ghoul]);
    assert!(lk::may_block(&menu, shadow, ghoul));
    assert!(!lk::may_block(&menu, bears, ghoul));
    lk::block(&mut e, &[]);
    lk::float(&mut e, P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, P0, index::LIGHTNING_BOLT, &[shadow], &[]);
    assert_eq!(pt(&e, ghoul), (2, 2));
}
