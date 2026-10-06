//! `cards/creatures/mv_3/tempest_drake.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tempest Drake — `{1}{W}{U}` — a 2/2 Drake with flying and vigilance, and
/// both keywords are claims about combat rather than about the card file. So
/// the drake is cast and walked into an attack on its controller's *next*
/// turn, beside a grounded Elf, against a board holding a grounded Elf and a
/// flier: the defending Elf is offered as a blocker for the grounded attacker
/// and never for the drake (CR 702.9b), and the drake alone is still untapped
/// after the declaration that tapped the Elf beside it (vigilance, CR 702.20b).
/// Flying is read as a *pairing* and vigilance against a creature that lacks
/// it, because either assertion on its own is satisfied by a board that never
/// asked the question.
#[test]
fn tempest_drake_flies_over_a_ground_creature_and_attacks_without_tapping() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), island(), llanowar_elves()])
        .hand(0, &[tempest_drake()])
        // Their grounded Elf is what keeps the flying claim from being empty;
        // their Strix is the one creature that may legally block a flier.
        .battlefield(1, &[llanowar_elves(), baleful_strix()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, tempest_drake());
    pass_until(&mut engine, stack_is_empty);
    let drake = on_battlefield(&engine, p0, tempest_drake()).expect("the Drake resolved");
    assert_eq!(pt(&engine, drake), (2, 2), "the printed 2/2 body");
    let kw = keywords(&engine, drake);
    assert!(
        kw.contains(KeywordSet::FLYING),
        "Flying reaches the permanent"
    );
    assert!(kw.contains(KeywordSet::VIGILANCE), "and so does vigilance");

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");

    // A creature cast this turn is summoning sick (CR 302.6), so the attack
    // waits for its controller's next turn.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(drake, Defender::Player(p1)), (elf, Defender::Player(p1))],
            },
        )
        .expect("both untapped creatures may attack");

    // Vigilance is read against a creature that lacks it: the Elf beside the
    // drake is tapped by the very declaration that left the drake standing.
    assert!(
        !is_tapped(&engine, drake),
        "\"vigilance\": attacking does not tap the drake (CR 702.20b)"
    );
    assert!(
        is_tapped(&engine, elf),
        "while the grounded attacker beside it paid the usual price"
    );

    // Flying is legible in the pairing the defender is offered (CR 702.9b)
    // — after the priority round CR 508.2 hands out, which is why the block
    // is not the pending the declaration left behind.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        panic!(
            "a flier may block, so the declaration happens: {:?}",
            engine.pending()
        )
    };
    let strix = on_battlefield(&engine, p1, baleful_strix()).expect("their Strix is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert!(
        blockers
            .iter()
            .any(|o| o.blocker == strix && o.attackers.contains(&drake)),
        "a creature with flying may block the drake: {blockers:?}"
    );
    assert!(
        !blockers
            .iter()
            .any(|o| o.blocker == theirs && o.attackers.contains(&drake)),
        "and the grounded Elf may not: {blockers:?}"
    );
    assert!(
        blockers
            .iter()
            .any(|o| o.blocker == theirs && o.attackers.contains(&elf)),
        "the same Elf is offered against the grounded attacker, so the \
         exclusion above is about flying and not about a creature that can \
         block nothing at all: {blockers:?}"
    );

    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        17,
        "2 + 1 went through unblocked: nothing turned the flying half aside"
    );
    assert_eq!(pt(&engine, drake), (2, 2), "and the body is untouched");
}
