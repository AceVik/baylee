//! `cards/creatures/artifacts/mv_2/gust_skimmer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn gust_skimmer() -> CardIndex {
    card_index("3f49dfa6-d0d4-4b58-8417-ed11c062e220")
}

/// Gust-Skimmer: a 2/1 with "{U}: This creature gains flying until end of
/// turn". Without it the Skimmer is blocked by the Bears; with it only the
/// Crow may block, and the flying is gone next turn.
#[test]
fn gust_skimmer_gains_flying_for_blue_until_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(
        &[],
        &[gust_skimmer(), island()],
        &[grizzly_bears(), storm_crow()],
    );
    let skimmer = on_battlefield(&engine, p0, gust_skimmer()).expect("seated");
    assert_eq!(pt(&engine, skimmer), (2, 1));
    assert!(!keywords(&engine, skimmer).contains(KeywordSet::FLYING));
    assert!(!priority_offer(&engine).abilities.contains(&(skimmer, 0)));

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, gust_skimmer(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, skimmer).contains(KeywordSet::FLYING));

    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");
    let crow = on_battlefield(&engine, p1, storm_crow()).expect("seated");
    let options = attack_and_collect_blocks(&mut engine, skimmer, p1);
    assert!(options.iter().any(|o| o.blocker == crow));
    assert!(!options.iter().any(|o| o.blocker == bears));
}

/// Flying lasts only the turn.
#[test]
fn gust_skimmer_loses_flying_next_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[gust_skimmer(), island()], &[]);
    let skimmer = on_battlefield(&engine, p0, gust_skimmer()).expect("seated");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, gust_skimmer(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, skimmer).contains(KeywordSet::FLYING));
    to_my_next_turn(&mut engine);
    assert!(!keywords(&engine, skimmer).contains(KeywordSet::FLYING));
}
