//! `cards/lands/restricted/madblind_mountain.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The ordinary mana ability remains available without two red permanents.
#[test]
fn madblind_mountain_enters_tapped_and_taps_for_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(107, forest())
        .hand(0, &[madblind_mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, madblind_mountain());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, madblind_mountain(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert!(is_tapped(&engine, land));
}

#[test]
fn madblind_mountain_requires_two_red_permanents_and_shuffles_without_moving_cards() {
    use crate::engine::card_tests::legends_kit as kit;
    for count in 0..=2 {
        let mut board = vec![madblind_mountain()];
        board.extend(std::iter::repeat_n(wild_colos(), count));
        let mut engine = kit::game(942, &board, &[wild_colos()], &[], &[]);
        kit::float(&mut engine, kit::P0, &[(ManaColor::Red, 1)]);
        assert_eq!(
            kit::offered(&engine, madblind_mountain(), kit::P0, 1),
            count == 2
        );
        if count == 2 {
            let mut before = engine
                .state()
                .zones
                .list(ZoneLocation::Library(kit::P0))
                .clone();
            kit::use_ability(&mut engine, kit::P0, madblind_mountain(), 1, &[], &[]);
            let mut after = engine
                .state()
                .zones
                .list(ZoneLocation::Library(kit::P0))
                .clone();
            assert_ne!(before, after);
            before.sort_unstable();
            after.sort_unstable();
            assert_eq!(before, after);
        }
    }
}
