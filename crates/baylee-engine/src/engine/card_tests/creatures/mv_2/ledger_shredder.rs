//! `cards/creatures/mv_2/ledger_shredder.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ledger Shredder prints `{1}{U}` for a 1/3 flying Bird Advisor, and the
/// connive trigger — "whenever a player casts their second spell each turn"
/// — is the half this printing does not have.
///
/// So two spells are cast in one first main phase, which is exactly the
/// second spell the missing trigger is about, and the Bird is a 1/3 with no
/// counter on it when the stack clears. The keyword the card *does* print is
/// played rather than read: a turn later the Shredder and a ground Elves
/// attack together, and the blocker the opponent's own Elves is offered for
/// is the ground attacker and never the flier — flying is a pairing question
/// (CR 509.1b), and an offer that simply omitted the Bird would say the same
/// on a board holding nothing that could block anything at all.
#[test]
fn ledger_shredder_flies_over_the_ground_and_never_connives() {
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
        "two spells in one turn and the Bird is untouched: the second-spell \
         connive is the `Coverage::Partial` gap, so no card was drawn and no \
         +1/+1 counter landed"
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
