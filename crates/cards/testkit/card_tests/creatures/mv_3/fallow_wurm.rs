//! `cards/creatures/mv_3/fallow_wurm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fallow Wurm is a `{2}{G}` 4/4 whose entire rules text is one price: "When
/// this creature enters, sacrifice it unless you discard a land card." Both
/// arms of that card are read off one board. With no land in hand there is
/// nothing to pay with — `PlayerMayPayCostOr` opens no question it has no
/// answer for — so the Wurm dies where it lands. With a Forest in hand the
/// same board asks for it: the seven Elves beside it are no lands and stay off
/// the menu, and the 4/4 is still standing once the Forest is gone. The Elf
/// filler is what makes "no land in hand" reachable at all, because a Forest
/// deck draws one.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn fallow_wurm_is_eaten_unless_a_land_card_is_discarded_for_it() {
    let p0 = PlayerId::new(0);
    let three_forests = [forest(), forest(), forest()];

    // No land in hand: every card drawn off the filler is an Elf, so the
    // printed price is unpayable and the trigger resolves without asking
    // anything at all.
    let mut engine = Duel::new(SEED, quiet_creature())
        .battlefield(0, &three_forests)
        .hand(0, &[fallow_wurm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, fallow_wurm());
    for _ in 0..40 {
        if at_rest(&engine, p0) {
            break;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!(
                "there is no land card to discard, so there is no price to ask \
                 about — the twin that does not ask what cannot be paid: got {:?}",
                engine.pending()
            )
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    assert!(
        in_graveyard(&engine, p0, fallow_wurm()).is_some(),
        "with nothing to discard the Wurm sacrifices itself"
    );
    assert!(
        on_battlefield(&engine, p0, fallow_wurm()).is_none(),
        "and it is not on the battlefield — nothing kept it there"
    );

    // The same board with one Forest in hand: the price is payable, so the
    // question is asked and answering it is what keeps the Wurm alive.
    let mut engine = Duel::new(SEED, quiet_creature())
        .battlefield(0, &three_forests)
        .hand(0, &[fallow_wurm(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, fallow_wurm());

    let mut asked_for_a_land = false;
    for _ in 0..40 {
        if at_rest(&engine, p0) {
            break;
        }
        match engine.pending().clone() {
            // The price is offered before it is charged.
            Pending::YesNo { player, .. } => {
                engine.apply(player, PlayerAction::YesNo(true)).unwrap();
            }
            Pending::ChooseCards {
                player,
                options,
                prompt,
                ..
            } => {
                asked_for_a_land = true;
                assert_eq!(
                    prompt,
                    ChoicePrompt::CostDiscard,
                    "paying the printed price is a discard cost and not a search"
                );
                assert_eq!(
                    options.len(),
                    1,
                    "\"discard a **land** card\": the seven Elves in hand are \
                     lands on no reading, so the Forest is the whole menu: {options:?}"
                );
                let land = options[0];
                assert!(
                    engine
                        .state()
                        .object(land)
                        .and_then(|o| o.card)
                        .is_some_and(|c| c.index == forest()),
                    "and the one card offered is the Forest that was dealt"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![land],
                        },
                    )
                    .expect("a card the question offered is a legal answer");
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the Wurm's trigger resolves: {other:?}"),
        }
    }

    assert!(
        asked_for_a_land,
        "a land card in hand is a price this Wurm can be paid with, so it is \
         asked for"
    );
    let wurm =
        on_battlefield(&engine, p0, fallow_wurm()).expect("the discard bought the Wurm its life");
    assert_eq!(
        pt(&engine, wurm),
        (4, 4),
        "and what was bought is the body the card prints"
    );
    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "the land card went to its owner's graveyard to pay the price"
    );
    assert!(
        in_hand(&engine, p0, forest()).is_none(),
        "and it left the hand, so it was discarded and not merely revealed"
    );
}
