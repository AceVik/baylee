//! `cards/lands/utility/buried_ruin.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Buried Ruin: "{2}, {T}, Sacrifice this land: Return target artifact card
/// from your graveyard to your hand."
///
/// Three clauses of that sentence are only readable by playing it. **Your**
/// graveyard: an artifact card lying in the opponent's is not an answer, and
/// a `PlayerRel` read as "any" would offer it. **Artifact** card: a creature
/// card in your own graveyard is not an answer either, and the two wrong
/// readings are different bugs, so both are on the table at once — while the
/// only artifact card in the game sits across from you the ability is
/// withheld outright, which is `ability_has_a_target` agreeing with `apply`
/// rather than the client lighting up a land that refuses the click.
///
/// And the sacrifice is a **cost**, not the effect: the land is already in
/// its owner's graveyard while the ability is still sitting on the stack, so
/// a card that spent it as part of resolving would return the artifact and
/// keep the land. The two mana are floating before any of this is asked, so
/// an ability that is not offered is not offered for want of a target.
#[test]
fn buried_ruin_pays_itself_into_the_graveyard_and_returns_only_your_own_artifact_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(311, forest())
        .battlefield(0, &[buried_ruin(), forest(), forest()])
        .hand(0, &[quiet_artifact(), llanowar_elves()])
        .hand(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let ruin = on_battlefield(&engine, p0, buried_ruin()).expect("the Ruin is on the table");

    // A creature card in my graveyard and an artifact card in theirs: every
    // near miss the target spec has to reject, and nothing it may accept.
    let elf = bury_from_hand(&mut engine, p0, llanowar_elves());
    let theirs = bury_from_hand(&mut engine, p1, quiet_artifact());
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "an artifact card really is lying in the other graveyard, so the \
         refusal below is about whose it is"
    );

    // Its own tap is part of the cost, so the Ruin is the one land that must
    // not be spent on the {2}.
    tap_mana_except(&mut engine, p0, ruin);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("still priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(ruin, 1)),
        "with two mana floating and no artifact card of my own in the \
         graveyard, the ability has nothing to point at: {:?}",
        legal.abilities
    );

    let mine = bury_from_hand(&mut engine, p0, quiet_artifact());
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("still priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(ruin, 1)),
        "and with one it is offered: {:?}",
        legal.abilities
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: ruin,
                ability_index: 1,
            },
        )
        .expect("two Forests pay {2}");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the return asks which card: {:?}", engine.pending())
    };
    assert!(
        options.contains(&mine),
        "my own artifact card is the answer: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "a creature card in the same graveyard is not an artifact card: \
         {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "and an artifact card in their graveyard is not in *your* graveyard: \
         {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the one legal target");

    // CR 601.2h: the costs are paid on activation. The land is gone before
    // anything resolves.
    assert!(
        !stack_is_empty(&engine),
        "the ability is on the stack and has not resolved yet"
    );
    assert!(
        on_battlefield(&engine, p0, buried_ruin()).is_none(),
        "the sacrifice is a cost, so the land left the battlefield to pay it"
    );
    assert!(
        in_graveyard(&engine, p0, buried_ruin()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard (CR 701.21a)"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_hand(&engine, p0, quiet_artifact()).is_some(),
        "the artifact card came back to hand"
    );
    assert!(
        in_graveyard(&engine, p0, quiet_artifact()).is_none(),
        "and it is no longer in the graveyard it came from"
    );
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "their artifact card was never touched"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "nor was the creature card lying beside mine"
    );
}
