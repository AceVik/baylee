//! `cards/artifacts/mv_5/predator_flagship.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn predator_flagship() -> CardIndex {
    card_index("647c1339-9508-4bb9-8e77-ebc3dfdf4069")
}

/// Will-o'-the-Wisp: {B} Creature — Spirit with flying.
fn will_o_the_wisp() -> CardIndex {
    card_index("8b60fcfe-fb90-4a00-a708-25b59bfc9b5a")
}

/// "{2}: Target creature gains flying until end of turn."
#[test]
fn two_mana_give_a_creature_flying_until_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[predator_flagship(), forest(), forest(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    assert!(!keywords(&engine, ogre).contains(KeywordSet::FLYING));

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, predator_flagship(), 0);
    aim_at(&mut engine, p0, ogre);
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, ogre).contains(KeywordSet::FLYING), "now flying");
    assert_eq!(pt(&engine, ogre), (2, 2), "and nothing else changed");

    reach_their_main_phase(&mut engine, p1);
    assert!(
        !keywords(&engine, ogre).contains(KeywordSet::FLYING),
        "until end of turn"
    );
}

/// "{5}, {T}: Destroy target creature with flying." Only flyers are on the
/// menu, so the flightless Ogre is not.
#[test]
fn it_destroys_a_creature_with_flying_and_only_one_with_flying() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[predator_flagship(), forest(), forest(), forest(), forest(), forest()],
        )
        .battlefield(1, &[gray_ogre(), will_o_the_wisp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let flyer = on_battlefield(&engine, p1, will_o_the_wisp()).unwrap();
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, predator_flagship(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("a target, got {:?}", engine.pending())
    };
    assert!(options.contains(&flyer), "{options:?}");
    assert!(!options.contains(&ogre), "no flying, no target: {options:?}");
    aim_at(&mut engine, p0, flyer);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p1, will_o_the_wisp()).is_some());
    assert!(on_battlefield(&engine, p1, gray_ogre()).is_some());
}
