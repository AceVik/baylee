//! `cards/creatures/mv_2/geist_of_saint_thalia.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn geist_of_saint_thalia_lands_as_a_flying_body_on_its_printed_cost() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[geist_of_saint_thalia()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, geist_of_saint_thalia());
    pass_until(&mut engine, stack_is_empty);

    let geist = on_battlefield(&engine, p0, geist_of_saint_thalia()).expect("the Geist resolved");
    assert!(
        types(&engine, geist).contains(TypeSet::CREATURE),
        "it landed as a creature",
    );
    assert_eq!(pt(&engine, geist), (1, 2), "the printed 1/2");
    assert!(
        keywords(&engine, geist).contains(KeywordSet::FLYING),
        "printed flying",
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{1}}{{U}} came out of the two Islands and nothing was left over",
    );
}

#[test]
fn geist_reduces_only_its_controllers_noncreature_spells() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    for owner in [0, 1] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(owner, &[geist_of_saint_thalia()])
            .hand(0, &[index::MIND_STONE, grizzly_bears()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        e.dev_state_mut(p0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::Green, 1);
        e.refresh_offer();
        let artifact = in_hand(&e, p0, index::MIND_STONE).unwrap();
        let bear = in_hand(&e, p0, grizzly_bears()).unwrap();
        let Pending::Priority { legal, .. } = e.pending() else {
            panic!("priority")
        };
        assert_eq!(legal.castable.contains(&artifact), owner == 0);
        assert!(!legal.castable.contains(&bear));
        if owner == 0 {
            cast_with_floating(&mut e, p0, index::MIND_STONE);
            assert_eq!(e.state().players[0].mana_pool.total(), 0);
            pass_until(&mut e, stack_is_empty);
        }
    }
}
