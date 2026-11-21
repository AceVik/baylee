//! `cards/enchantments/mv_2/spatial_binding.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn spatial_binding() -> CardIndex {
    card_index("52fc8217-5a14-40b5-a8fc-b6fc45af5f33")
}

fn sandbar_crocodile() -> CardIndex {
    card_index("e11926bf-ca14-4e1e-aca5-9ffedb91868c")
}

fn reality_ripple() -> CardIndex {
    card_index("504d5c29-7c37-4c31-8549-2a49eeef74c8")
}

fn is_phased_out(e: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    e.state()
        .object(id)
        .is_some_and(|o| o.status.contains(Status::PHASED_OUT))
}

/// Pay 1 life: until your next upkeep, target permanent can't phase out — stops
/// spell phasing on opponent's turn and Phasing during your next untap step,
/// then expires at your upkeep so phasing works again afterwards.
#[test]
fn spatial_binding_prevents_phasing_until_your_next_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[spatial_binding()],
        &[],
        &[sandbar_crocodile(), reality_ripple()],
        &[reality_ripple()],
    );

    // Cast Sandbar Crocodile on Turn 1 so it starts on the battlefield phased in.
    lk::float(&mut e, p0, &[(ManaColor::Blue, 5)]);
    cast_with_floating(&mut e, p0, sandbar_crocodile());
    lk::settle(&mut e);
    let croc = lk::obj(&e, p0, sandbar_crocodile());

    // Pay 1 life -> shield Sandbar Crocodile until P0's next upkeep.
    activate(&mut e, p0, spatial_binding(), 0);
    lk::aim(&mut e, &[croc], &[]);
    lk::settle(&mut e);
    assert_eq!(lk::life(&e, p0), 19);

    // Opponent casts Reality Ripple targeting Crocodile: prevented!
    reach_their_main_phase(&mut e, p1);
    lk::float(&mut e, p1, &[(ManaColor::Blue, 2)]);
    cast_with_floating(&mut e, p1, reality_ripple());
    lk::aim(&mut e, &[croc], &[]);
    lk::settle(&mut e);
    assert!(!is_phased_out(&e, croc));

    // P0's next untap step precedes P0's upkeep: Crocodile's Phasing cannot phase it out!
    reach_their_main_phase(&mut e, p0);
    assert!(
        !is_phased_out(&e, croc),
        "UntilYourNextUpkeep still active during preceding untap step"
    );

    // Now in P0's main phase (after P0's upkeep expired the duration), Reality Ripple works.
    lk::float(&mut e, p0, &[(ManaColor::Blue, 2)]);
    cast_with_floating(&mut e, p0, reality_ripple());
    lk::aim(&mut e, &[croc], &[]);
    lk::settle(&mut e);
    assert!(is_phased_out(&e, croc));
}
