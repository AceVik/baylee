//! `cards/creatures/mv_2/ledger_shredder.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// A Shredder cast as the second spell was not yet on the battlefield
/// to see its own casting. It still flies over a ground blocker.
#[test]
fn ledger_shredder_does_not_see_its_own_cast_and_flies_over_the_ground() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(6111, forest())
        .battlefield(0, &[island(), island(), forest()])
        .hand(0, &[llanowar_elves(), ledger_shredder()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The Elves go first: `{G}` can only come off the Forest and leaves the
    // two Islands in the pool for the Bird, so neither cast has to choose.
    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    cast_from_hand(&mut engine, p0, ledger_shredder());
    pass_until(&mut engine, stack_is_empty);

    let shredder = on_battlefield(&engine, p0, ledger_shredder()).expect("the Shredder resolved");
    let ground = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves resolved");
    assert_eq!(pt(&engine, shredder), (1, 3), "a 1/3 as printed");
    assert!(
        keywords(&engine, shredder).contains(KeywordSet::FLYING),
        "and the one keyword the card prints"
    );
    assert_eq!(
        pt(&engine, shredder),
        (1, 3),
        "it was not yet on the battlefield when the second spell was cast"
    );

    // A turn has to turn before either of them may attack.
    reach_their_main_phase(&mut engine, p1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let Pending::ChooseAttackers {
        attackers,
        defenders,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate above stops on nothing else")
    };
    assert!(
        attackers.contains(&shredder) && attackers.contains(&ground),
        "both are untapped and no longer summoning sick: {attackers:?}"
    );
    let target = *defenders.first().expect("the opponent is there to attack");
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(shredder, target), (ground, target)],
            },
        )
        .expect("a 1/3 flier and a 1/1 may attack the opponent");

    // The block step. The Elves across the table can block the ground
    // attacker, which is what makes their silence about the flier mean
    // something rather than being an empty menu.
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let mut pairings: Option<Vec<ObjectId>> = None;
    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseBlockers {
                player, blockers, ..
            } => {
                pairings = Some(
                    blockers
                        .iter()
                        .filter(|option| option.blocker == theirs)
                        .flat_map(|option| option.attackers.clone())
                        .collect(),
                );
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
                break;
            }
            other => panic!("expected the block step, got {other:?}"),
        }
    }
    let pairings = pairings.expect("their Elves can block, so the step is asked");
    assert!(
        pairings.contains(&ground),
        "a ground 1/1 may block a ground 1/1: {pairings:?}"
    );
    assert!(
        !pairings.contains(&shredder),
        "\"flying\" (CR 509.1b): the same blocker is offered the ground \
         attacker and never the Bird"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        is_tapped(&engine, shredder),
        "and it went through — attacking tapped it and nothing blocked it"
    );
}
