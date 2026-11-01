#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::generated::index;
fn resuscitate() -> CardIndex {
    card_index("4390e251-e154-4ef8-9929-0c78aa2c8f10")
}
#[test]
fn resuscitate_grants_paid_regeneration_only_to_your_current_creatures_until_cleanup() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[grizzly_bears(), gray_ogre()],
        &[grizzly_bears()],
        &[resuscitate(), lightning_bolt(), index::HILL_GIANT],
        &[],
    );
    let bear = lk::obj(&e, p0, grizzly_bears());
    let ogre = lk::obj(&e, p0, gray_ogre());
    let enemy = lk::obj(&e, p1, grizzly_bears());
    lk::float(&mut e, p0, &[(ManaColor::Green, 20), (ManaColor::Red, 20)]);
    assert!(kit::offered(&e, bear).is_empty());
    lk::cast_at(&mut e, p0, resuscitate(), &[], &[]);
    let bear_abilities = kit::offered(&e, bear);
    assert_eq!(bear_abilities.len(), 1, "one granted regeneration ability");
    let bear_regeneration = bear_abilities[0];
    assert_eq!(
        kit::offered(&e, ogre).len(),
        1,
        "the other existing ally also receives the grant"
    );
    // A later entrant is outside the spell's original cohort.
    lk::cast_at(&mut e, p0, index::HILL_GIANT, &[], &[]);
    let newcomer = lk::obj(&e, p0, index::HILL_GIANT);
    assert!(kit::offered(&e, newcomer).is_empty());
    let before = lk::pool_total(&e, p0);
    kit::activate_object(&mut e, p0, bear, bear_regeneration);
    lk::settle(&mut e);
    assert_eq!(lk::pool_total(&e, p0), before - 1);
    lk::cast_at(&mut e, p0, lightning_bolt(), &[bear], &[]);
    assert!(on_battlefield(&e, p0, grizzly_bears()).is_some());
    assert!(is_tapped(&e, bear));
    assert_eq!(lk::damage_on(&e, bear), 0);
    lk::pass_once(&mut e);
    lk::float(&mut e, p1, &[(ManaColor::Green, 10)]);
    assert!(kit::offered(&e, enemy).is_empty());
    reach_their_main_phase(&mut e, p1);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    lk::float(&mut e, p0, &[(ManaColor::Green, 10)]);
    assert!(
        kit::offered(&e, ogre).is_empty(),
        "grant expired at cleanup"
    );
}
