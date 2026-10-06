//! `cards/creatures/mv_1/manta_riders.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Manta Riders — {U} 1/1 Merfolk whose whole printed text is
/// "{U}: This creature gains flying until end of turn."
///
/// The keyword is read twice, because a keyword can be a label in the
/// characteristics and nothing else in the rules: once off the layer
/// projection after the activation, and once in the combat step, where an
/// opponent's Llanowar Elves — a creature with no evasion clause of its own —
/// must still be offered against the plain 1/1 attacking beside the Riders and
/// must not be offered against the flier. The two Islands pay the {U}, and the
/// pump carries a power and toughness of 0, so the body stays a (1, 1).
#[test]
fn manta_riders_buys_its_own_flying_and_the_ground_cannot_block_it() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), manta_riders(), llanowar_elves()])
        // The control: an untapped 1/1 with no evasion clause, which is a
        // legal blocker for an ordinary ground attacker and not for a flier.
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let riders = on_battlefield(&engine, p0, manta_riders()).expect("the Riders are out");
    let ground = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let blocker = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, riders), (1, 1), "a printed 1/1");
    assert!(
        !keywords(&engine, riders).contains(KeywordSet::FLYING),
        "a vanilla Merfolk until its own ability is paid for"
    );

    // Mana before the claim: `LegalActions` is filtered through
    // `can_afford`, which reads the pool and not the untapped Islands.
    // The Islands and not the Elves beside them: `tap_all_mana` taps every
    // source on the board, and the Elves are one — which would both add a
    // green nobody asked for and lie the control down before it can attack.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Islands, two blue — the Riders' own {{T}} is no mana ability"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds its own main phase: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(riders, 0)),
        "the {{U}} ability is offered once the mana is there: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, manta_riders(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "one of the two blue was the {{U}} the ability charges"
    );
    assert!(
        keywords(&engine, riders).contains(KeywordSet::FLYING),
        "Filter::This puts the keyword on the creature that printed it"
    );
    assert!(
        !keywords(&engine, ground).contains(KeywordSet::FLYING),
        "and on no other creature under the same seat"
    );
    assert!(
        !keywords(&engine, blocker).contains(KeywordSet::FLYING),
        "nor on anything across the table"
    );
    assert_eq!(
        pt(&engine, riders),
        (1, 1),
        "the pump grants the keyword and moves no numbers"
    );

    // The keyword as a rule and not a label: both 1/1s swing, and only one of
    // them the ground can get in front of.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&riders) && attackers.contains(&ground),
        "both untapped 1/1s may attack: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (riders, Defender::Player(p1)),
                    (ground, Defender::Player(p1)),
                ],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the block declaration")
    };
    let offered = blockers.iter().find(|o| o.blocker == blocker);
    assert!(
        offered.is_some_and(|o| o.attackers.contains(&ground)),
        "the Elf is a legal blocker for the ground 1/1 beside the flier, so \
         the reading below is about flying and not about an empty board: {blockers:?}"
    );
    assert!(
        !offered.is_some_and(|o| o.attackers.contains(&riders)),
        "and no legal blocker for a creature that gained flying (CR 702.9b): {blockers:?}"
    );
}
