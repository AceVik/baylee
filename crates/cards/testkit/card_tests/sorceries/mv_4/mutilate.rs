//! `cards/sorceries/mv_4/mutilate.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn mutilate_shrinks_all_creatures_by_number_of_swamps_you_control() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("698e79ec-c2bc-4973-a095-f3c57b50e39c");
    let mut game = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), plains(), index::HILL_GIANT])
        .battlefield(1, &[swamp(), index::GRIZZLY_BEARS, index::SERRA_ANGEL])
        .hand(0, &[card])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let giant = on_battlefield(&game, p0, index::HILL_GIANT).unwrap();
    let angel = on_battlefield(&game, p1, index::SERRA_ANGEL).unwrap();

    // p0 controls 3 Swamps (opponent's Swamp does not count) -> all creatures get -3/-3.
    tap_all_mana(&mut game, p0);
    cast_with_floating(&mut game, p0, card);
    let rest = drive_to_rest(&mut game, p0);
    assert!(matches!(rest, Rest::Reached), "{rest:?}");

    assert!(on_battlefield(&game, p0, index::HILL_GIANT).is_none());
    assert!(on_battlefield(&game, p1, index::GRIZZLY_BEARS).is_none());
    assert!(
        game.state().object(giant).is_some()
            && in_graveyard(&game, p0, index::HILL_GIANT).is_some()
    );
    assert_eq!(pt(&game, angel), (1, 1));
}
