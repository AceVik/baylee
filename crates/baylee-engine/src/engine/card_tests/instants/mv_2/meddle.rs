//! `cards/instants/mv_2/meddle.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "67d6fbcb-2df7-4b5a-8464-158f6b720a2e"
fn meddle() -> CardIndex {
    card_index("67d6fbcb-2df7-4b5a-8464-158f6b720a2e")
}

#[test]
fn meddle_changes_target_of_single_target_spell_to_another_creature() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[index::HILL_GIANT],
        &[index::TERROR],
        &[meddle()],
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
        &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
    );
    cast_with_floating(&mut engine, P1, meddle());
    lk::aim(&mut engine, &[terror_spell], &[]);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. }) || stack_is_empty(e)
    });
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        lk::aim(&mut engine, &[bears], &[]);
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_none(),
        "Meddle redirected Terror to destroy P0's Grizzly Bears"
    );
    assert!(
        on_battlefield(&engine, P1, index::HILL_GIANT).is_some(),
        "P1's Hill Giant survived"
    );
}
