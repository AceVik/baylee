//! `cards/creatures/mv_4/spirit_en_kor.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w2c_kit::power_toughness;

const P0: PlayerId = PlayerId::new(0);

fn spirit_en_kor() -> CardIndex {
    card_index("f066174a-a959-4365-920f-c04506d94a5a")
}

/// Spirit en-Kor (`f066174a-a959-4365-920f-c04506d94a5a`) — {3}{W} 2/2 Creature — Kor Spirit (Partial):
/// Plays as a 2/2 Kor Spirit with flying.
#[test]
fn spirit_en_kor_enters_with_flying() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .hand(0, &[spirit_en_kor()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, spirit_en_kor());
    pass_until(&mut engine, stack_is_empty);

    let spirit = on_battlefield(&engine, P0, spirit_en_kor()).unwrap();
    assert_eq!(power_toughness(&engine, spirit), (2, 2));
    assert!(keywords(&engine, spirit).contains(KeywordSet::FLYING));
}
