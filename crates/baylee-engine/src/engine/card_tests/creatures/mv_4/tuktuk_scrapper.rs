//! `cards/creatures/mv_4/tuktuk_scrapper.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tuktuk Scrapper prints one rally trigger whose two halves are read on
/// opposite sides of the table: it destroys a target artifact and then burns
/// *that artifact's* controller for the number of Allies its own controller
/// has. The board here is the smallest one that can tell every word of the
/// sentence apart — the only artifact in play is the opponent's Sol Ring, so
/// "target artifact" has exactly one legal object, and the Llanowar Elves
/// standing beside the Scrapper is a creature but no Ally, so the count is one
/// (the Scrapper itself) and not two. The two life totals then say which seat
/// the damage went to, and the body of the Scrapper says the count was read as
/// Allies and not as creatures.
#[test]
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
fn tuktuk_scrapper_destroys_an_artifact_and_burns_its_controller_for_each_ally() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                quiet_creature(),
            ],
        )
        .hand(0, &[tuktuk_scrapper()])
        .battlefield(1, &[quiet_artifact()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("the Elf is out");
    let ring = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");

    // Four Mountains and the Elf are the whole of this board's mana, and the
    // trigger that follows costs none of it: nothing below is a claim about
    // what could have been tapped.
    cast_from_hand(&mut engine, p0, tuktuk_scrapper());
    assert!(
        on_battlefield(&engine, p0, tuktuk_scrapper()).is_none(),
        "the Scrapper is a spell on the stack, not yet a permanent"
    );

    // The Scrapper enters, "this creature … is an Ally you control" makes its
    // own rally trigger fire, and the trigger asks for its target as it is put
    // on the stack.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });

    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(
        player, p0,
        "the Scrapper's controller is the one that aims the trigger"
    );
    assert_eq!(
        (min, max),
        (0, 1),
        "\"you may destroy target artifact\" is an up-to-one choice, and \
         naming nothing is how the may is declined"
    );
    assert_eq!(
        options,
        vec![ring],
        "the opponent's Sol Ring is the only artifact on the battlefield — the \
         Mountains are lands and the Elf is a creature, and neither is an \
         artifact: {options:?}"
    );
    assert!(
        player_options.is_empty(),
        "\"target artifact\" names objects and no seats: {player_options:?}"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "the target is named while the artifact is still on the battlefield"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .expect("the artifact the question enumerated is a legal answer");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "\"destroy target artifact\": the artifact went to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "and it left the battlefield, which is what \"put into a graveyard \
         this way\" turns on"
    );
    assert_eq!(
        engine.state().players[1].life,
        19,
        "\"deals damage to that artifact's controller\" — one, for the single \
         Ally the Scrapper has: itself. The Elf beside it is a creature and no \
         Ally, so a count over creatures would have said two, and a count that \
         skipped the source would have said nothing at all"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the seat that cast the Scrapper takes nothing: the damage belongs to \
         the artifact's controller and not to the trigger's"
    );
    assert!(
        engine.state().object(elf).is_some(),
        "the creature that is no Ally never moved"
    );
    assert!(
        on_battlefield(&engine, p0, tuktuk_scrapper()).is_some(),
        "the Scrapper outlives its own trigger"
    );
}
