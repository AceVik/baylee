//! `cards/creatures/mv_4/possessed_centaur.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::index;

#[test]
fn possessed_centaur_gains_plus_one_becomes_black_and_destroys_green_creature_at_threshold() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("2fd1707b-917c-4842-a5c6-dfffbedcf918");
    let mut board = vec![card];
    board.extend_from_slice(&[swamp(); 10]);
    let mut game = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .battlefield(1, &[index::GRIZZLY_BEARS, index::HILL_GIANT])
        .hand(0, &[swamp()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let centaur = on_battlefield(&game, p0, card).unwrap();
    let bear = on_battlefield(&game, p1, index::GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&game, p1, index::HILL_GIANT).unwrap();
    assert_eq!(pt(&game, centaur), (3, 3));
    assert!(keywords(&game, centaur).contains(KeywordSet::TRAMPLE));
    assert_eq!(
        game.state()
            .object(centaur)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::from_slice(&[Color::Green])
    );

    let swamps = all_on_battlefield(&game, p0, swamp());
    bury(&mut game, &swamps[..6]);
    game.refresh_offer();
    tap_all_mana(&mut game, p0);
    let Pending::Priority { legal, .. } = game.pending() else {
        panic!("expected priority");
    };
    assert!(
        !legal.abilities.contains(&(centaur, 0)),
        "threshold ability must not be offered with 6 cards in graveyard"
    );

    bury(&mut game, &[swamps[6]]);
    game.refresh_offer();
    play_land(&mut game, p0, swamp());
    assert_eq!(pt(&game, centaur), (4, 4));
    assert_eq!(
        game.state()
            .object(centaur)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::from_slice(&[Color::Black])
    );
    activate(&mut game, p0, card, 0);
    let Pending::ChooseTargets { options, .. } = game.pending().clone() else {
        panic!("expected ChooseTargets");
    };
    assert!(options.contains(&bear));
    assert!(!options.contains(&giant));
    assert!(!options.contains(&centaur));
    game.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![bear],
            players: vec![],
        },
    )
    .unwrap();
    let rest = drive_to_rest(&mut game, p0);
    assert!(matches!(rest, Rest::Reached), "{rest:?}");
    assert!(in_graveyard(&game, p1, index::GRIZZLY_BEARS).is_some());
}
