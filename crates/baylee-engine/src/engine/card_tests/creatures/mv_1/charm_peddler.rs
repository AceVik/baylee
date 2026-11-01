//! `cards/creatures/mv_1/charm_peddler.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "c425fe26-0027-4319-80a9-a053925651cb"
fn charm_peddler() -> CardIndex {
    card_index("c425fe26-0027-4319-80a9-a053925651cb")
}

/// Charm Peddler ({W} 1/1 Creature — Human Spellshaper, Partial):
/// Casts for {W} as a 1/1 Human Spellshaper and deals 1 combat damage.
#[test]
fn charm_peddler_casts_for_one_white_and_attacks_as_a_one_one() {
    let mut engine = lk::game(SEED, &[], &[], &[charm_peddler()], &[]);

    lk::float(&mut engine, P0, &[(ManaColor::White, 1)]);
    cast_with_floating(&mut engine, P0, charm_peddler());
    lk::settle(&mut engine);

    let peddler = lk::obj(&engine, P0, charm_peddler());
    assert_eq!(pt(&engine, peddler), (1, 1));

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);

    lk::attack(&mut engine, &[peddler]);
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);

    assert_eq!(lk::life(&engine, P1), 19);
}
