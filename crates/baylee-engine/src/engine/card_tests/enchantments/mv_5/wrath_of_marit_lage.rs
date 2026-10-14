//! `cards/enchantments/mv_5/wrath_of_marit_lage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn wrath_of_marit_lage() -> CardIndex {
    card_index("740cafc6-9521-403a-ab5f-11582bf55af0")
}

/// "When this enchantment enters, tap all red creatures. Red creatures
/// don't untap during their controllers' untap steps." Both players' red
/// creatures are tapped and stay so; a green creature is left alone.
#[test]
fn wrath_of_marit_lage_taps_red_creatures_and_keeps_them_tapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[wrath_of_marit_lage()])
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                gray_ogre(),
            ],
        )
        .battlefield(1, &[gray_ogre(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let mine = on_battlefield(&engine, p0, gray_ogre()).expect("my Ogre");
    let theirs = on_battlefield(&engine, p1, gray_ogre()).expect("their Ogre");
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves");

    cast_from_hand(&mut engine, p0, wrath_of_marit_lage());
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && on_battlefield(e, p0, wrath_of_marit_lage()).is_some()
    });
    assert!(is_tapped(&engine, mine), "my red creature");
    assert!(is_tapped(&engine, theirs), "their red creature");
    assert!(!is_tapped(&engine, elves), "a green creature is not red");

    pass_until(&mut engine, |e| {
        e.state().turn.number == 3
            && e.state().turn.active == p0
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert!(is_tapped(&engine, theirs), "theirs skipped its untap");
    assert!(is_tapped(&engine, mine), "mine skipped its untap");
}
