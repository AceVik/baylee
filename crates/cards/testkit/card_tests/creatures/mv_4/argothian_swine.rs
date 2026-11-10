//! `cards/creatures/mv_4/argothian_swine.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Argothian Swine is `{3}{G}` for a 3/3 Boar whose whole rules text is
/// "Trample", so the keyword is the only thing a scenario can prove and it has
/// to be proved by attacking. The cast reads the body the card prints; the
/// attack has to wait a turn, because a creature that arrived this turn is sick
/// (CR 302.6) and the combat step offers it nothing. The 1/1 blocker is what
/// tells trample from a plain 3/3: without the keyword the Swine would spend its
/// single lethal point on the Elf and stop, and the defending player's life
/// total would never move.
#[test]
fn argothian_swine_arrives_as_a_three_three_and_tramples_over_its_blocker() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[argothian_swine()])
        .battlefield(1, &[llanowar_elves()])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, argothian_swine());
    pass_until(&mut engine, stack_is_empty);

    let swine = on_battlefield(&engine, p0, argothian_swine()).expect("the Swine resolved");
    assert_eq!(
        pt(&engine, swine),
        (3, 3),
        "{{3}}{{G}} buys the 3/3 the card prints"
    );
    assert!(
        keywords(&engine, swine).contains(KeywordSet::TRAMPLE),
        "and the one line under the body reaches the permanent"
    );

    // One turn has to pass before the Swine may attack at all. Both walks
    // answer the other seat's combat with an empty declaration, so p1's Elf
    // never leaves the ground.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until only stops on an attack declaration")
    };
    assert!(
        attackers.contains(&swine),
        "untapped and past its first turn, the Swine may attack: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(swine, Defender::Player(p1))],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("pass_until only stops on a block declaration")
    };
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is still out");
    let option = blockers
        .iter()
        .find(|o| o.blocker == elf)
        .expect("the defending creature is offered as a blocker");
    assert!(
        option.attackers.contains(&swine),
        "and the Swine is one of the attackers it may be paired with: {option:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elf, swine)],
            },
        )
        .unwrap();

    // Past the combat damage step (CR 510.2). Not `stack_is_empty`: the stack
    // is already empty the moment blockers are declared, so that predicate
    // would stop the walk before a single point had been dealt.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "one point of damage is lethal to a printed 1/1, so the blocker died"
    );
    assert_eq!(
        engine.state().players[1].life,
        18,
        "\"Trample\" is the whole difference on this board: the two damage \
         past the lethal point went on to the player instead of nowhere"
    );
    assert!(
        on_battlefield(&engine, p0, argothian_swine()).is_some(),
        "and the Swine survived the 1/1 it ran over"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the blocker's one point of power could not reach a 3/3"
    );
}
