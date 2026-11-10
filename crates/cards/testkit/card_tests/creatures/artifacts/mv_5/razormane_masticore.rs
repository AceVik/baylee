//! `cards/creatures/artifacts/mv_5/razormane_masticore.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Razormane Masticore` prints `First strike`, upkeep sacrifice unless discard, and draw step damage to target creature with `Coverage::Implemented`.
/// In this scenario, seat 0 starts with `Razormane Masticore` on the battlefield and a card in hand, while seat 1 controls a 1/1 `llanowar_elves()`.
/// During the turn 1 upkeep, paying the discard cost preserves the masticore on the battlefield.
/// The player on the play has no draw step on turn 1 (CR 103.8a), so the
/// second trigger waits for turn 3, whose upkeep is paid the same way; then it
/// targets and deals 3 damage to the opponent's elf, destroying it.
#[test]
fn razormane_masticore_pays_upkeep_discard_and_shoots_at_draw_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[razormane_masticore()])
        .hand(0, &[forest(), forest()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);

    let masticore = on_battlefield(&engine, p0, razormane_masticore()).expect("masticore seated");
    assert_eq!(pt(&engine, masticore), (5, 5));
    assert!(keywords(&engine, masticore).contains(KeywordSet::FIRST_STRIKE));
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their elf seated");

    // Turn 1 upkeep trigger: discard a card or sacrifice.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on ChooseCards");
    };
    assert_eq!(prompt, ChoicePrompt::CostDiscard);
    assert!(!options.is_empty(), "hand provides discard options");
    engine
        .apply(
            player,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("discarding pays the upkeep cost");

    assert!(
        on_battlefield(&engine, p0, razormane_masticore()).is_some(),
        "`Razormane Masticore` survives having paid the upkeep discard"
    );

    // Turn 3: the upkeep is paid again, and the draw step trigger targets a
    // creature.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on ChooseCards");
    };
    assert_eq!(engine.state().turn.number, 3, "p0's next upkeep");
    engine
        .apply(
            player,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("discarding pays the upkeep cost again");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player: target_player,
        options: target_options,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on ChooseTargets");
    };
    assert!(target_options.contains(&their_elf));
    engine
        .apply(
            target_player,
            PlayerAction::ChooseObjects {
                objects: vec![their_elf],
            },
        )
        .expect("targeting their elf");

    // The MayDo prompt resolves the choice to deal damage.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine
        .apply(p0, PlayerAction::YesNo(true))
        .expect("choose to deal 3 damage");

    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the 1/1 elf was destroyed by 3 damage"
    );
}
