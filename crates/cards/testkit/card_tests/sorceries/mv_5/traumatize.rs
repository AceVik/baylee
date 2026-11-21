//! `cards/sorceries/mv_5/traumatize.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn traumatize() -> CardIndex {
    card_index("e2ea7d01-6564-4a7f-b935-49a5e3978dac")
}

/// Traumatize (e2ea7d01-6564-4a7f-b935-49a5e3978dac) — {3}{U}{U} Sorcery:
/// "Target player mills half their library, rounded down."
#[test]
fn traumatize_mills_half_target_players_library_rounded_down() {
    let mut engine = lk::game(SEED, &[island(); 5], &[], &[traumatize()], &[forest()]);
    // Ensure P1 has an odd library size so rounded-down vs rounded-up is distinct.
    if library_size(&engine, P1).is_multiple_of(2) {
        hand_to_library_top(&mut engine, P1, forest());
    }
    let before = library_size(&engine, P1);
    assert!(!before.is_multiple_of(2), "odd library size {before}");
    let milled = before / 2;

    tap_all_mana(&mut engine, P0);
    lk::announce(&mut engine, P0, traumatize(), 0, 0, &[], &[P1]);
    lk::settle(&mut engine);

    assert_eq!(
        library_size(&engine, P1),
        before - milled,
        "odd library {before} mills floor({before}/2) = {milled}, leaving {}",
        before - milled
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(P1)).len(),
        milled
    );
}
