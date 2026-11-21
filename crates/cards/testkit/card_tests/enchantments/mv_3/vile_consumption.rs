//! `cards/enchantments/mv_3/vile_consumption.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn vile_consumption() -> CardIndex {
    card_index("b6a005db-e7e2-4b88-bc8a-c257bcc31cf5")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

/// Every creature has "At the beginning of your upkeep, sacrifice this creature
/// unless you pay 1 life." Paying keeps it alive; declining sacrifices it;
/// noncreatures are unaffected.
#[test]
fn vile_consumption_taxes_all_creatures_on_their_controllers_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[grizzly_bears(), sol_ring()],
        &[llanowar_elves(), savannah_lions()],
        &[vile_consumption()],
        &[],
    );
    let my_bear = lk::obj(&e, p0, grizzly_bears());
    let my_ring = lk::obj(&e, p0, sol_ring());
    let opp_elves = lk::obj(&e, p1, llanowar_elves());
    let opp_lions = lk::obj(&e, p1, savannah_lions());

    lk::float(&mut e, p0, &[(ManaColor::Blue, 1), (ManaColor::Black, 2)]);
    lk::cast_at(&mut e, p0, vile_consumption(), &[], &[]);

    // Advance to P1's upkeep: P1 pays 1 life for one creature and declines for the other.
    pass_until(&mut e, |e| {
        e.state().turn.active == p1
            && matches!(e.pending(), Pending::YesNo { player, .. } if *player == p1)
    });
    lk::yes_no(&mut e, true);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == p1),
    );
    lk::yes_no(&mut e, false);
    lk::settle(&mut e);

    assert_eq!(lk::life(&e, p1), 19);
    let p1_alive = usize::from(lk::zone_of(&e, opp_elves) == Zone::Battlefield)
        + usize::from(lk::zone_of(&e, opp_lions) == Zone::Battlefield);
    assert_eq!(p1_alive, 1);

    // Advance to P0's upkeep: only P0's creature asks (Sol Ring does not); paying keeps it.
    pass_until(&mut e, |e| {
        e.state().turn.active == p0
            && matches!(e.pending(), Pending::YesNo { player, .. } if *player == p0)
    });
    lk::yes_no(&mut e, true);
    lk::settle(&mut e);

    assert_eq!(lk::life(&e, p0), 19);
    assert_eq!(lk::zone_of(&e, my_bear), Zone::Battlefield);
    assert_eq!(lk::zone_of(&e, my_ring), Zone::Battlefield);
}
