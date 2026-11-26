//! `cards/enchantments/mv_6/yawgmoth_s_bargain.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn yawgmoth_s_bargain() -> CardIndex {
    card_index("f7f76f39-a0de-4bda-86b6-0f291892fcec")
}

fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}

#[test]
fn yawgmoth_s_bargain_skips_draw_step_and_pays_1_life_to_draw() {
    let mut e = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[yawgmoth_s_bargain()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);

    cast_from_hand(&mut e, P0, yawgmoth_s_bargain());
    pass_until(&mut e, stack_is_empty);

    // Walk into P0's next main phase: P0's draw step must be skipped (hand stays 0!).
    reach_their_main_phase(&mut e, P1);
    let lib_before = library_size(&e, P0);
    reach_their_main_phase(&mut e, P0);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(P0)).len(),
        0,
        "draw step skipped"
    );
    assert_eq!(library_size(&e, P0), lib_before);

    // Pay 1 life twice to draw 2 cards.
    for _ in 0..2 {
        activate(&mut e, P0, yawgmoth_s_bargain(), 1);
        pass_until(&mut e, stack_is_empty);
    }
    assert_eq!(e.state().players[0].life, 18);
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(P0)).len(), 2);
}
