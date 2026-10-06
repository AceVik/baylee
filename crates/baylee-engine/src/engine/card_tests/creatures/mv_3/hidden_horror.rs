//! `cards/creatures/mv_3/hidden_horror.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hidden Horror is a `{1}{B}{B}` 4/4 whose one printed sentence is a price
/// asked as it enters: "sacrifice it unless you discard a creature card"
/// (`Effect::PlayerMayPayCostOr`). Both answers are played here, because
/// neither half alone is the card — the first board trades the Elf in hand for
/// the 4/4, and the second holds no creature card at all, which is the control
/// that tells a Horror kept by paying from one that merely survived.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn hidden_horror_trades_a_creature_card_for_itself_and_dies_without_one() {
    let p0 = PlayerId::new(0);

    // ---- the price is paid: a creature card is discarded and the Horror stays
    let mut engine = Duel::new(31, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[hidden_horror(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    cast_from_hand(&mut engine, p0, hidden_horror());

    // The trigger asks before it does anything, so the two questions it can
    // raise — "will you pay?" and "which card pays?" — are answered in the
    // order they arrive rather than in the order they are expected.
    let mut asked = false;
    for _ in 0..40 {
        if at_rest(&engine, p0) {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::YesNo { player, .. } => {
                engine.apply(player, PlayerAction::YesNo(true)).unwrap();
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                ..
            } => {
                assert_eq!(
                    prompt,
                    ChoicePrompt::CostDiscard,
                    "the discard is a price and not a search"
                );
                // Zero is a legal answer and that is the whole sentence:
                // "sacrifice it unless you discard a creature card" asks no
                // yes-or-no question, and naming nothing is how the price is
                // declined. The `max` is what says one card pays it.
                assert_eq!(
                    (min, max),
                    (0, 1),
                    "declining is naming nothing, and one card is the price"
                );
                assert_eq!(
                    options.len(),
                    1,
                    "and the only creature card in hand is the whole menu: {options:?}"
                );
                let elf = options
                    .iter()
                    .copied()
                    .find(|id| {
                        engine
                            .state()
                            .object(*id)
                            .is_some_and(|o| o.card.is_some_and(|c| c.index == llanowar_elves()))
                    })
                    .expect("the creature card in hand is on its own menu");
                engine
                    .apply(player, PlayerAction::ChooseObjects { objects: vec![elf] })
                    .unwrap();
                asked = true;
            }
            other => panic!("unexpected while the enters-trigger resolves: {other:?}"),
        }
    }
    assert!(
        asked,
        "a creature card was in hand, so the price was offered and paid"
    );
    let horror = on_battlefield(&engine, p0, hidden_horror())
        .expect("the price was paid, so the Horror is still on the battlefield");
    assert_eq!(pt(&engine, horror), (4, 4), "and it is the body it prints");
    assert!(
        in_hand(&engine, p0, llanowar_elves()).is_none()
            && in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "the creature card left the hand that gave it up and is in its owner's graveyard"
    );

    // ---- the price cannot be paid: no creature card, so the Horror dies
    let mut engine = Duel::new(31, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[hidden_horror()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    assert!(
        in_hand(&engine, p0, llanowar_elves()).is_none(),
        "the control: this seat holds no creature card to discard"
    );
    cast_from_hand(&mut engine, p0, hidden_horror());
    for _ in 0..40 {
        if at_rest(&engine, p0) {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::YesNo { player, .. } => {
                // Declining and never being asked are the same branch: with no
                // creature card in hand there is no price, and the `or` half
                // of the sentence is the Horror's own death.
                engine.apply(player, PlayerAction::YesNo(false)).unwrap();
            }
            other => panic!(
                "nothing in hand can pay `discard a creature card`, so the \
                 trigger has only its `or` branch left: {other:?}"
            ),
        }
    }
    assert!(
        in_graveyard(&engine, p0, hidden_horror()).is_some(),
        "with no creature card to discard, the enters-trigger sacrifices the Horror"
    );
    assert!(
        on_battlefield(&engine, p0, hidden_horror()).is_none(),
        "so it never stays on the battlefield"
    );
}
