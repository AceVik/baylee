//! `cards/lands/utility/swarmyard.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Swarmyard` prints `{{T}}: Add {{C}}.` and `{{T}}: Regenerate target Insect,
/// Rat, Spider, or Squirrel.`
///
/// Four subtypes joined with `Filter::Or`, which is a shape a single-subtype
/// card cannot show is wrong: the Spider is one of the four and the Wolf is
/// none of them, so the menu is the assertion.
///
/// The Wolf was already here when this pinned the missing shield — and it is
/// exactly why that pin was hollow. `young_wolf()` is no Insect, Rat, Spider
/// or Squirrel, so ability 1 was off the offer for want of a target and the
/// pin passed on both sides of the rule it was watching.
#[test]
fn swarmyard_regenerates_the_spider_and_not_the_wolf() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swarmyard(), rib_cage_spider(), young_wolf()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let yard = on_battlefield(&engine, p0, swarmyard()).expect("swarmyard on battlefield");
    assert!(!is_tapped(&engine, yard));
    let spider = on_battlefield(&engine, p0, rib_cage_spider()).expect("the Spider is seated");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(yard, 0)),
        "ability 0 ({{T}}: Add {{C}}) is offered"
    );
    assert!(
        legal.abilities.contains(&(yard, 1)),
        "and the regeneration, because one of the four subtypes is on the board"
    );

    activate(&mut engine, p0, swarmyard(), 1);
    let menu = aim_at(&mut engine, p0, spider);
    assert_eq!(
        menu,
        vec![spider],
        "the Wolf is a creature and none of the four vermin"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(spider)
            .expect("the Spider is still there")
            .regeneration_shields,
        1
    );
    assert!(is_tapped(&engine, yard));
}
