//! `cards/creatures/mv_4/moorish_cavalry.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Moorish Cavalry — {2}{W}{W} for a 3/3 Human Knight whose entire rules text
/// is "Trample".
///
/// A keyword the card prints and a keyword the damage step obeys are two
/// different claims, so the test makes both: the Cavalry is cast for real off
/// the four Plains and reads (3, 3) with trample through the layers, then it
/// is sent at the opponent and blocked by a printed 1/1 — one of the three
/// points is lethal for the blocker (CR 704.5g) and the other two spill past
/// it onto the defending player (CR 702.19b), which is the whole of what the
/// word buys. The turn in between is the summoning sickness the creature is
/// born with (CR 302.6), and the 1/1 is small enough that "lethal" and "the
/// excess" are told apart by the numbers rather than by a keyword of their
/// own.
#[test]
fn moorish_cavalry_arrives_as_a_trampling_three_three_and_spills_the_excess_over_its_blocker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .hand(0, &[moorish_cavalry()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, moorish_cavalry());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, moorish_cavalry()).is_some()
    });
    let cavalry = on_battlefield(&engine, p0, moorish_cavalry()).expect("the Cavalry resolved");
    assert_eq!(
        pt(&engine, cavalry),
        (3, 3),
        "the body the card prints, off the {{2}}{{W}}{{W}} the four Plains paid"
    );
    assert!(
        types(&engine, cavalry).contains(TypeSet::CREATURE),
        "a Human Knight is a creature and not a spell left on the stack"
    );
    assert!(
        keywords(&engine, cavalry).contains(KeywordSet::TRAMPLE),
        "the one line the card prints, projected through the layers"
    );

    // The creature arrived during this turn, so its first attack is its
    // controller's next one (CR 302.6). The walk answers every combat
    // declaration on the way with nothing, so nothing else moves either.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("the blocker is out");

    let blocks = attack_and_collect_blocks(&mut engine, cavalry, p1);
    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == elf && b.attackers.contains(&cavalry)),
        "the 1/1 across the table may block the trampler: {blocks:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elf, cavalry)],
            },
        )
        .expect("the pairing the engine offered is the pairing it accepts");

    // Past the combat damage step (CR 510.2): the end step is where both the
    // lethal-damage death and the assignment behind it have happened.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "one point of the three is lethal for a printed 1/1 (CR 704.5g)"
    );
    assert_eq!(
        engine.state().players[1].life,
        18,
        "and trample (CR 702.19b) sends the remaining two past the blocker to \
         the player rather than wasting them on a creature that is already dead"
    );
    assert!(
        on_battlefield(&engine, p0, moorish_cavalry()).is_some(),
        "the trampler took one point back and is still standing, so the two \
         damage above are excess and not a trade"
    );
}
