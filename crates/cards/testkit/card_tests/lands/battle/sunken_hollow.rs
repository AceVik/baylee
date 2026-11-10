//! `cards/lands/battle/sunken_hollow.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sunken Hollow, a battle land: "This land enters tapped unless you control
/// two or more basic lands."
///
/// The condition **counts**, and what it counts is *basic* lands — two
/// things the checkland sentence next to it says neither of. The card was
/// written as a checkland ("unless you control an Island or a Swamp"), which
/// is the same answer on most boards and the wrong one on this one: two
/// Forests are two basic lands and neither is an Island.
#[test]
fn a_battle_land_counts_two_basics_of_any_kind() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(111, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[sunken_hollow()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hollow = play_land(&mut engine, p0, sunken_hollow());
    assert!(
        !entered_tapped(&engine, hollow),
        "two Forests are two basic lands"
    );
}

/// The other half, and it carries both bystanders the sentence needs.
///
/// One basic of your own is not two; an opponent's basics are not yours; and
/// a land that *prints* the right subtype is not basic — Irrigated Farmland
/// is a `Plains Island` and counts for nothing here, which is the half a
/// filter over subtypes gets exactly backwards.
#[test]
fn a_battle_land_counts_only_your_own_basic_lands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(112, forest())
        .battlefield(0, &[island(), irrigated_farmland()])
        .battlefield(1, &[forest(), forest()])
        .hand(0, &[sunken_hollow()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hollow = play_land(&mut engine, p0, sunken_hollow());
    assert!(
        entered_tapped(&engine, hollow),
        "one basic land, a nonbasic Island and two basics across the table"
    );
}
