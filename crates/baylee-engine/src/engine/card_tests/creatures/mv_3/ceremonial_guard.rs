//! `cards/creatures/mv_3/ceremonial_guard.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "3d8aae86-266f-4bfd-a058-0fe12b5ea354"
fn ceremonial_guard() -> CardIndex {
    card_index("3d8aae86-266f-4bfd-a058-0fe12b5ea354")
}

/// Ceremonial Guard ({2}{R} 3/4 Creature — Human Soldier):
/// "When this creature attacks or blocks, destroy it at end of combat."
#[test]
fn ceremonial_guard_deals_combat_damage_when_attacking_then_is_destroyed_at_end_of_combat() {
    let mut engine = lk::game(SEED, &[ceremonial_guard()], &[], &[], &[]);
    let guard = lk::obj(&engine, P0, ceremonial_guard());
    assert_eq!(pt(&engine, guard), (3, 4));

    lk::attack(&mut engine, &[guard]);
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);

    assert_eq!(
        lk::life(&engine, P1),
        17,
        "Ceremonial Guard dealt 3 combat damage before end of combat"
    );
    assert!(
        in_graveyard(&engine, P0, ceremonial_guard()).is_some(),
        "Ceremonial Guard was destroyed at end of combat after attacking"
    );
}

#[test]
fn ceremonial_guard_is_destroyed_at_end_of_combat_after_blocking() {
    let mut engine = lk::game(
        SEED,
        &[ceremonial_guard()],
        &[index::GRIZZLY_BEARS],
        &[],
        &[],
    );
    let guard = lk::obj(&engine, P0, ceremonial_guard());
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    reach_their_main_phase(&mut engine, P1);
    lk::attack(&mut engine, &[bears]);
    lk::block(&mut engine, &[(guard, bears)]);
    lk::through_combat(&mut engine);

    assert!(
        in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some(),
        "3/4 Ceremonial Guard killed the 2/2 attacker in combat"
    );
    assert!(
        in_graveyard(&engine, P0, ceremonial_guard()).is_some(),
        "Ceremonial Guard was destroyed at end of combat after blocking"
    );
}
