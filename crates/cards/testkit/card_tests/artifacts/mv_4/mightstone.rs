//! `cards/artifacts/mv_4/mightstone.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mightstone — {4} artifact: "Attacking creatures get +1/+0."
///
/// The static prints neither a controller nor a duration, so one board reads
/// both words it does print: an Elf of mine that attacks is a 2/1 while an Elf
/// of mine standing beside it and the defending Elf are still printed 1/1s,
/// and the next turn's attack by the *opponent's* Elf is pumped by the same
/// artifact — which is what "attacking creatures" without "you control" means.
/// Walking past the combat is the other half: the pump is keyed to the
/// attacking state rather than granted for the turn, so the attacker is a
/// plain 1/1 again once the combat phase is behind it.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn mightstone_pumps_exactly_the_creatures_that_are_attacking() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[mightstone()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays at home");
    let (my_attacker, home) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(
        pt(&engine, my_attacker),
        (1, 1),
        "a printed 1/1 before the pump"
    );

    // The artifact arrives the way the artifact arrives: {4} off the four
    // Forests, with both Elves named as the printing kept back — they are the
    // creatures this test reads afterwards, and a 1/1 tapped for mana may not
    // attack.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Forests tapped, four green, and neither Elf contributed"
    );
    cast_with_floating(&mut engine, p0, mightstone());
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        on_battlefield(&engine, p0, mightstone()).is_some(),
        "the Mightstone resolved"
    );
    assert_eq!(
        pt(&engine, my_attacker),
        (1, 1),
        "a static that pumps attackers pumps nothing while nobody is attacking"
    );

    // Only the Elf that is declared gets the +1/+0. The one beside it and the
    // one across the table are two different bystanders: "attacking" is not
    // "creatures", and the defending creature is on the board as the control.
    let blocks = attack_and_collect_blocks(&mut engine, my_attacker, p1);
    assert!(
        blocks
            .iter()
            .any(|o| o.blocker == theirs && o.attackers.contains(&my_attacker)),
        "their Elf is the blocker the offer names, so it is really on the \
         battlefield and really untapped: {blocks:?}"
    );
    assert_eq!(
        pt(&engine, my_attacker),
        (2, 1),
        "the attacking creature gets +1/+0 — power up, toughness untouched"
    );
    assert_eq!(
        pt(&engine, home),
        (1, 1),
        "the Elf that stayed at home is still the 1/1 it was printed as"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and so is the defender, which is not attacking"
    );

    // No blocks, so the Elf survives its own attack: what takes the +1/+0 away
    // is the end of combat and not the end of the creature.
    let Pending::ChooseBlockers { player, .. } = engine.pending().clone() else {
        panic!(
            "expected the declare-blockers question, got {:?}",
            engine.pending()
        )
    };
    engine
        .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("declaring no blockers is always legal");
    pass_until(&mut engine, |e| pt(e, my_attacker).0 == 1);
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the Elf survived the turn it attacked in"
    );
    assert_eq!(
        pt(&engine, my_attacker),
        (1, 1),
        "the pump lasts exactly as long as the creature is attacking, and \
         nothing on the card says \"until end of turn\""
    );
    assert_eq!(
        pt(&engine, home),
        (1, 1),
        "and the Elf at home never changed at all"
    );

    // "attacking creatures" is not "attacking creatures you control": the
    // opponent's Elf is armed by my artifact on its own attack.
    reach_their_main_phase(&mut engine, p1);
    attack_and_collect_blocks(&mut engine, theirs, p0);
    assert_eq!(
        pt(&engine, theirs),
        (2, 1),
        "the Elf across the table attacks and the Mightstone pumps it, which a \
         filter carrying `ControlledByYou` would not do"
    );
    assert_eq!(
        pt(&engine, my_attacker),
        (1, 1),
        "while my Elf, which is not attacking, stays the printed 1/1"
    );
    assert_eq!(
        pt(&engine, home),
        (1, 1),
        "and so does the one that never moved"
    );
    assert!(
        on_battlefield(&engine, p0, mightstone()).is_some(),
        "the artifact outlives both attacks"
    );
}
