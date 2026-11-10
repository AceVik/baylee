//! `cards/enchantments/auras/mv_1/burrowing.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Burrowing: "Enchanted creature has mountainwalk." With the defending
/// player controlling a Mountain, the enchanted creature cannot be blocked
/// at all (CR 702.14c).
#[test]
fn burrowing_grants_mountainwalk_and_the_attacker_is_unblockable_against_a_mountain() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let burrowing = burrowing();
    // Two boards, differing only in whether the defender controls a
    // Mountain: the control proves `attack_and_collect_blocks` actually
    // offers the Elves as a blocker when mountainwalk doesn't apply, so the
    // Mountain board's empty offer is the landwalk and not a quirk of the
    // harness.
    for (land_name, defender_land, can_be_blocked) in
        [("Mountain", mountain(), false), ("Forest", forest(), true)]
    {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[mountain(), quiet_creature()])
            .battlefield(1, &[defender_land, quiet_creature()])
            .hand(0, &[burrowing])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let attacker = on_battlefield(&engine, p0, quiet_creature()).expect("attacker seated");

        // Tap only the Mountain: the Elves also carry a mana ability, and
        // `cast_from_hand` would tap everything that can pay, tapping the
        // attacker along with the land.
        let mountain_id = on_battlefield(&engine, p0, mountain()).expect("the Mountain");
        engine
            .apply(
                p0,
                PlayerAction::ActivateManaAbility {
                    source: mountain_id,
                },
            )
            .unwrap();
        cast_with_floating(&mut engine, p0, burrowing);
        aim_at(&mut engine, p0, attacker);
        pass_until(&mut engine, stack_is_empty);
        assert!(keywords(&engine, attacker).contains(KeywordSet::MOUNTAINWALK));
        assert!(
            !is_tapped(&engine, attacker),
            "the attacker stayed untapped"
        );

        let blockers = attack_and_collect_blocks(&mut engine, attacker, p1);
        assert_eq!(
            blockers.iter().any(|b| b.attackers.contains(&attacker)),
            can_be_blocked,
            "defender's land is a {land_name}, blockers: {blockers:?}"
        );
    }
}
