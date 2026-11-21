//! `cards/creatures/mv_2/ertai_s_familiar.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn familiar() -> CardIndex {
    card_index("711cd80d-c58d-4c2a-87db-d2e4c75f60d3")
}

fn gy_count(e: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    e.state().zones.list(ZoneLocation::Graveyard(seat)).len()
}

/// Ertai's Familiar ({1}{U} 2/2 Illusion):
/// "Phasing; When this creature phases out or leaves the battlefield, mill
/// three cards. {U}: Until your next upkeep, this creature can't phase out."
#[test]
fn ertai_s_familiar_mills_three_when_phasing_out_and_when_leaving_battlefield() {
    let mut e = lk::game(
        SEED,
        &[island(), island(), mountain()],
        &[],
        &[familiar(), index::LIGHTNING_BOLT],
        &[],
    );
    cast_from_hand(&mut e, lk::P0, familiar());
    lk::settle(&mut e);
    let fam = lk::obj(&e, lk::P0, familiar());

    // Opponent's untap step does not phase it out.
    reach_their_main_phase(&mut e, lk::P1);
    assert!(
        !e.state()
            .object(fam)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );
    assert_eq!(gy_count(&e, lk::P0), 0);

    // P0's next untap phases it out -> trigger mills 3.
    reach_their_main_phase(&mut e, lk::P0);
    assert!(
        e.state()
            .object(fam)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );
    assert_eq!(gy_count(&e, lk::P0), 3);

    // Next P0 untap phases it back in without milling.
    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    assert!(
        !e.state()
            .object(fam)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );
    assert_eq!(gy_count(&e, lk::P0), 3);

    // Destroying it triggers leaves-the-battlefield -> mills 3 more (+ Bolt + Familiar = 8).
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, lk::P0, index::LIGHTNING_BOLT, &[fam], &[]);
    assert_eq!(gy_count(&e, lk::P0), 8);
}

#[test]
fn ertai_s_familiar_blue_ability_locks_phasing_until_your_next_upkeep_then_expires() {
    let mut e = lk::game(SEED, &[island(), island()], &[], &[familiar()], &[]);
    cast_from_hand(&mut e, lk::P0, familiar());
    lk::settle(&mut e);
    let fam = lk::obj(&e, lk::P0, familiar());

    lk::float(&mut e, lk::P0, &[(ManaColor::Blue, 1)]);
    lk::use_ability(&mut e, lk::P0, familiar(), 1, &[], &[]);

    // Holds through P0's next untap step: stays phased in and mills nothing.
    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    assert!(
        !e.state()
            .object(fam)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );
    assert_eq!(gy_count(&e, lk::P0), 0);

    // Having expired at that upkeep, the following P0 untap phases it out and mills 3.
    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    assert!(
        e.state()
            .object(fam)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );
    assert_eq!(gy_count(&e, lk::P0), 3);
}
