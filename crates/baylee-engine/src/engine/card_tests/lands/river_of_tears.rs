//! Printed rules exercised through the real game driver.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn river_of_tears_reads_the_current_turns_land_plays() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .hand(0, &[river_of_tears()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let land = play_land(&mut e, p0, river_of_tears());
    activate(&mut e, p0, river_of_tears(), 0);
    assert_eq!(
        e.state().players[0].mana_pool.available(ManaColor::Black),
        1
    );
    assert_eq!(e.state().players[0].mana_pool.available(ManaColor::Blue), 0);
    reach_their_main_phase(&mut e, p1);
    // Untap a land in the opponent's turn, without changing its play history.
    e.dev_state_mut(p0).unwrap().set_tapped(land, false);
    while matches!(e.pending(), Pending::Priority { player, .. } if *player != p0) {
        let player = match e.pending() {
            Pending::Priority { player, .. } => *player,
            _ => unreachable!(),
        };
        e.apply(player, PlayerAction::PassPriority).unwrap();
    }
    e.refresh_offer();
    activate(&mut e, p0, river_of_tears(), 0);
    assert_eq!(e.state().players[0].mana_pool.available(ManaColor::Blue), 1);
    assert_eq!(
        e.state().players[0].mana_pool.available(ManaColor::Black),
        0
    );
}
