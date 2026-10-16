//! `cards/creatures/mv_7/spearbreaker_behemoth.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn spearbreaker_behemoth() -> CardIndex {
    card_index("025f8db8-11d1-4c59-8e1f-004c9c843fd8")
}

/// Spearbreaker Behemoth — a 5/5 with indestructible for {5}{G}{G}: a destroy
/// effect leaves it standing.
#[test]
fn spearbreaker_behemoth_is_indestructible() {
    let p0 = PlayerId::new(0);
    let mut engine = table7(&lots(forest(), 7), &[spearbreaker_behemoth()], &[]);
    let behemoth = cast_and_resolve(&mut engine, spearbreaker_behemoth());
    assert_body(&engine, behemoth, (5, 5), KeywordSet::INDESTRUCTIBLE);
    kill(&mut engine, behemoth);
    assert_eq!(
        on_battlefield(&engine, p0, spearbreaker_behemoth()),
        Some(behemoth)
    );
}

/// "{1}: Target creature with power 5 or greater gains indestructible until end
/// of turn." The menu is the creatures of power 5+ (the Behemoth and the 8/7
/// flier, not the 2/2 Ogre), and the flier, destroyed afterwards, survives.
#[test]
fn spearbreaker_behemoth_shields_a_big_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = table7(
        &[spearbreaker_behemoth(), flier(), gray_ogre(), forest()],
        &[],
        &[],
    );
    let behemoth = on_battlefield(&engine, p0, spearbreaker_behemoth()).expect("out");
    let sphinx = on_battlefield(&engine, p0, flier()).expect("out");
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("out");
    assert!(!keywords(&engine, sphinx).contains(KeywordSet::INDESTRUCTIBLE));
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, spearbreaker_behemoth(), 0);
    let (options, _) = target_menu(&engine);
    assert!(options.contains(&sphinx) && options.contains(&behemoth));
    assert!(!options.contains(&ogre), "power 2 is below 5");
    point_at(&mut engine, p0, &[sphinx], &[]);
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, sphinx).contains(KeywordSet::INDESTRUCTIBLE));
    kill(&mut engine, sphinx);
    assert_eq!(
        on_battlefield(&engine, p0, flier()),
        Some(sphinx),
        "survived"
    );
}
