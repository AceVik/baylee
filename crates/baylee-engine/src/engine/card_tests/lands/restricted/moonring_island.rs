//! `cards/lands/restricted/moonring_island.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Moonring Island prints `This land enters tapped`, `{T}: Add {U}`, and `{U}, {T}: Look at the top card
/// of target player's library. Activate only if you control two or more blue permanents.`
/// The card is marked `Coverage::Partial` because peeking at another player's library is not supported.
/// Moonring Island enters tapped, untaps on the following turn cycle, and with two `island` permanents
/// present offers only ability index 0 for `{U}` and never the library inspection ability.
#[test]
fn moonring_island_enters_tapped_and_taps_for_blue_without_peek_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[moonring_island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, moonring_island());
    assert!(entered_tapped(&engine, land));

    let from = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > from
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority");
    };
    let offered: Vec<(ObjectId, u32)> = legal
        .abilities
        .iter()
        .copied()
        .filter(|(source, _)| *source == land)
        .collect();
    assert_eq!(offered, vec![(land, 0)]);

    activate(&mut engine, p0, moonring_island(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, land));
}
