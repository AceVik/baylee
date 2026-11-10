//! Mindstorm Crown reads the hand as it was at the start of the turn.

use super::*;

fn mindstorm_crown() -> CardIndex {
    card_index("0ad26c65-bc2c-4aa7-8c5d-e4c8f0629cb7")
}

fn next_upkeep(initial_hand: &[CardIndex]) -> Engine<RegistryLookup> {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[mindstorm_crown()])
        .hand(0, initial_hand)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let drawn = in_hand(&e, p0, forest()).unwrap();
    e.apply(p0, PlayerAction::PlayLand { card: drawn }).unwrap();
    reach_their_main_phase(&mut e, p1);
    reach_their_main_phase(&mut e, p0);
    e
}

#[test]
fn mindstorm_crown_draws_if_turn_began_empty_and_otherwise_deals_one() {
    let p0 = PlayerId::new(0);
    let e = next_upkeep(&[]);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(p0)).len(),
        2,
        "draw step and Crown"
    );
    assert_eq!(e.state().players[0].life, 20);

    let e = next_upkeep(&[forest()]);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(p0)).len(),
        2,
        "initial card and draw step"
    );
    assert_eq!(e.state().players[0].life, 19);
}
