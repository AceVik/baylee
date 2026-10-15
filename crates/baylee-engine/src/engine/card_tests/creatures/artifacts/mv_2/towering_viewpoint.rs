//! `cards/creatures/artifacts/mv_2/towering_viewpoint.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn towering_viewpoint() -> CardIndex {
    card_index("46c404c3-636a-4945-a61c-62f013a9c50f")
}

/// Towering Viewpoint: a 0/4 with "Defender, reach". Not an attacker, and
/// able to block a flier.
#[test]
fn towering_viewpoint_has_defender_and_reach() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(
        &[],
        &[towering_viewpoint(), grizzly_bears()],
        &[storm_crow()],
    );
    let viewpoint = on_battlefield(&engine, p0, towering_viewpoint()).expect("seated");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    assert_eq!(pt(&engine, viewpoint), (0, 4));
    assert!(keywords(&engine, viewpoint).contains(KeywordSet::DEFENDER));
    assert!(keywords(&engine, viewpoint).contains(KeywordSet::REACH));
    reach_their_main_phase(&mut engine, p1);
    let crow = on_battlefield(&engine, p1, storm_crow()).expect("seated");
    let options = attack_and_collect_blocks(&mut engine, crow, p0);
    assert!(options.iter().any(|o| o.blocker == viewpoint));
    assert!(!options.iter().any(|o| o.blocker == bears));
}

/// "Leap of Faith — {3}: Target creature gains flying until end of turn."
/// Aimed at the Bears, three mana give them flying and the Viewpoint itself
/// is a legal target too; nothing happens to the Viewpoint.
#[test]
fn towering_viewpoint_gives_a_target_creature_flying() {
    let p0 = PlayerId::new(0);
    let mut engine = table(
        &[],
        &[
            towering_viewpoint(),
            grizzly_bears(),
            forest(),
            forest(),
            forest(),
        ],
        &[],
    );
    let viewpoint = on_battlefield(&engine, p0, towering_viewpoint()).expect("seated");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    assert!(!keywords(&engine, bears).contains(KeywordSet::FLYING));
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, towering_viewpoint(), 0);
    let options = aim_at(&mut engine, p0, bears);
    assert!(options.contains(&bears) && options.contains(&viewpoint));
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, bears).contains(KeywordSet::FLYING));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    to_my_next_turn(&mut engine);
    assert!(!keywords(&engine, bears).contains(KeywordSet::FLYING));
}
