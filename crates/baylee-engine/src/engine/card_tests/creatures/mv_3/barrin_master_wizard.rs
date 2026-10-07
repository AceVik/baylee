//! `cards/creatures/mv_3/barrin_master_wizard.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Barrin, Master Wizard — {1}{U}{U}, 1/1 legendary Human Wizard:
/// "{2}, Sacrifice a permanent: Return target creature to its owner's hand."
///
/// Neither half of the price names a particular permanent, so the engine has
/// to ask twice and each menu is half the card: "target creature" carries no
/// controller (CR 115.1), so the Elf across the table is on the first menu,
/// while "sacrifice a permanent" is read against CR 701.21a and offers only
/// what this seat controls — the same Elf must not appear on the second.
/// The {2} is read off the pool and not off the untapped lands, so the empty
/// pool before anything is tapped is the control on the one line the card
/// prints, and the two Islands' worth of mana is what pays for it.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn barrin_master_wizard_trades_a_permanent_for_an_opponents_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[barrin_master_wizard(), braidwood_cup(), island(), island()],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let barrin = on_battlefield(&engine, p0, barrin_master_wizard()).expect("Barrin is out");
    let fodder = on_battlefield(&engine, p0, braidwood_cup()).expect("the Cup is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(pt(&engine, barrin), (1, 1), "the body the card prints");

    // `legal.abilities` is filtered through `can_afford`, which reads the pool
    // rather than the untapped lands: nothing floats, so the {2} is unpayable
    // and the ability is absent from the offer altogether.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.contains(&(barrin, 0)),
        "an empty pool pays no {{2}}, so the offer is empty of Barrin: {:?}",
        legal.abilities
    );

    // Now the mana. Two Islands, and the Cup prints no mana ability of its
    // own — its whole text is a life gain, so `tap_all_mana` leaves it
    // standing while the lands pay the price Barrin charges.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Islands, and nothing off the artifact this test will sacrifice"
    );

    activate(&mut engine, p0, barrin_master_wizard(), 0);

    // CR 601.2c picks the target and CR 601.2h pays afterwards, so the two
    // questions arrive in that order — answered in the order they arrive
    // rather than the order they are expected in.
    let mut target_question: Vec<ObjectId> = Vec::new();
    let mut sacrifice_question: Vec<ObjectId> = Vec::new();
    for _ in 0..12 {
        if !target_question.is_empty() && !sacrifice_question.is_empty() {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player, options, ..
            } => {
                target_question = options;
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![theirs],
                        },
                    )
                    .expect("the Elf across the table was one of the options");
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
                    crate::choice::ChoicePrompt::CostSacrifice,
                    "a cost and not a search, which is all a client has to tell the two apart"
                );
                assert_eq!((min, max), (1, 1), "one permanent, no more and no fewer");
                sacrifice_question = options;
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![fodder],
                        },
                    )
                    .expect("the Cup the question offered pays the cost");
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while Barrin's activation resolves: {other:?}"),
        }
    }

    assert!(
        target_question.contains(&theirs) && target_question.contains(&barrin),
        "\"target creature\" names no controller, so both sides of the table \
         are on the menu: {target_question:?}"
    );
    assert!(
        !target_question.contains(&fodder),
        "the Cup is an artifact and no creature: {target_question:?}"
    );
    assert!(
        sacrifice_question.contains(&fodder) && sacrifice_question.contains(&barrin),
        "every permanent this seat controls is on the sacrifice menu, Barrin \
         included: {sacrifice_question:?}"
    );
    assert!(
        !sacrifice_question.contains(&theirs),
        "`CR 701.21a`: an opponent's creature is not yours to sacrifice, even \
         when it is the very creature that was named: {sacrifice_question:?}"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "the targeted creature left the battlefield"
    );
    assert!(
        in_hand(&engine, p1, llanowar_elves()).is_some(),
        "\"to its owner's hand\": the Elf goes back to the seat that owns it, \
         not to the seat that aimed the bounce"
    );
    assert!(
        in_graveyard(&engine, p0, braidwood_cup()).is_some(),
        "and the sacrificed permanent goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, barrin_master_wizard()).is_some(),
        "only the permanent that was named: Barrin ate the Cup, not himself"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}} came out of the two mana the Islands made"
    );
}
