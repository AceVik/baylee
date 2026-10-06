//! `cards/creatures/mv_3/skyknight_legionnaire.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Skyknight Legionnaire prints one body and two words: `{1}{R}{W}` for a 2/2
/// Human Knight with flying and haste. Haste is the half a card file cannot
/// show, so it is read where the engine reads it — the attack declaration of
/// the very turn the creature arrived, which CR 302.6 would otherwise bar it
/// from — and flying is read in the blocking pairing, where the flyer across
/// the table is offered as a blocker and the ground creature beside it is not.
/// Mountain, Plains and Forest pay the `{R}`, the `{W}` and the generic, so
/// both coloured symbols are real payments, and the two damage that reach the
/// opponent are the printed 2/2.
#[test]
fn skyknight_legionnaire_flies_and_attacks_the_turn_it_lands() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), plains(), forest()])
        .hand(0, &[skyknight_legionnaire()])
        // The block question only exists if somebody may block, so the other
        // seat gets a flyer — the card under test, the one flying creature
        // these tests already name — beside a ground creature, which is the
        // half that flying is supposed to decline.
        .battlefield(1, &[skyknight_legionnaire(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    // Tolerant of the opponent's turn: it declares empty attackers, so no
    // creature across the table is tapped when the blocking step below asks.
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, skyknight_legionnaire());
    pass_until(&mut engine, stack_is_empty);
    let knight = on_battlefield(&engine, p0, skyknight_legionnaire())
        .expect("the Legionnaire resolved onto the battlefield");
    assert_eq!(pt(&engine, knight), (2, 2), "the body the card prints");
    let printed = keywords(&engine, knight);
    assert!(printed.contains(KeywordSet::FLYING), "flying");
    assert!(printed.contains(KeywordSet::HASTE), "and haste");

    // CR 302.6: a creature that arrived this turn may not attack unless
    // something gives it haste, so a Legionnaire missing from this offer is
    // the keyword never having reached the permanent.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&knight),
        "haste: it arrived this turn and the combat step still offers it: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(knight, Defender::Player(p1))],
            },
        )
        .unwrap();

    // Flying, read in the pairing rather than in the keyword alone.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers {
        player, blockers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but the block declaration")
    };
    assert_eq!(player, p1, "the seat being attacked declares the blockers");
    let their_flyer = on_battlefield(&engine, p1, skyknight_legionnaire())
        .expect("their Legionnaire is on the battlefield");
    let their_ground =
        on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are on the battlefield");
    assert!(
        blockers
            .iter()
            .any(|option| option.blocker == their_flyer && option.attackers.contains(&knight)),
        "a creature that flies may block one: {blockers:?}"
    );
    assert!(
        !blockers
            .iter()
            .any(|option| option.blocker == their_ground && option.attackers.contains(&knight)),
        "CR 702.9b: neither flying nor reach, so it may not block a flying \
         attacker: {blockers:?}"
    );
    engine
        .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    // Past the damage step (CR 510.2). `stack_is_empty` is already true the
    // moment attackers are declared, so the ending phase is what the life
    // total below needs.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        18,
        "two unblocked damage, which is the power the card prints"
    );
}
