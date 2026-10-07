//! `cards/artifacts/mv_0/claws_of_gix.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Claws of Gix — {0} artifact: "{1}, Sacrifice a permanent: You gain 1 life."
///
/// "Permanent" is the whole card. The price names no type, so the engine has
/// to ask which of the seat's permanents is being given up, and
/// `Filter::ControlledByYou` is the only thing keeping the opponent's board
/// off that menu. The Elf is the answer worth giving, because a creature is
/// neither the artifact that asks nor a land: a menu quietly narrowed to one
/// of those would still offer something and still pass. The {1} is read twice
/// — absent from the offer on an empty pool, offered the moment the sources
/// are tapped — because `legal.abilities` is filtered through `can_afford`,
/// which reads the pool and not the untapped lands.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn claws_of_gix_eats_a_permanent_you_control_for_one_life() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(617, forest())
        .battlefield(0, &[claws_of_gix(), forest(), forest(), llanowar_elves()])
        .battlefield(1, &[forest(), llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let claws = on_battlefield(&engine, p0, claws_of_gix()).expect("the Claws are on the table");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(claws, 0)),
        "an empty pool pays no {{1}}, and an unaffordable ability is absent \
         from the offer rather than refused: {:?}",
        legal.abilities
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "two Forests and one Elf: every source on the board is tapped"
    );
    assert!(
        !is_tapped(&engine, claws),
        "and the Claws are not, because their price is not their own {{T}}"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(claws, 0)),
        "with {{1}} floating the whole price is payable: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, claws_of_gix(), 0);
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
            "the sacrifice is a cost and is asked before it is paid: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell them apart"
    );
    assert_eq!((min, max), (1, 1), "one permanent, no more and no fewer");
    assert!(
        options.contains(&claws),
        "the Claws are a permanent this seat controls, so they are on their \
         own menu: {options:?}"
    );
    assert!(
        options.contains(&elf),
        "and a creature is as much a permanent as an artifact: {options:?}"
    );
    assert_eq!(
        options.len(),
        4,
        "the Claws, the Elf and the two Forests — everything this seat has and \
         nothing else: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "`CR 701.21a`: an opponent's permanent is not yours to sacrifice: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the creature the question offered pays the cost");
    assert!(
        !stack_is_empty(&engine),
        "gaining life is no mana ability, so the Claws' ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        21,
        "\"You gain 1 life\" — one, and not one per permanent on the board"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the life belongs to the seat that paid the price"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "the sacrificed creature left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "a sacrificed permanent goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, claws_of_gix()).is_some(),
        "and only the permanent that was named: the Claws ate the Elf, not \
         themselves"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        2,
        "the {{1}} came out of the pool"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the opponent's board never moved"
    );
}
