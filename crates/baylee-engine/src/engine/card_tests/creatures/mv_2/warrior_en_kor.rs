//! `cards/creatures/mv_2/warrior_en_kor.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn warrior_en_kor() -> CardIndex {
    card_index("1245b165-e26f-4839-8468-2e95148fc6f6")
}

/// Warrior en-Kor (`1245b165-e26f-4839-8468-2e95148fc6f6`) — {W}{W} Creature — Kor Warrior Knight (Partial):
/// 2/2 body (the `{0}` creature-to-creature damage redirection ability is not supported).
#[test]
fn warrior_en_kor_enters_as_two_two_creature() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[warrior_en_kor()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, warrior_en_kor());
    pass_until(&mut engine, stack_is_empty);

    let on_bf = on_battlefield(&engine, P0, warrior_en_kor()).expect("Warrior en-Kor on board");
    assert_eq!(pt(&engine, on_bf), (2, 2));
}
