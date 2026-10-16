//! `cards/artifacts/mv_4/ring_of_the_lucii.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn ring_of_the_lucii() -> CardIndex {
    card_index("d37e595f-afe7-4ad9-87db-b9ceb3f435e8")
}

/// "{T}: Add {C}{C}."
#[test]
fn tapping_the_ring_adds_two_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ring_of_the_lucii()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 2);
    assert_eq!(pool.total(), 2);
}

/// "{2}, {T}, Pay 1 life: Tap target nonland permanent." Lands are not on the
/// menu; the target is tapped and the life is paid.
#[test]
fn it_taps_a_nonland_permanent_for_two_mana_and_a_life() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ring_of_the_lucii(), forest(), forest()])
        .battlefield(1, &[gray_ogre(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ring = on_battlefield(&engine, p0, ring_of_the_lucii()).unwrap();
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    let land = on_battlefield(&engine, p1, forest()).unwrap();
    let life = life_of(&engine, p0);

    tap_mana_except(&mut engine, p0, ring);
    activate(&mut engine, p0, ring_of_the_lucii(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("a target, got {:?}", engine.pending())
    };
    assert!(options.contains(&ogre), "{options:?}");
    assert!(!options.contains(&land), "\"nonland\": {options:?}");
    aim_at(&mut engine, p0, ogre);
    assert_eq!(life_of(&engine, p0), life - 1, "the life is a cost");
    assert!(is_tapped(&engine, ring));
    assert!(!is_tapped(&engine, ogre), "not yet: the ability is on the stack");
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, ogre));
    assert!(!is_tapped(&engine, land));
}
