//! `cards/artifacts/mv_3/ashnod_s_altar.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ashnod's Altar ({3}): "Sacrifice a creature: Add {C}{C}."
///
/// The inversion of the test that stood here. The cost names no creature,
/// and while an activation had nowhere to ask which one, `can_afford`
/// refused `CostPart::Sacrifice` outright and the Altar was never offered
/// at all — so the card stood at `Coverage::Partial` and this test asserted
/// an empty offer. `cost_wizard` asks the question now, and the Altar plays
/// the line it prints.
///
/// Reading the card cannot replace playing it, because every half of the
/// sentence is the engine's answer rather than the card's. Which creatures
/// the question offers is a board reading (`CR 701.21a`: a player
/// sacrifices only a permanent *they control*), so the opponent's Elves are
/// the counter-half and so is the Altar itself, which is an artifact and no
/// creature — a filter that let either one in would read the same in the
/// card file. And "Add {C}{C}" is a mana ability (`CR 605.1`), so it uses no
/// stack (`CR 605.3b`) and the mana is in the pool the moment the answer is
/// applied, with the creature already in its owner's graveyard.
///
/// The refused answer is the other probe: the engine validates against the
/// very list it published, so naming the opponent's Elves is rejected and
/// the question still stands.
#[allow(clippy::too_many_lines)] // one activation, every gate it passes asserted
#[test]
fn ashnods_altar_eats_the_creature_you_name_and_pays_two_colorless() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(71, forest())
        .battlefield(0, &[ashnods_altar(), llanowar_elves(), forest()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let altar = on_battlefield(&engine, p0, ashnods_altar()).expect("the Altar stands");
    let fodder = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves stand");
    let land = on_battlefield(&engine, p0, forest()).expect("my Forest stands");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves stand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(altar, 0)),
        "the Altar's only line is offered now that a cost can ask: {:?}",
        legal.abilities
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before the Altar eats"
    );

    activate(&mut engine, p0, ashnods_altar(), 0);
    let Pending::ChooseCards {
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("the cost asks which creature, got {:?}", engine.pending())
    };
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "the variant is what tells a client this is a cost and not a search"
    );
    assert_eq!((min, max), (1, 1), "one creature, and the cost asks once");
    assert_eq!(
        options,
        vec![fodder],
        "the creature you control is the whole of the answer"
    );
    assert!(
        !options.contains(&altar),
        "the Altar is an artifact: it cannot eat itself"
    );
    assert!(!options.contains(&land), "a land is no creature");
    assert!(
        !options.contains(&theirs),
        "`CR 701.21a`: an opponent's creature is not yours to sacrifice"
    );

    let refused = engine.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![theirs],
        },
    );
    assert!(
        matches!(
            refused,
            Err(EngineError::IllegalAction(why))
                if why == crate::choice::AnswerFault::NotOffered.reason()
        ),
        "the answer is validated against the list that was published: {refused:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("the creature the question offered pays the cost");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        2,
        "`Add {{C}}{{C}}` is in the pool the moment the answer lands"
    );
    assert_eq!(pool.total(), 2, "and nothing else came with it");
    assert!(
        stack_is_empty(&engine),
        "`CR 605.3b`: a mana ability never uses the stack"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "the sacrificed creature left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "and is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the opponent's Elves never moved"
    );
}
