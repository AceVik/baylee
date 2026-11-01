//! `cards/artifacts/mv_3/barbed_wire.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "cdc6e187-73d2-4f79-a0ff-daa6d2de5bdb"
fn barbed_wire() -> CardIndex {
    card_index("cdc6e187-73d2-4f79-a0ff-daa6d2de5bdb")
}

/// Barbed Wire ({3} Artifact, Partial):
/// "At the beginning of each player's upkeep, this artifact deals 1 damage to that player."
#[test]
fn barbed_wire_deals_one_damage_to_each_player_on_their_upkeep() {
    let mut engine = lk::game(SEED, &[], &[], &[barbed_wire()], &[]);

    lk::float(&mut engine, P0, &[(ManaColor::Colorless, 3)]);
    cast_with_floating(&mut engine, P0, barbed_wire());
    lk::settle(&mut engine);

    assert_eq!(lk::life(&engine, P0), 20);
    assert_eq!(lk::life(&engine, P1), 20);

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(
        lk::life(&engine, P1),
        19,
        "Barbed Wire dealt 1 damage to P1 in P1's upkeep"
    );
    assert_eq!(lk::life(&engine, P0), 20);

    reach_their_main_phase(&mut engine, P0);
    assert_eq!(
        lk::life(&engine, P0),
        19,
        "Barbed Wire dealt 1 damage to P0 in P0's upkeep"
    );
}
