//! `cards/sorceries/mv_4/flowstone_flood.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

fn flowstone_flood() -> CardIndex {
    card_index("7f77de5d-1088-42d0-8f68-5633aeab0d74")
}

#[test]
fn flowstone_flood_destroys_target_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .battlefield(1, &[forest(), gray_ogre()])
        .hand(0, &[flowstone_flood()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let target_land = on_battlefield(&engine, p1, forest()).unwrap();
    cast_from_hand(&mut engine, p0, flowstone_flood());
    let creature = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    kit::settle(&mut engine, p0, |_, p| {
        if let Pending::ChooseTargets { options, .. } = p {
            assert!(!options.contains(&creature));
        }
        kit::choose_targets(p, &[target_land], &[])
    });

    assert!(on_battlefield(&engine, p1, forest()).is_none());
    assert!(in_graveyard(&engine, p0, flowstone_flood()).is_some());
}
