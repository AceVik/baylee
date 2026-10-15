//! `cards/creatures/artifacts/mv_1/sparring_construct.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn sparring_construct() -> CardIndex {
    card_index("bc948e19-816b-4c13-a36b-73b3e0472dca")
}

/// "When this creature dies, put a +1/+1 counter on target creature you
/// control." The menu holds the controller's creatures only, never the
/// opponent's and never the dead Construct.
#[test]
fn sparring_construct_hands_a_counter_to_a_creature_you_control_when_it_dies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = main_with(
        &[sparring_construct(), grizzly_bears(), llanowar_elves()],
        &[gray_ogre()],
    );
    let me = on_battlefield(&engine, p0, sparring_construct()).expect("seated");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("bears");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("elves");
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("ogre");

    crate::sba::destroy(
        engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up"),
        me,
    );
    engine
        .apply(p0, PlayerAction::PassPriority)
        .expect("passing is legal");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let options = aim_at(&mut engine, p0, bears);
    assert!(options.contains(&bears) && options.contains(&elves));
    assert!(!options.contains(&ogre), "only creatures you control");
    assert!(!options.contains(&me), "it is dead");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, bears, CounterKind::P1P1), 1);
    assert_eq!(counters_on(&engine, elves, CounterKind::P1P1), 0);
    assert!(in_graveyard(&engine, p0, sparring_construct()).is_some());
}
