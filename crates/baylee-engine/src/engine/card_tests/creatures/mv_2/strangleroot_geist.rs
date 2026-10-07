//! `cards/creatures/mv_2/strangleroot_geist.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Strangleroot Geist prints `{G}{G}` for a 2/1 Spirit with haste; the
/// undying line was the `Coverage::Partial` gap and is played by the test
/// below, so nothing here presses it and this stays a reading of haste.
///
/// Haste is a keyword bit the engine reads, and the only way to see it *do*
/// anything is the attack it permits: the Geist arrives in p0's first main
/// phase and is still offered as an attacker in that same turn's combat
/// step. The Llanowar Elves cast beside it is the control — untapped, so its
/// exclusion cannot be read as "it already paid for something", and no haste,
/// so being left off the list is summoning sickness and nothing else. The
/// attack itself then lands two damage, which is the printed body paid out
/// rather than a list merely containing a name.
#[test]
fn strangleroot_geist_attacks_the_turn_it_arrives_and_the_elves_beside_it_cannot() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(29, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[strangleroot_geist(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, strangleroot_geist());
    pass_until(&mut engine, stack_is_empty);
    let geist = on_battlefield(&engine, p0, strangleroot_geist()).expect("the Geist resolved");
    assert_eq!(pt(&engine, geist), (2, 1), "the body the card prints");
    assert!(
        keywords(&engine, geist).contains(KeywordSet::HASTE),
        "haste is a keyword bit the layer projection carries"
    );

    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves resolved");
    assert!(
        !is_tapped(&engine, elves),
        "the control is untapped, so only summoning sickness can keep it home"
    );

    // Out of the main phase and into the declare-attackers step.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player,
        attackers,
        defenders,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on nothing else")
    };
    assert_eq!(player, p0, "the turn is p0's");
    assert!(
        attackers.contains(&geist),
        "the Geist came under p0's control this turn and may attack anyway: {attackers:?}"
    );
    assert!(
        !attackers.contains(&elves),
        "the Elves came under p0's control this turn and are summoning sick: {attackers:?}"
    );

    let life_before = engine.state().players[1].life;
    assert_eq!(
        defenders.len(),
        1,
        "one surviving opponent and no planeswalkers of theirs: {defenders:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(geist, defenders[0])],
            },
        )
        .expect("the defender the offer enumerated is legal");
    pass_until(&mut engine, |e| e.state().players[1].life < life_before);
    assert_eq!(
        engine.state().players[1].life,
        life_before - 2,
        "two damage, the Geist's printed power, so the attack resolved rather \
         than merely being declared"
    );
}

/// Undying, and haste is what lets the return be seen in the turn it
/// happens.
///
/// Both creatures are killed in p0's own precombat main and both come back
/// with a +1/+1 counter, so the attacker list that follows differs in one
/// thing only: the Geist prints haste and the Wolf does not. A returned
/// permanent has not been controlled since the turn began (CR 302.6), which
/// is why the Wolf is the control rather than a second subject — the rule
/// itself is read in `engine::undying_tests`.
#[test]
fn strangleroot_geist_returns_bigger_and_haste_lets_it_attack_at_once() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(30, forest())
        .battlefield(0, &[strangleroot_geist(), young_wolf()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let geist = on_battlefield(&engine, p0, strangleroot_geist()).expect("the Geist is seated");
    let wolf = on_battlefield(&engine, p0, young_wolf()).expect("the Wolf is seated beside it");
    kill(&mut engine, geist);
    kill(&mut engine, wolf);

    let geist = on_battlefield(&engine, p0, strangleroot_geist()).expect("undying returned it");
    let wolf = on_battlefield(&engine, p0, young_wolf()).expect("and it returned the Wolf too");
    assert_eq!(
        pt(&engine, geist),
        (3, 2),
        "the printed 2/1 with a +1/+1 counter on it"
    );
    assert_eq!(pt(&engine, wolf), (2, 2), "and the printed 1/1 as a 2/2");
    assert!(
        keywords(&engine, geist).contains(KeywordSet::HASTE),
        "haste came back with it: it is printed on the card, not granted"
    );
    assert!(
        !keywords(&engine, wolf).contains(KeywordSet::HASTE),
        "and the control has none, which is the whole difference between them"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on nothing else")
    };
    assert!(
        attackers.contains(&geist),
        "the Geist arrived this turn and attacks anyway: {attackers:?}"
    );
    assert!(
        !attackers.contains(&wolf),
        "the Wolf arrived the same way and may not: {attackers:?}"
    );
}
