//! `cards/creatures/mv_8/crash_of_rhinos.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Crash of Rhinos prints one line of rules text — trample — on an
/// `{6}{G}{G}` Rhino that is an 8/4, so the whole card is its cost, its body
/// and that keyword.
///
/// Eight Forests are the entire price, which is what makes the cast a payment
/// rather than a label: the pool is empty before it and empty after it, and
/// what arrives is the 8/4 the file prints with the keyword on it once the
/// layers have run. Printing a keyword is not playing it, so the test then
/// walks a turn — a creature that entered this turn could not have attacked
/// (CR 302.6) — and swings: eight unblocked combat damage is exactly the
/// number the printed power promises, and the attacker is still standing
/// afterwards.
#[test]
fn crash_of_rhinos_arrives_as_an_eight_four_trampler_and_attacks_for_eight() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 8])
        .hand(0, &[crash_of_rhinos()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches a main phase of its own"
    );

    // The whole price is on the board as eight untapped Forests and nothing is
    // floating: `legal.castable` is filtered through `can_afford`, which reads
    // the pool rather than the lands, so the mana goes in first and the claim
    // about the cast comes afterwards.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before the Forests are tapped"
    );
    cast_from_hand(&mut engine, p0, crash_of_rhinos());
    pass_until(&mut engine, stack_is_empty);

    let rhino = on_battlefield(&engine, p0, crash_of_rhinos()).expect("the Rhinos resolved");
    assert_eq!(
        pt(&engine, rhino),
        (8, 4),
        "the power and toughness the card prints"
    );
    assert!(
        keywords(&engine, rhino).contains(KeywordSet::TRAMPLE),
        "and the one keyword it prints, read after the layer system has run"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "eight Forests paid {{6}}{{G}}{{G}} and nothing was left floating"
    );

    // A creature that entered this turn may not attack (CR 302.6), so the
    // combat below needs a full turn cycle. Nothing on either board moves on
    // the way: the walkers declare empty attackers and blockers, and the
    // opponent's side of the table is its filler deck alone.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert_eq!(player, p0, "the active seat declares its own attackers");
    assert!(
        attackers.contains(&rhino),
        "untapped and past summoning sickness, the 8/4 is offered: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(rhino, Defender::Player(p1))],
            },
        )
        .expect("the attacker came out of the list that offered it");

    // The damage step is behind us by the end step (CR 510.2), so the life
    // totals are read there rather than the moment blockers are declared.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        12,
        "eight combat damage from the printed 8/4, with nothing in the way"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the attack was never blocked, so nothing hit back"
    );
    assert!(
        on_battlefield(&engine, p0, crash_of_rhinos()).is_some(),
        "the Rhinos survived their own attack"
    );
}
