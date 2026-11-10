//! `cards/creatures/artifacts/mv_4/tower_gargoyle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn tower_gargoyle() -> CardIndex {
    card_index("681ca1eb-2ef5-4d60-b834-58bd9b5a47b8")
}

/// Tower Gargoyle — {1}{W}{U}{B} 4/4 artifact creature with flying. Cast off
/// a Plains, an Island, a Swamp and a Forest it is a 4/4 flier, and a
/// Grizzly Bears may not block it.
#[test]
fn tower_gargoyle_is_a_four_four_flier() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), island(), swamp(), forest()])
        .hand(0, &[tower_gargoyle()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, tower_gargoyle());
    pass_until(&mut engine, stack_is_empty);
    let gargoyle = on_battlefield(&engine, p0, tower_gargoyle()).expect("it resolved");
    assert_eq!(pt(&engine, gargoyle), (4, 4));
    assert!(keywords(&engine, gargoyle).contains(KeywordSet::FLYING));

    assert!(!bears_may_block(tower_gargoyle()), "flying");
    assert!(bears_may_block(quiet_creature()), "the control");
}
