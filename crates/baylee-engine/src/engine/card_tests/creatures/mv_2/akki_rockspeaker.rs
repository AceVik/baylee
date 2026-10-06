//! `cards/creatures/mv_2/akki_rockspeaker.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Akki Rockspeaker prints one line: "When this creature enters, add {R}."
/// The board is two Mountains and the card, so the {1}{R} is paid out of the
/// very pool the trigger will pay into — and while the Goblin is on the stack
/// that pool reads exactly zero, which is what makes the single red mana
/// afterwards the creature's own and not a land's that merely stayed untapped.
#[test]
fn akki_rockspeaker_adds_one_red_mana_when_it_enters() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(41, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[akki_rockspeaker()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, akki_rockspeaker());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{1}}{{R}} is spent the moment the spell is cast, so the pool is \
         empty while the Rockspeaker is still a spell"
    );
    assert!(
        on_stack(&engine, akki_rockspeaker()).is_some(),
        "and the card is on the stack, not yet a permanent"
    );

    pass_until(&mut engine, stack_is_empty);

    let goblin = on_battlefield(&engine, p0, akki_rockspeaker()).expect("the Rockspeaker resolved");
    assert_eq!(pt(&engine, goblin), (1, 1), "the printed 1/1 body");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "\"when this creature enters, add {{R}}\""
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana and nothing else: both Mountains went to the spell, so \
         nothing but the enters-trigger could have made this"
    );
}
