//! `cards/creatures/mv_1/cinder_wall.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn cinder_wall() -> CardIndex {
    card_index("16128b90-9b81-476e-8999-3ef498728c39")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// P1 attacks P0 with one Savannah Lions (2/1); P0's board is whatever the
/// test names plus the Wall cast from hand on turn 1.
fn lions_attack_into(p0_board: &[CardIndex]) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut e = lk::game(SEED, p0_board, &[savannah_lions()], &[cinder_wall()], &[]);
    cast_from_hand(&mut e, P0, cinder_wall());
    pass_until(&mut e, stack_is_empty);
    let wall = lk::obj(&e, P0, cinder_wall());
    let lions = lk::obj(&e, P1, savannah_lions());
    pass_until(&mut e, |e| {
        e.state().turn.active == P1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    lk::declare(&mut e, &[lions]);
    (e, wall, lions)
}

/// Cinder Wall (16128b90-9b81-476e-8999-3ef498728c39) — "When this creature
/// blocks, destroy it at end of combat." The Wall survives the combat damage
/// of the block (a 2/1 cannot kill a 3/3) and is destroyed only when the end
/// of combat step begins.
#[test]
fn cinder_wall_is_destroyed_at_end_of_combat_not_before() {
    let (mut e, wall, lions) = lions_attack_into(&[mountain()]);
    lk::block(&mut e, &[(wall, lions)]);

    lk::to_step(&mut e, P1, Step::CombatDamage);
    assert!(
        on_battlefield(&e, P0, cinder_wall()).is_some(),
        "the Wall is still there after the combat damage step began"
    );
    assert_eq!(lk::damage_on(&e, wall), 2, "the Lions' damage reached it");
    assert!(
        in_graveyard(&e, P1, savannah_lions()).is_some(),
        "its three damage killed the Lions"
    );

    lk::to_step(&mut e, P1, Step::CombatEnd);
    lk::settle(&mut e);
    assert!(
        in_graveyard(&e, P0, cinder_wall()).is_some(),
        "destroyed at end of combat"
    );
    assert!(on_battlefield(&e, P0, cinder_wall()).is_none());
}

/// Cinder Wall — the trigger is "when *this* creature blocks": a Wall that
/// stays out of the combat is not destroyed, and neither is it by another
/// creature of its controller blocking.
#[test]
fn cinder_wall_survives_a_combat_it_does_not_block_in() {
    let (mut e, wall, lions) = lions_attack_into(&[mountain(), grizzly_bears()]);
    let bears = lk::obj(&e, P0, grizzly_bears());
    lk::block(&mut e, &[(bears, lions)]);

    lk::to_step(&mut e, P1, Step::CombatEnd);
    lk::settle(&mut e);
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(
        e.state()
            .object(wall)
            .is_some_and(|o| o.zone == Zone::Battlefield),
        "the Wall did not block, so it stays"
    );
    assert!(in_graveyard(&e, P0, cinder_wall()).is_none());
}
