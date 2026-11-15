#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn own_token_entries_gain_life_separately_and_opposing_entries_do_not() {
    for rarity in Rarity::ALL {
        let mut game = prepared(
            &[
                item(Family::Hearthstone, rarity),
                item(Family::Grove, Rarity::Uncommon),
                item(Family::BloodAmber, Rarity::Common),
            ],
            &[("Plains", 40)],
        );
        let first = game.state().turn.active;
        let other = PlayerId::new(1 - first.get());
        assert_eq!(
            game.state().players[first.get() as usize].life,
            20,
            "artifacts are not creatures"
        );
        main(&mut game, 1, first);
        assert_eq!(
            game.state().players[first.get() as usize].life,
            20 + i32::from(rarity.tier()) * 2
        );
        assert_eq!(game.state().players[other.get() as usize].life, 20);
        let amber = source(&game, first, LIFE_RELICS[2][0]);
        assert_eq!(
            game.state()
                .arena
                .get(amber)
                .unwrap()
                .counters
                .get(CounterKind::Charge),
            2,
            "two entry events, not gained life points"
        );
        main(&mut game, 2, other);
        assert_eq!(
            game.state().players[first.get() as usize].life,
            20 + i32::from(rarity.tier()) * 2
        );
        assert_eq!(
            game.state().players[other.get() as usize].life,
            20 + i32::from(rarity.tier()) * 2
        );
        assert_eq!(
            game.state()
                .arena
                .get(amber)
                .unwrap()
                .counters
                .get(CounterKind::Charge),
            2,
            "opposing life gain does not charge this artifact"
        );
    }
}
