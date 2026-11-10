//! `cards/creatures/artifacts/mv_2/hedron_scrabbler.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn hedron_scrabbler() -> CardIndex {
    card_index("180d9c48-6d6c-4d20-965a-0ee2ad57f99b")
}

/// Hedron Scrabbler: a 1/1 with "Landfall — Whenever a land you control
/// enters, this creature gets +1/+1 until end of turn". Playing a Forest makes
/// it a 2/2 until the turn ends.
#[test]
fn hedron_scrabbler_grows_when_a_land_enters() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[forest()], &[hedron_scrabbler()], &[]);
    let scrabbler = on_battlefield(&engine, p0, hedron_scrabbler()).expect("seated");
    assert_eq!(pt(&engine, scrabbler), (1, 1));
    play_land(&mut engine, p0, forest());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, scrabbler), (2, 2), "landfall: +1/+1");
    to_my_next_turn(&mut engine);
    assert_eq!(pt(&engine, scrabbler), (1, 1), "until end of turn");
}

/// A land entering under the opponent's control is not "a land you
/// control": their Forest does nothing.
#[test]
fn hedron_scrabbler_ignores_the_opponents_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hedron_scrabbler()])
        .hand(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let scrabbler = on_battlefield(&engine, p0, hedron_scrabbler()).expect("seated");
    reach_their_main_phase(&mut engine, p1);
    play_land(&mut engine, p1, forest());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, scrabbler), (1, 1));
}
