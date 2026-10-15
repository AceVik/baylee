// Oracle checklist: At the beginning of your upkeep, each opponent discards a card.
// Exercise the named effect, target/trigger boundary, exact recipient and outcome.
#[allow(clippy::wildcard_imports)] // Shared card-test vocabulary.
use super::*;

fn subject() -> CardIndex {
    card_index("484e8e49-e604-4721-95a4-dd69eb11d4c6")
}

// Own upkeep forces each opponent, not the controller, to discard one card.
#[test]
fn own_upkeep_makes_opponent_discard_exactly_one() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut game = Duel::new(SEED, forest())
        .battlefield(0, &[subject()])
        .hand(0, &[gray_ogre()])
        .hand(1, &[wild_elephant(), llanowar_elves()])
        .start();
    keep_mulligans(&mut game);
    pass_until(&mut game, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::Generic,
                ..
            }
        )
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = game.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(game.state().turn.active, p0);
    assert_eq!(game.state().turn.step, crate::turn::Step::Upkeep);
    assert_eq!(player, p1);
    assert_eq!((min, max), (1, 1));
    let chosen = in_hand(&game, p1, wild_elephant()).unwrap();
    assert!(options.contains(&chosen));
    game.apply(
        p1,
        PlayerAction::ChooseObjects {
            objects: vec![chosen],
        },
    )
    .unwrap();
    pass_until(&mut game, stack_is_empty);
    assert!(in_graveyard(&game, p1, wild_elephant()).is_some());
    assert!(in_hand(&game, p1, llanowar_elves()).is_some());
    assert!(in_hand(&game, p0, gray_ogre()).is_some());
    // The opponent has a discardable card through their upkeep. No Hag prompt
    // may interrupt the trip to their main phase, and our hand remains intact.
    reach_their_main_phase(&mut game, p1);
    assert_eq!(game.state().turn.active, p1);
    assert_eq!(game.state().turn.phase, Phase::FirstMain);
    assert!(in_hand(&game, p0, gray_ogre()).is_some());
    assert!(in_hand(&game, p1, llanowar_elves()).is_some());
}
