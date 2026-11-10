//! `cards/creatures/artifacts/mv_4/millennial_gargoyle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn millennial_gargoyle() -> CardIndex {
    card_index("fe07a23a-e526-4277-8c39-6bae5355f0d4")
}

/// Millennial Gargoyle — {4} 2/2 artifact creature with flying. Cast for
/// four generic mana it is a 2/2 flier, and a Grizzly Bears may not block it.
#[test]
fn millennial_gargoyle_is_a_two_two_flier() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 4])
        .hand(0, &[millennial_gargoyle()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, millennial_gargoyle());
    pass_until(&mut engine, stack_is_empty);
    let gargoyle = on_battlefield(&engine, p0, millennial_gargoyle()).expect("it resolved");
    assert_eq!(pt(&engine, gargoyle), (2, 2));
    assert!(keywords(&engine, gargoyle).contains(KeywordSet::FLYING));

    assert!(!bears_may_block(millennial_gargoyle()), "flying");
    assert!(bears_may_block(quiet_creature()), "the control");
}
