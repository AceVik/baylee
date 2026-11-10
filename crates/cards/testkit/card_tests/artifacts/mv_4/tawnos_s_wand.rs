//! `cards/artifacts/mv_4/tawnos_s_wand.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tawnos's Wand: "{2}, {T}: Target creature with power 2 or less can't be
/// blocked this turn."
///
/// "Power 2 or less" is the menu (CR 601.2c, read against projected power):
/// the 2/2 Ogre and the 1/1 Elf across the table are offered, the 3/3 Giant
/// is not. The grant then takes exactly the target out of the blockers'
/// pairings (CR 509.1b), and the untargeted Giant stays in them — which is
/// what makes the empty pairing about the grant and not about a blocker that
/// simply could not block.
#[test]
fn tawnos_s_wand_points_at_power_two_or_less_and_that_creature_cannot_be_blocked() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(17, forest())
        .battlefield(
            0,
            &[
                tawnos_s_wand(),
                forest(),
                forest(),
                gray_ogre(),
                hill_giant(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);

    // The creatures start the game under p0's control, so the attack waits
    // for a turn that is not the first (summoning sickness, CR 302.6).
    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });

    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("the Ogre is out");
    let giant = on_battlefield(&engine, p0, hill_giant()).expect("the Giant is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, tawnos_s_wand(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the Wand targets a creature, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&ogre),
        "power 2 is \"2 or less\": {options:?}"
    );
    assert!(
        options.contains(&elf),
        "\"target creature\" reaches the other side of the table: {options:?}"
    );
    assert!(
        !options.contains(&giant),
        "power 3 is not offered: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ogre],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        keywords(e, ogre).contains(KeywordSet::UNBLOCKABLE)
    });
    assert!(
        !keywords(&engine, giant).contains(KeywordSet::UNBLOCKABLE),
        "only the target is granted anything"
    );
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::UNBLOCKABLE),
        "and the Elf across the table got nothing"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(ogre, Defender::Player(p1)), (giant, Defender::Player(p1))],
            },
        )
        .unwrap();
    let blockers = loop {
        match engine.pending().clone() {
            Pending::ChooseBlockers { blockers, .. } => break blockers,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while reaching blockers: {other:?}"),
        }
    };
    assert_eq!(blockers.len(), 1, "the one Elf can pair with one attacker");
    assert_eq!(
        blockers[0].attackers,
        vec![giant],
        "the Wand's target is not among the attackers it may be paired with"
    );
}
