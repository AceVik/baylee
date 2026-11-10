//! `cards/creatures/mv_7/boldwyr_intimidator.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::subtypes;

fn boldwyr_intimidator() -> CardIndex {
    card_index("6ad1d8d0-06c0-4804-af5f-60ffb358f371")
}

/// "{R}: Target creature becomes a Coward until end of turn." and "Cowards
/// can't block Warriors." The Intimidator is a Giant Warrior. Before the
/// ability the opposing Ogre may block it; once the Ogre is made a Coward it
/// may not.
#[test]
fn boldwyr_intimidator_cowards_cannot_block_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table7(&[boldwyr_intimidator(), mountain()], &[], &[gray_ogre()]);
    let boldwyr = on_battlefield(&engine, p0, boldwyr_intimidator()).expect("out");
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("out");
    assert!(
        pt(&engine, boldwyr) == (5, 5)
            && engine
                .state()
                .object(boldwyr)
                .expect("out")
                .characteristics()
                .subtypes
                .contains(subtypes::creature::WARRIOR),
        "a 5/5 Warrior"
    );

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, boldwyr_intimidator(), 1);
    let _ = aim_at(&mut engine, p0, ogre);
    pass_until(&mut engine, stack_is_empty);
    let ogre_subtypes = engine
        .state()
        .object(ogre)
        .expect("out")
        .characteristics()
        .subtypes;
    assert!(
        ogre_subtypes.contains(subtypes::creature::COWARD),
        "the targeted Ogre is a Coward"
    );

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(boldwyr, Defender::Player(p1))],
            },
        )
        .expect("the Intimidator attacks");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("the pass stopped here")
    };
    assert!(
        !blockers
            .iter()
            .any(|b| b.blocker == ogre && b.attackers.contains(&boldwyr)),
        "a Coward Ogre may not block a Warrior"
    );
}

/// The control for the test above: with no Coward on the table the Ogre is
/// offered the block.
#[test]
fn boldwyr_intimidator_is_blockable_by_an_ordinary_creature() {
    let (mut engine, boldwyr, ogre) = duel_vs_ogre(boldwyr_intimidator(), &[]);
    let blocks = attack_and_collect_blocks(&mut engine, boldwyr, PlayerId::new(1));
    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == ogre && b.attackers.contains(&boldwyr))
    );
}

/// "{2}{R}: Target creature becomes a Warrior until end of turn": the types
/// are replaced, so the Ogre is a Warrior and no longer an Ogre.
#[test]
fn boldwyr_intimidator_makes_a_creature_a_warrior() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table7(
        &[boldwyr_intimidator(), mountain(), mountain(), mountain()],
        &[],
        &[gray_ogre()],
    );
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("out");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, boldwyr_intimidator(), 2);
    let _ = aim_at(&mut engine, p0, ogre);
    pass_until(&mut engine, stack_is_empty);
    let subtypes_now = engine
        .state()
        .object(ogre)
        .expect("out")
        .characteristics()
        .subtypes;
    assert!(subtypes_now.contains(subtypes::creature::WARRIOR));
    assert!(!subtypes_now.contains(subtypes::creature::OGRE));
}
