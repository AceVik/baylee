//! `cards/enchantments/mv_4/zur_s_weirding.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn zur_s_weirding() -> CardIndex {
    card_index("c6eaa147-3566-43a9-999a-d58b877496f5")
}

/// Players play with hands revealed; when any player would draw a card, it is
/// revealed instead and another player may pay 2 life to put it into its
/// owner's graveyard instead of drawing it.
#[test]
fn zur_s_weirding_reveals_hands_and_lets_other_player_pay_2_life_to_deny_draw() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(SEED, &[zur_s_weirding()], &[], &[], &[]);
    assert!(e.state().hand_revealed(p0) && e.state().hand_revealed(p1));

    // P1's draw step: P0 pays 2 life to send P1's drawn card to graveyard.
    let p1_top = *e
        .state()
        .zones
        .list(ZoneLocation::Library(p1))
        .last()
        .unwrap();
    pass_until(&mut e, |e| {
        e.state().turn.active == p1
            && matches!(e.pending(), Pending::YesNo { player, .. } if *player == p0)
    });
    lk::yes_no(&mut e, true);
    lk::settle(&mut e);
    assert_eq!(lk::life(&e, p0), 18);
    assert_eq!(lk::zone_of(&e, p1_top), Zone::Graveyard);
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(p1)).len(), 0);

    // P0's draw step: P1 declines to pay 2 life -> P0 draws the card into hand.
    let p0_top = *e
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .last()
        .unwrap();
    pass_until(&mut e, |e| {
        e.state().turn.active == p0
            && matches!(e.pending(), Pending::YesNo { player, .. } if *player == p1)
    });
    lk::yes_no(&mut e, false);
    lk::settle(&mut e);
    assert_eq!(lk::life(&e, p1), 20);
    assert_eq!(lk::zone_of(&e, p0_top), Zone::Hand);
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(p0)).len(), 1);
}
