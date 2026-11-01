//! `cards/creatures/mv_4/possessed_aven.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::index;

#[test]
fn possessed_aven_gains_plus_one_becomes_black_and_destroys_blue_creature_at_threshold() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("1a5f1670-3e71-4379-90ee-a883c8dc6070");
    let mut board = vec![card];
    board.extend_from_slice(&[swamp(); 10]);
    let mut game = Duel::new(SEED, island())
        .battlefield(0, &board)
        .battlefield(1, &[index::PHANTOM_MONSTER, index::GRIZZLY_BEARS])
        .hand(0, &[swamp()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let aven = on_battlefield(&game, p0, card).unwrap();
    let phantom = on_battlefield(&game, p1, index::PHANTOM_MONSTER).unwrap();
    let bear = on_battlefield(&game, p1, index::GRIZZLY_BEARS).unwrap();
    assert_eq!(pt(&game, aven), (3, 3));
    assert!(keywords(&game, aven).contains(KeywordSet::FLYING));
    assert_eq!(
        game.state().object(aven).unwrap().characteristics().colors,
        ColorSet::from_slice(&[Color::Blue])
    );

    let swamps = all_on_battlefield(&game, p0, swamp());
    bury(&mut game, &swamps[..6]);
    game.refresh_offer();
    tap_all_mana(&mut game, p0);
    let Pending::Priority { legal, .. } = game.pending() else {
        panic!("expected priority");
    };
    assert!(
        !legal.abilities.contains(&(aven, 0)),
        "threshold ability must not be offered with 6 cards in graveyard"
    );

    bury(&mut game, &[swamps[6]]);
    game.refresh_offer();
    play_land(&mut game, p0, swamp());
    assert_eq!(pt(&game, aven), (4, 4));
    assert_eq!(
        game.state().object(aven).unwrap().characteristics().colors,
        ColorSet::from_slice(&[Color::Black])
    );
    activate(&mut game, p0, card, 0);
    let Pending::ChooseTargets { options, .. } = game.pending().clone() else {
        panic!("expected ChooseTargets");
    };
    assert!(options.contains(&phantom));
    assert!(
        !options.contains(&bear),
        "only blue creatures are legal targets"
    );
    assert!(
        !options.contains(&aven),
        "Possessed Aven is now black (not blue) at threshold"
    );
    game.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![phantom],
            players: vec![],
        },
    )
    .unwrap();
    let rest = drive_to_rest(&mut game, p0);
    assert!(matches!(rest, Rest::Reached), "{rest:?}");
    assert!(in_graveyard(&game, p1, index::PHANTOM_MONSTER).is_some());
}
