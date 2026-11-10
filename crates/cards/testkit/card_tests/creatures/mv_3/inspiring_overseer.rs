//! `cards/creatures/mv_3/inspiring_overseer.rs`, played.
#[allow(clippy::wildcard_imports)]
use super::*;

fn subject() -> CardIndex {
    card_index("d646e42b-5635-4798-b633-29c093b66a55")
}

#[test]
fn inspiring_overseer_flies_and_its_entry_gains_one_and_draws_one() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[subject()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let life = e.state().players[0].life;
    let library = library_size(&e, p0);
    cast_from_hand(&mut e, p0, subject());
    pass_until(&mut e, stack_is_empty);
    let angel = on_battlefield(&e, p0, subject()).expect("Overseer entered");
    assert!(keywords(&e, angel).contains(KeywordSet::FLYING));
    assert_eq!(e.state().players[0].life, life + 1);
    assert_eq!(library_size(&e, p0), library - 1);
}
