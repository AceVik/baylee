//! `cards/creatures/artifacts/mv_3/kiln_walker.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn kiln_walker() -> CardIndex {
    card_index("ca868ac4-6144-402e-ae23-629628d02883")
}

/// Kiln Walker — a 0/3 that "gets +3/+0 until end of turn" whenever it
/// attacks: it is a 0/3 before, a 3/3 by the time blockers are declared,
/// deals 3 to an unblocked defender, and is a 0/3 again next turn.
#[test]
fn kiln_walker_gets_plus_three_power_until_end_of_turn_when_it_attacks() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[], &[kiln_walker()]);
    let walker = on_battlefield(&engine, p0, kiln_walker()).expect("seated");
    assert_eq!(pt(&engine, walker), (0, 3));

    attack_to_blocks(&mut engine, &[walker]);
    assert_eq!(pt(&engine, walker), (3, 3), "the attack trigger resolved");
    engine
        .apply(PlayerId::new(1), PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(life_of(&engine, 1), 17, "three damage went through");

    pass_until(&mut engine, |e| e.state().turn.number > 1);
    assert_eq!(pt(&engine, walker), (0, 3), "until end of turn");
}
