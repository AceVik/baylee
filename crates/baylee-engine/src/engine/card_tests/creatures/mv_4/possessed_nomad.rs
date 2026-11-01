//! `cards/creatures/mv_4/possessed_nomad.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::index;

#[test]
fn possessed_nomad_gains_plus_one_becomes_black_and_destroys_white_creature_at_threshold() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("9bf9870e-32a3-494c-bd42-f9f013463619");
    let mut board = vec![card];
    board.extend_from_slice(&[swamp(); 10]);
    let mut game = Duel::new(SEED, plains())
        .battlefield(0, &board)
        .battlefield(1, &[index::SAVANNAH_LIONS, index::GRIZZLY_BEARS])
        .hand(0, &[swamp()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let nomad = on_battlefield(&game, p0, card).unwrap();
    let lions = on_battlefield(&game, p1, index::SAVANNAH_LIONS).unwrap();
    let bear = on_battlefield(&game, p1, index::GRIZZLY_BEARS).unwrap();
    assert_eq!(pt(&game, nomad), (3, 3));
    assert!(keywords(&game, nomad).contains(KeywordSet::VIGILANCE));
    assert_eq!(
        game.state().object(nomad).unwrap().characteristics().colors,
        ColorSet::from_slice(&[Color::White])
    );

    let swamps = all_on_battlefield(&game, p0, swamp());
    bury(&mut game, &swamps[..6]);
    game.refresh_offer();
    tap_all_mana(&mut game, p0);
    let Pending::Priority { legal, .. } = game.pending() else {
        panic!("expected priority");
    };
    assert!(
        !legal.abilities.contains(&(nomad, 0)),
        "threshold ability must not be offered with 6 cards in graveyard"
    );

    bury(&mut game, &[swamps[6]]);
    game.refresh_offer();
    play_land(&mut game, p0, swamp());
    assert_eq!(pt(&game, nomad), (4, 4));
    assert_eq!(
        game.state().object(nomad).unwrap().characteristics().colors,
        ColorSet::from_slice(&[Color::Black])
    );
    activate(&mut game, p0, card, 0);
    let Pending::ChooseTargets { options, .. } = game.pending().clone() else {
        panic!("expected ChooseTargets");
    };
    assert!(options.contains(&lions));
    assert!(!options.contains(&bear));
    assert!(!options.contains(&nomad));
    game.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![lions],
            players: vec![],
        },
    )
    .unwrap();
    let rest = drive_to_rest(&mut game, p0);
    assert!(matches!(rest, Rest::Reached), "{rest:?}");
    assert!(in_graveyard(&game, p1, index::SAVANNAH_LIONS).is_some());
}
