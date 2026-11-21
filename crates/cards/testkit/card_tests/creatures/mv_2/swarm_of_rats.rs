//! `cards/creatures/mv_2/swarm_of_rats.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn swarm_of_rats() -> CardIndex {
    card_index("7fe21ade-01f1-45ff-9b70-bc6302c3d284")
}

/// Swarm of Rats ({1}{B} */1 Rat):
/// "Swarm of Rats's power is equal to the number of Rats you control."
#[test]
fn swarm_of_rats_power_counts_only_rats_you_control_while_toughness_stays_one() {
    let mut e = lk::game(
        SEED,
        &[index::SWAMP, index::SWAMP, pearled_unicorn()],
        &[index::BOG_RATS],
        &[swarm_of_rats(), index::BOG_RATS, index::SHOCK],
        &[],
    );
    cast_from_hand(&mut e, lk::P0, swarm_of_rats());
    lk::settle(&mut e);
    let swarm = lk::obj(&e, lk::P0, swarm_of_rats());

    // Counts itself (1), ignoring your non-Rat Unicorn and P1's Bog Rats.
    assert_eq!(pt(&e, swarm), (1, 1));

    lk::float(&mut e, lk::P0, &[(ManaColor::Black, 1)]);
    cast_with_floating(&mut e, lk::P0, index::BOG_RATS);
    lk::settle(&mut e);
    let my_rat = lk::obj(&e, lk::P0, index::BOG_RATS);
    assert_eq!(pt(&e, swarm), (2, 1));

    // Removing the second Rat drops Swarm of Rats back to 1/1.
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, lk::P0, index::SHOCK, &[my_rat], &[]);
    assert_eq!(pt(&e, swarm), (1, 1));
}
