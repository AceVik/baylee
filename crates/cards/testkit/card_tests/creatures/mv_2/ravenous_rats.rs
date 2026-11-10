//! Oracle: “When this creature enters, target opponent discards a card.”
#[allow(clippy::wildcard_imports)]
use super::*;

fn ravenous_rats() -> CardIndex {
    card_index("2fa1bbfd-92b5-482c-b32d-4cdc286474c4")
}

/// Checklist: the trigger targets an opponent, that player chooses a card,
/// and exactly one of two cards is discarded; the Rats remain on the board.
#[test]
fn ravenous_rats_make_the_opponent_discard_one_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut game = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[ravenous_rats()])
        .hand(1, &[dark_ritual(), unsummon()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    cast_from_hand(&mut game, p0, ravenous_rats());
    pass_until(&mut game, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        player_options,
        ..
    } = game.pending().clone()
    else {
        unreachable!("the entry trigger asks for its opponent target")
    };
    assert_eq!(player, p0);
    assert_eq!(player_options, vec![p1]);
    game.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![p1],
        },
    )
    .expect("the only opponent is a legal target");

    pass_until(&mut game, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = game.pending().clone()
    else {
        panic!(
            "the targeted opponent chooses a card to discard: {:?}",
            game.pending()
        )
    };
    assert_eq!(player, p1);
    assert_eq!((min, max), (1, 1));
    assert_eq!(options.len(), 2);
    let discarded = options[0];
    game.apply(
        p1,
        PlayerAction::ChooseObjects {
            objects: vec![discarded],
        },
    )
    .expect("one card is discarded");
    pass_until(&mut game, stack_is_empty);

    assert_eq!(game.state().zones.list(ZoneLocation::Hand(p1)).len(), 1);
    assert_eq!(
        game.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        1
    );
    assert_ne!(
        in_hand(&game, p1, dark_ritual()).is_some(),
        in_hand(&game, p1, unsummon()).is_some()
    );
    assert_ne!(
        in_graveyard(&game, p1, dark_ritual()).is_some(),
        in_graveyard(&game, p1, unsummon()).is_some()
    );
    assert!(on_battlefield(&game, p0, ravenous_rats()).is_some());
    assert_eq!(game.state().players[0].life, 20);
    assert_eq!(game.state().players[1].life, 20);
}
