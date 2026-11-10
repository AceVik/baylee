//! `cards/creatures/mv_3/fleshgrafter.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fleshgrafter — {2}{B}, a 2/2 — prints one line: "Discard an artifact card:
/// This creature gets +2/+2 until end of turn."
///
/// The cost is the whole card, so two cards sit in hand and the discard menu
/// has to offer the artifact card while declining the land card beside it:
/// "an artifact card" read and not skipped. The pump is then read off the
/// Grafter *and* off a bare Elf standing next to it, because `Filter::This`
/// names one creature and not the seat's board.
#[test]
#[allow(clippy::too_many_lines)]
fn fleshgrafter_discards_an_artifact_card_for_two_power_and_leaves_the_board_alone() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[fleshgrafter(), llanowar_elves()])
        .hand(0, &[quiet_artifact(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let grafter = on_battlefield(&engine, p0, fleshgrafter()).expect("the Grafter is on the table");
    let bystander = on_battlefield(&engine, p0, llanowar_elves()).expect("an Elf beside it");
    let fodder = in_hand(&engine, p0, quiet_artifact()).expect("an artifact card in hand");
    let spare = in_hand(&engine, p0, island()).expect("a land card in the same hand");
    assert_eq!(
        pt(&engine, grafter),
        (2, 2),
        "a printed 2/2 before anything"
    );
    assert_eq!(pt(&engine, bystander), (1, 1), "and the Elf a printed 1/1");

    activate(&mut engine, p0, fleshgrafter(), 0);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the discard is a cost and is asked before it is paid: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        ChoicePrompt::CostDiscard,
        "a cost and not an effect, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one card, no more and no fewer");
    assert_eq!(
        options,
        vec![fodder],
        "the artifact card is the whole menu: {options:?}"
    );
    assert!(
        !options.contains(&spare),
        "\"an artifact card\" — a land card in the same hand is not one: {options:?}"
    );

    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![spare],
                },
            )
            .is_err(),
        "an answer the question did not enumerate is refused"
    );
    assert!(
        in_hand(&engine, p0, island()).is_some(),
        "and the refusal costs nothing: the land card is still in hand"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("the card the question offered pays the cost");

    assert!(
        in_graveyard(&engine, p0, quiet_artifact()).is_some(),
        "CR 601.2h: the price is paid as the ability is activated, so the \
         artifact card is already in its owner's graveyard"
    );
    assert!(
        in_hand(&engine, p0, island()).is_some(),
        "and nothing else was discarded"
    );
    assert_eq!(
        pt(&engine, grafter),
        (2, 2),
        "nothing has resolved yet: the ability is on the stack"
    );
    assert!(
        !stack_is_empty(&engine),
        "the pump is no mana ability, so it is waiting to resolve"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, grafter),
        (4, 4),
        "+2/+2 until end of turn on the creature the ability names"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf beside it is untouched: `Filter::This` is one creature, not \
         the seat's board"
    );
}
