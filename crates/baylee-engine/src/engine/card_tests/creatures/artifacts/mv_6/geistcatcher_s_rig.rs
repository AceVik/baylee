//! `cards/creatures/artifacts/mv_6/geistcatcher_s_rig.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn geistcatcher_s_rig() -> CardIndex {
    card_index("1bfdbb96-cc18-4609-ba17-d169967df8d0")
}

/// Geistcatcher's Rig — "When this creature enters, you may have it deal 4
/// damage to target creature with flying." Only the flyer is a target; yes
/// kills it, no leaves it be.
#[test]
fn geistcatchers_rig_may_shoot_a_flyer_for_four() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for shoot in [true, false] {
        let mut engine = Duel::new(SEED, forest())
            .hand(0, &[geistcatcher_s_rig()])
            .battlefield(
                0,
                &[forest(), forest(), forest(), forest(), forest(), forest()],
            )
            .battlefield(1, &[gray_ogre(), aven_envoy()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let envoy = on_battlefield(&engine, p1, aven_envoy()).expect("seated");
        let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("seated");

        cast_from_hand(&mut engine, p0, geistcatcher_s_rig());
        let options = pass_until_targets(&mut engine, p0);
        assert!(options.contains(&envoy));
        assert!(!options.contains(&ogre), "\"with flying\"");
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![envoy],
                },
            )
            .expect("the flyer is a legal target");
        pass_until(&mut engine, |e| {
            matches!(
                e.pending(),
                Pending::YesNo {
                    prompt: crate::choice::YesNoPrompt::MayDo,
                    ..
                }
            )
        });
        engine.apply(p0, PlayerAction::YesNo(shoot)).expect("may");
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            in_graveyard(&engine, p1, aven_envoy()).is_some(),
            shoot,
            "4 damage kills the 0/2 only when the Rig is told to"
        );
        assert!(on_battlefield(&engine, p1, gray_ogre()).is_some());
        let rig = on_battlefield(&engine, p0, geistcatcher_s_rig()).expect("the Rig");
        assert_eq!(pt(&engine, rig), (4, 5));
    }
}

/// Without a flyer anywhere there is nothing to target, so nothing is asked.
#[test]
fn geistcatchers_rig_does_nothing_without_a_flyer() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[geistcatcher_s_rig()])
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, geistcatcher_s_rig());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, geistcatcher_s_rig()).is_some());
    assert!(
        on_battlefield(&engine, p1, gray_ogre()).is_some(),
        "the ground Ogre is not a target"
    );
}
