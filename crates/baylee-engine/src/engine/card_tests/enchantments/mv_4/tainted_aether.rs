#[allow(clippy::wildcard_imports)]
use super::*;

fn tainted_aether() -> CardIndex {
    card_index("a61ceda1-5993-479e-945f-15753eeb7049")
}

#[test]
fn tainted_aether_makes_controller_sacrifice_creature_or_land_when_creature_enters() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[gray_ogre()])
        .battlefield(0, &[tainted_aether(), mountain(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, gray_ogre());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    let ogre_id = on_battlefield(&engine, p0, gray_ogre()).expect("ogre entered");
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![ogre_id],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, gray_ogre()).is_none());
    assert!(in_graveyard(&engine, p0, gray_ogre()).is_some());
}
