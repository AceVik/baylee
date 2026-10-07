//! `cards/creatures/mv_3/pegasus_charger.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Pegasus Charger is `{2}{W}` for a 2/1 Pegasus whose whole text is flying
/// and first strike, and both words only ever matter in combat, so the card
/// is cast and then attacks. Flying is read off the block question rather
/// than off the card file: the Elf across the table may block the ground
/// attacker beside the Charger and may not be assigned the Charger, while a
/// Baleful Strix — the pool's flier — may block either. First strike is then
/// read off the aftermath: a 2/1 that blocks-and-dies to a 1/1 in the
/// ordinary damage step instead kills it in the first one and never takes the
/// counter-blow, so the Strix is in its owner's graveyard and the Charger is
/// still standing.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn pegasus_charger_flies_over_the_ground_and_strikes_first() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains(), llanowar_elves()])
        .hand(0, &[pegasus_charger()])
        // A flier and a ground creature across the table: the two halves of
        // the block question the flying line is read from.
        .battlefield(1, &[baleful_strix(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The card arrives the way the card arrives: {2}{W} out of three Plains,
    // with the Elf named as the source kept back — it is the ground attacker
    // the block question below needs, and `tap_all_mana` would have spent its
    // own printed `{T}: Add {G}`.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Plains, and the Elf is still standing to attack with"
    );
    cast_with_floating(&mut engine, p0, pegasus_charger());
    pass_until(&mut engine, stack_is_empty);

    let charger = on_battlefield(&engine, p0, pegasus_charger()).expect("the Charger resolved");
    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    assert_eq!(pt(&engine, charger), (2, 1), "the body the card prints");
    let granted = keywords(&engine, charger);
    assert!(granted.contains(KeywordSet::FLYING), "printed flying");
    assert!(
        granted.contains(KeywordSet::FIRST_STRIKE),
        "printed first strike"
    );
    assert!(
        !keywords(&engine, my_elf).contains(KeywordSet::FLYING),
        "the projection reaches the Charger and no other creature on the board"
    );

    // One full turn cycle: a creature that has not been under its controller's
    // control since their last untap step may not attack (CR 302.6).
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&charger) && attackers.contains(&my_elf),
        "both untapped creatures may attack: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (charger, Defender::Player(p1)),
                    (my_elf, Defender::Player(p1)),
                ],
            },
        )
        .unwrap();

    // CR 508.2: priority goes round once the attack is declared, so the
    // block question is not the next pending.
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
        panic!(
            "two attackers declare a block step, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p1, "the defending seat declares");
    assert_eq!(attacker, p0, "against the attacking seat");

    let strix = on_battlefield(&engine, p1, baleful_strix()).expect("the Strix is out");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let may_block = |blocker: ObjectId| -> Vec<ObjectId> {
        blockers
            .iter()
            .find(|option| option.blocker == blocker)
            .unwrap_or_else(|| panic!("{blocker:?} is on the block menu"))
            .attackers
            .clone()
    };

    let strix_may_block = may_block(strix);
    assert!(
        strix_may_block.contains(&charger),
        "flying blocks flying: {strix_may_block:?}"
    );
    let elf_may_block = may_block(their_elf);
    assert!(
        elf_may_block.contains(&my_elf),
        "an untapped ground creature may still block the ground attacker: {elf_may_block:?}"
    );
    assert!(
        !elf_may_block.contains(&charger),
        "\"can't be blocked except by creatures with flying or reach\": the Elf \
         has neither, so the flier is not one of the attackers it may be \
         assigned to: {elf_may_block:?}"
    );

    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(strix, charger)],
            },
        )
        .expect("the question offered exactly this pairing");
    // Not `stack_is_empty`: the stack is already empty the moment blockers are
    // declared, so that would stop the walk before the first-strike damage
    // step. The end step is past both damage steps (CR 510.2).
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert!(
        in_graveyard(&engine, p1, baleful_strix()).is_some(),
        "first strike deals its damage first, so two damage to a printed 1/1 \
         is lethal before the ordinary damage step ever begins"
    );
    assert!(
        on_battlefield(&engine, p0, pegasus_charger()).is_some(),
        "and the Strix was gone by then, so it never dealt its one damage — \
         which is lethal to a 2/1 and would have taken the Charger with it"
    );
    assert_eq!(
        engine.state().players[1].life,
        19,
        "the unblocked Elf got through for one, and the Charger's damage went \
         to the blocker that was assigned to it and never to the player"
    );
}
