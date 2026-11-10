//! `cards/creatures/mv_3/sedge_troll.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sedge Troll — "This creature gets +1/+1 as long as you control a
/// Swamp." / "{B}: Regenerate this creature." Two boards, with and without
/// the Swamp, and the regeneration itself.
#[test]
fn sedge_troll_is_pumped_only_with_a_swamp_and_regenerates_for_b() {
    let p0 = PlayerId::new(0);

    let mut without_swamp = Duel::new(SEED, mountain())
        .battlefield(0, &[sedge_troll(), mountain()])
        .start();
    keep_mulligans(&mut without_swamp);
    let troll = on_battlefield(&without_swamp, p0, sedge_troll()).expect("seated");
    assert_eq!(
        pt(&without_swamp, troll),
        (2, 2),
        "no Swamp: the printed body"
    );

    let mut with_swamp = Duel::new(SEED, mountain())
        .battlefield(0, &[sedge_troll(), mountain(), swamp()])
        .start();
    keep_mulligans(&mut with_swamp);
    reach_main_phase(&mut with_swamp, p0);
    let troll = on_battlefield(&with_swamp, p0, sedge_troll()).expect("seated");
    assert_eq!(pt(&with_swamp, troll), (3, 3), "a Swamp under its control");

    tap_all_mana(&mut with_swamp, p0);
    activate(&mut with_swamp, p0, sedge_troll(), 1);
    pass_until(&mut with_swamp, stack_is_empty);
    assert_eq!(
        with_swamp
            .state()
            .object(troll)
            .unwrap()
            .regeneration_shields,
        1,
        "the {{B}} the Swamp made pays the regeneration"
    );
}
