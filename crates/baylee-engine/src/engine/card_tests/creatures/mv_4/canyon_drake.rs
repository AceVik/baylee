//! `cards/creatures/mv_4/canyon_drake.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn canyon_drake() -> CardIndex {
    card_index("b5b46c99-1cc1-465a-a831-3fd6665ef560")
}

/// Canyon Drake (b5b46c99-1cc1-465a-a831-3fd6665ef560) — {2}{R}{R} Creature — Drake:
/// "Flying. {1}, Discard a card at random: This creature gets +2/+0 until end of turn."
#[test]
fn canyon_drake_discards_at_random_for_plus_two_plus_zero() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[canyon_drake(), mountain()])
        .hand(0, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let drake = on_battlefield(&engine, P0, canyon_drake()).unwrap();
    assert_eq!(pt(&engine, drake), (1, 2));

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, canyon_drake(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, quiet_creature()).is_some());
    assert_eq!(pt(&engine, drake), (3, 2));
}
