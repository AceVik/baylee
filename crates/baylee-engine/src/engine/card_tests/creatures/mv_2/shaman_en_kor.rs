//! `cards/creatures/mv_2/shaman_en_kor.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w2c_kit::power_toughness;

const P0: PlayerId = PlayerId::new(0);

fn shaman_en_kor() -> CardIndex {
    card_index("4d9d5dbb-25ab-41a5-a277-3ee5f9ef579b")
}

/// Shaman en-Kor (`4d9d5dbb-25ab-41a5-a277-3ee5f9ef579b`) — {1}{W} 1/2 Creature — Kor Cleric Shaman (Partial):
/// Plays as a 1/2 Kor Cleric Shaman.
#[test]
fn shaman_en_kor_enters_battlefield_as_one_two() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[shaman_en_kor()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, shaman_en_kor());
    pass_until(&mut engine, stack_is_empty);

    let shaman = on_battlefield(&engine, P0, shaman_en_kor()).unwrap();
    assert_eq!(power_toughness(&engine, shaman), (1, 2));
}
