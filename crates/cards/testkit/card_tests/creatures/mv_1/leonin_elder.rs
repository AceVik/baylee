//! `cards/creatures/mv_1/leonin_elder.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Leonin Elder — {W} 1/1 Cat Cleric: "Whenever an artifact enters, you
/// may gain 1 life."
///
/// Both halves of the "may" are played, and "an artifact" is read on both
/// sides of the table: the Elder's controller is asked when their own Sol
/// Ring arrives and when the opponent's does, which is what separates this
/// trigger from an "artifact you control" one. A Llanowar Elves entering is
/// the other half of the filter — it asks nothing, so a life total that
/// moved for it would be a trigger that never read the word "artifact".
///
/// Mana is floated before anything is cast (CR 500.5 keeps it until the step
/// ends, and every cast here happens inside p0's first main phase), because
/// whether a spell is castable is read off the pool — and every source on
/// this board is a land whose whole price is its own tap.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn leonin_elder_offers_a_life_for_an_artifact_entering_on_either_side_of_the_table() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                forest(),
                leonin_elder(),
            ],
        )
        .hand(0, &[quiet_artifact(), quiet_creature(), quiet_artifact()])
        .battlefield(1, &[island(), island()])
        .hand(1, &[quiet_artifact()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Four Plains and a Forest, and nothing else on the board makes mana:
    // the Elder is a 1/1 whose whole text is one triggered ability.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "the whole board is five lands"
    );

    // The filter's other half: a creature entering is not an artifact
    // entering, and the engine has nothing to ask about it.
    cast_with_floating(&mut engine, p0, quiet_creature());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        20,
        "\"whenever an *artifact* enters\" — the Elves are a creature"
    );

    // The first Sol Ring: the question is asked, and declined.
    cast_with_floating(&mut engine, p0, quiet_artifact());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the question it was told to find")
    };
    assert_eq!(
        player, p0,
        "the Elder's controller is asked, and not the artifact's"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the life is offered rather than given: answering is still to come"
    );
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        20,
        "\"you may\" — declining buys nothing, so the trigger is optional \
         and not an upkeep"
    );

    // The second Sol Ring: the same question, answered the other way.
    cast_with_floating(&mut engine, p0, quiet_artifact());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        21,
        "one life per artifact, whatever else is standing on the board"
    );

    // And "an artifact" is not "an artifact you control": the opponent's own
    // Sol Ring asks the Elder's controller for the life.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, quiet_artifact());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the question it was told to find")
    };
    assert_eq!(
        player, p0,
        "the artifact is the opponent's; the question belongs to the Elder"
    );
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        22,
        "a Sol Ring cast across the table pays the Elder's controller"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the seat that cast it gains nothing"
    );
}
