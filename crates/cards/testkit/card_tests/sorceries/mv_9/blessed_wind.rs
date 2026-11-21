//! `cards/sorceries/mv_9/blessed_wind.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn blessed_wind() -> CardIndex {
    card_index("93da4a35-d2d8-476c-ad03-318101c0118b")
}

/// Blessed Wind (93da4a35-d2d8-476c-ad03-318101c0118b) — {7}{W}{W} Sorcery:
/// "Target player's life total becomes 20."
#[test]
fn blessed_wind_sets_target_players_life_total_to_twenty_up_or_down() {
    let mut raise = Duel::new(SEED, forest())
        .life(0, 7)
        .battlefield(0, &[plains(); 9])
        .hand(0, &[blessed_wind()])
        .start();
    keep_mulligans(&mut raise);
    reach_main_phase(&mut raise, P0);

    tap_all_mana(&mut raise, P0);
    lk::announce(&mut raise, P0, blessed_wind(), 0, 0, &[], &[P0]);
    lk::settle(&mut raise);
    assert_eq!(lk::life(&raise, P0), 20, "raised from 7 to 20");
    assert_eq!(lk::life(&raise, P1), 20);

    let mut lower = Duel::new(SEED, forest())
        .life(1, 32)
        .battlefield(0, &[plains(); 9])
        .hand(0, &[blessed_wind()])
        .start();
    keep_mulligans(&mut lower);
    reach_main_phase(&mut lower, P0);

    tap_all_mana(&mut lower, P0);
    lk::announce(&mut lower, P0, blessed_wind(), 0, 0, &[], &[P1]);
    lk::settle(&mut lower);
    assert_eq!(lk::life(&lower, P1), 20, "lowered from 32 to 20");
}
