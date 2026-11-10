//! `cards/creatures/mv_4/anaba_bodyguard.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Anaba Bodyguard — `{3}{R}` — Creature — Minotaur 2/3: "First strike."
///
/// A keyword that only reorders a combat damage step is worth nothing on an
/// idle board, so the scenario is a trade it turns into a one-sided kill: a
/// 3/1 Vendilion Clique blocks the Bodyguard, and a 2/3 and a 3/1 that deal
/// damage to each other *simultaneously* destroy one another. With first
/// strike the Bodyguard's two damage land in the first combat damage step, so
/// the Clique is already in the graveyard (CR 704.5g) when the regular step
/// would have let it strike back — which is why the assertion is the
/// Bodyguard's survival and not merely its printed stat line. The Clique also
/// carries flying, which is a fact about *blocking* fliers and not about
/// blocking: a creature without flying may block a flier, so the pairing is
/// legal and the turn still has to reach the damage step.
#[test]
fn anaba_bodyguard_kills_its_blocker_before_the_blocker_can_strike_back() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .battlefield(1, &[vendilion_clique()])
        .hand(0, &[anaba_bodyguard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, anaba_bodyguard());
    pass_until(&mut engine, stack_is_empty);
    let bodyguard = on_battlefield(&engine, p0, anaba_bodyguard()).expect("the Bodyguard resolved");
    assert_eq!(pt(&engine, bodyguard), (2, 3), "the body the card prints");
    assert!(
        keywords(&engine, bodyguard).contains(KeywordSet::FIRST_STRIKE),
        "and the one line of text it prints, read through the layers"
    );

    let clique = on_battlefield(&engine, p1, vendilion_clique()).expect("a 3/1 flier is out");
    assert!(
        keywords(&engine, clique).contains(KeywordSet::FLYING),
        "an evasion keyword on the blocker, which just means it may also \
         block a flier rather than that it can only be blocked by one"
    );

    // The Bodyguard entered this turn, so it cannot attack (CR 302.6): the
    // first strike is read off the attack it makes on the turn after next.
    //
    // It attacks rather than blocks, and that is the card's own sentence
    // rather than a convenience: flying says a creature may be blocked
    // *only* by flying or reach (CR 702.9b), so a Bodyguard on the ground
    // could never have been offered the Clique as an attacker to stop. The
    // other way round it is an ordinary block — a flier may block anything.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. }) && e.state().turn.active == p0
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert!(
        attackers.contains(&bodyguard),
        "a turn has passed, so CR 302.6 is spent and the Bodyguard may attack"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bodyguard, baylee_core::ids::Defender::Player(p1))],
            },
        )
        .expect("the attacker came out of the offer");

    // CR 508.2 hands priority round after attackers are declared, so the
    // block is the *next* question and not this one.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers {
        player,
        attacker,
        blockers,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p1, "the defending seat declares the blockers");
    assert_eq!(
        attacker, p0,
        "and the attacking seat is the one whose turn it is"
    );
    let option = blockers
        .iter()
        .find(|b| b.blocker == clique)
        .expect("the untapped Clique may block");
    assert!(
        option.attackers.contains(&bodyguard),
        "a flier may block a creature on the ground: {option:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(clique, bodyguard)],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert!(
        in_graveyard(&engine, p1, vendilion_clique()).is_some(),
        "two damage is lethal to a 3/1 (CR 704.5g)"
    );
    assert!(
        on_battlefield(&engine, p0, anaba_bodyguard()).is_some(),
        "and the first combat damage step is the whole reason it is still \
         standing: damage dealt simultaneously by a 2/3 and a 3/1 kills both, \
         which is what a Bodyguard without the keyword would show"
    );
}
