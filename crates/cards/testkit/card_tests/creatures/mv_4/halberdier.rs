//! `cards/creatures/mv_4/halberdier.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "1c3c23b5-b771-4117-83b5-febf7e96281a"

/// Halberdier is `{3}{R}` for a 3/1 with first strike and no other text, so
/// the whole card is that one keyword and the only board that reads it is a
/// combat step. A 3/1 dies to any damage at all, which is what makes the
/// exchange decisive: the Halberdier attacks, a Llanowar Elves blocks, and the
/// Elf has to be dead in the first-strike damage step before it can deal its
/// one point back. An engine that dealt damage simultaneously would trade the
/// pair, and the Halberdier still standing is what tells the two apart.
#[test]
fn halberdier_kills_its_blocker_in_the_first_strike_step_and_survives() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[halberdier()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The card arrives the way the card arrives: four Mountains pay {3}{R}.
    // Cast on p0's own turn, it cannot attack yet (CR 302.6), so the attack
    // below waits out the whole turn cycle.
    cast_from_hand(&mut engine, p0, halberdier());
    pass_until(&mut engine, stack_is_empty);
    let halberd = on_battlefield(&engine, p0, halberdier()).expect("the Halberdier resolved");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("the blocker is out");
    assert_eq!(pt(&engine, halberd), (3, 1), "the body the card prints");
    assert!(
        keywords(&engine, halberd).contains(KeywordSet::FIRST_STRIKE),
        "and the one keyword it prints, which is the whole of its text"
    );
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::FIRST_STRIKE),
        "the blocker is the control: one power and no first strike of its own"
    );

    // Round to p0's next main phase, where the Halberdier has been on the
    // battlefield since last turn and may be declared as an attacker.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let blocks = attack_and_collect_blocks(&mut engine, halberd, p1);
    let option = blocks
        .iter()
        .find(|b| b.blocker == elf)
        .expect("the Elves may block the Halberdier");
    assert!(
        option.attackers.contains(&halberd),
        "and the Halberdier is what they may block: {:?}",
        option.attackers
    );
    let Pending::ChooseBlockers { player, .. } = engine.pending().clone() else {
        panic!("expected the block declaration, got {:?}", engine.pending());
    };
    assert_eq!(player, p1, "the defending seat declares the blockers");
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elf, halberd)],
            },
        )
        .expect("the pairing came out of the list that offered it");

    // Not `stack_is_empty`: the stack is already empty the moment blockers are
    // declared, so that predicate stops the walk *before* combat damage. The
    // blocker's own death is the moment this test is about.
    pass_until(&mut engine, |e| {
        in_graveyard(e, p1, llanowar_elves()).is_some()
    });

    assert!(
        on_battlefield(&engine, p0, halberdier()).is_some(),
        "the Elf never dealt its one point back: a 3/1 survives only if the \
         damage went in the first-strike step, and a simultaneous engine would \
         have traded the two (CR 704.5g)"
    );
    assert_eq!(
        pt(&engine, halberd),
        (3, 1),
        "and it survived untouched rather than merely alive"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the attack was blocked, so nothing reached the player"
    );
}
