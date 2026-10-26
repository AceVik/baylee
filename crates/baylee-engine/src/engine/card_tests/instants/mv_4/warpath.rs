//! `cards/instants/mv_4/warpath.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

// oracle_id = "413520c7-5227-406f-8861-e844c153e5b0"
fn warpath() -> CardIndex {
    card_index("413520c7-5227-406f-8861-e844c153e5b0")
}

/// `Warpath` (`Coverage::Implemented`):
/// "Warpath deals 3 damage to each blocking creature and each blocked creature."
#[test]
fn warpath_deals_three_damage_to_blocking_and_blocked_creatures_only() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(9719, mountain())
        .battlefield(
            0,
            &[
                index::TRAINED_ARMODON,
                index::GRAY_OGRE,
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[index::GIANT_SPIDER])
        .hand(0, &[warpath()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let armodon = on_battlefield(&engine, p0, index::TRAINED_ARMODON).expect("Trained Armodon"); // 3/3
    let ogre = on_battlefield(&engine, p0, index::GRAY_OGRE).expect("Gray Ogre"); // 2/2
    let spider = on_battlefield(&engine, p1, index::GIANT_SPIDER).expect("Giant Spider"); // 2/4

    // P0 attacks with both Trained Armodon and Gray Ogre; P1 blocks Trained Armodon with Giant Spider,
    // leaving Gray Ogre unblocked.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (armodon, Defender::Player(p1)),
                    (ogre, Defender::Player(p1)),
                ],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(spider, armodon)],
            },
        )
        .unwrap();

    // Still in DeclareBlockers step: P0 casts Warpath.
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, warpath());
    pass_until(&mut engine, stack_is_empty);

    // Trained Armodon (blocked 3/3) dies to 3 damage; Giant Spider (blocking 2/4) has 3 damage marked;
    // Gray Ogre (unblocked 2/2) takes 0 damage from Warpath.
    assert!(
        in_graveyard(&engine, p0, index::TRAINED_ARMODON).is_some(),
        "Blocked 3/3 creature dies to Warpath's 3 damage"
    );
    assert_eq!(
        engine.state().object(spider).unwrap().damage,
        3,
        "Blocking 2/4 creature takes 3 damage from Warpath"
    );
    assert_eq!(
        engine.state().object(ogre).unwrap().damage,
        0,
        "Unblocked attacking creature takes 0 damage from Warpath"
    );
}
