//! `cards/sorceries/mv_2/morningtide.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn morningtide_exiles_all_cards_from_all_graveyards() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("a8432372-672b-4bf7-9000-17e43356f149");
    let mut game = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[plains(), plains(), index::GRIZZLY_BEARS, quiet_artifact()],
        )
        .battlefield(1, &[index::HILL_GIANT, swamp()])
        .hand(0, &[card])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let bear = on_battlefield(&game, p0, index::GRIZZLY_BEARS).unwrap();
    let art = on_battlefield(&game, p0, quiet_artifact()).unwrap();
    let giant = on_battlefield(&game, p1, index::HILL_GIANT).unwrap();
    let sw = on_battlefield(&game, p1, swamp()).unwrap();
    bury(&mut game, &[bear, art, giant, sw]);
    game.refresh_offer();

    tap_all_mana(&mut game, p0);
    cast_with_floating(&mut game, p0, card);
    let rest = drive_to_rest(&mut game, p0);
    assert!(matches!(rest, Rest::Reached), "{rest:?}");

    for exiled in [bear, art, giant, sw] {
        assert_eq!(
            game.state().object(exiled).map(|o| o.zone),
            Some(Zone::Exile)
        );
    }
    assert!(
        game.state()
            .zones
            .list(ZoneLocation::Graveyard(p1))
            .is_empty()
    );
    // Only Morningtide itself goes to p0's graveyard after resolving.
    assert_eq!(
        game.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        1
    );
    assert!(in_graveyard(&game, p0, card).is_some());
}
