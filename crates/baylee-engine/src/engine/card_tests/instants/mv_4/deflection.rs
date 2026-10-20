//! `cards/instants/mv_4/deflection.rs`, played.
//!
//! Oracle:
//! - "Change the target of target spell with a single target."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "ec7ae9ed-dc5b-47ed-a4ad-086f3c7c377c"
fn deflection() -> CardIndex {
    card_index("ec7ae9ed-dc5b-47ed-a4ad-086f3c7c377c")
}

#[test]
fn deflection_changes_the_target_of_target_single_target_spell() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[index::HILL_GIANT],
        &[index::TERROR],
        &[deflection()],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Black, 1)],
    );
    cast_with_floating(&mut engine, P0, index::TERROR);
    lk::aim(&mut engine, &[giant], &[]);
    let terror_spell = on_stack(&engine, index::TERROR).expect("Terror on stack");

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );
    lk::float(
        &mut engine,
        P1,
        &[(ManaColor::Colorless, 3), (ManaColor::Blue, 1)],
    );
    cast_with_floating(&mut engine, P1, deflection());
    lk::aim(&mut engine, &[terror_spell], &[]);

    // Deflection resolves and asks P1 for the new target of Terror; only Grizzly Bears is another creature.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. }) || stack_is_empty(e)
    });
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        lk::aim(&mut engine, &[bears], &[]);
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_none(),
        "redirected Terror destroyed P0's Grizzly Bears"
    );
    assert!(
        on_battlefield(&engine, P1, index::HILL_GIANT).is_some(),
        "P1's Hill Giant survived"
    );
}
