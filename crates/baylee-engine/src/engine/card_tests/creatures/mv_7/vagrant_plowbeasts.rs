//! `cards/creatures/mv_7/vagrant_plowbeasts.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn vagrant_plowbeasts() -> CardIndex {
    card_index("e97870a3-855a-4701-8344-91a90095f70a")
}

/// Vagrant Plowbeasts — a 6/6 for {5}{G}{W}.
#[test]
fn vagrant_plowbeasts_costs_seven_and_is_a_six_six() {
    let board = [lots(forest(), 6), vec![plains()]].concat();
    let mut engine = table7(&board, &[vagrant_plowbeasts()], &[]);
    let beast = cast_and_resolve(&mut engine, vagrant_plowbeasts());
    assert_body(&engine, beast, (6, 6), KeywordSet::default());
}

/// "{1}: Regenerate target creature with power 5 or greater." The menu holds
/// the Plowbeasts and the 8/7 flier and not the 2/2 Ogre; with a shield on the
/// flier, destruction regenerates it.
#[test]
fn vagrant_plowbeasts_regenerates_a_big_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = table7(
        &[vagrant_plowbeasts(), flier(), gray_ogre(), forest()],
        &[],
        &[],
    );
    let beast = on_battlefield(&engine, p0, vagrant_plowbeasts()).expect("out");
    let sphinx = on_battlefield(&engine, p0, flier()).expect("out");
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("out");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, vagrant_plowbeasts(), 0);
    let (options, _) = target_menu(&engine);
    assert!(options.contains(&beast) && options.contains(&sphinx));
    assert!(!options.contains(&ogre), "power 2 is below 5");
    point_at(&mut engine, p0, &[sphinx], &[]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(sphinx)
            .expect("out")
            .regeneration_shields,
        1
    );
    kill(&mut engine, sphinx);
    assert_eq!(on_battlefield(&engine, p0, flier()), Some(sphinx));
}
