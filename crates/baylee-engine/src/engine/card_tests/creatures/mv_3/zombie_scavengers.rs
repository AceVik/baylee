//! `cards/creatures/mv_3/zombie_scavengers.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn zombie_scavengers() -> CardIndex {
    card_index("8c5298da-6759-444e-8870-6c96d226c940")
}

/// Zombie Scavengers retains its 3/1 body while the top-creature-card exile
/// cost cannot be expressed. A creature card in the graveyard offers no
/// substitute pick-any-card regeneration ability; the body still attacks.
#[test]
fn zombie_scavengers_attacks_without_the_unmodelled_regeneration() {
    let mut e = lk::game(
        SEED,
        &[swamp(), swamp(), swamp()],
        &[],
        &[zombie_scavengers(), quiet_creature()],
        &[],
    );
    hand_to_graveyard(&mut e, P0, quiet_creature());
    cast_from_hand(&mut e, P0, zombie_scavengers());
    lk::settle(&mut e);
    assert!(!lk::offered(&e, zombie_scavengers(), P0, 0));
    assert!(in_graveyard(&e, P0, quiet_creature()).is_some());
    reach_their_main_phase(&mut e, P1);
    reach_their_main_phase(&mut e, P0);
    let body = lk::obj(&e, P0, zombie_scavengers());
    lk::attack(&mut e, &[body]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, P1), 17);
}
