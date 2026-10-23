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

#[test]
fn tainted_aether_uses_entrants_controller_and_accepts_land_but_not_artifact() {
    use crate::engine::card_tests::legends_kit as lk;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[tainted_aether(), gray_ogre()],
        &[forest()],
        &[],
        &[sol_ring(), gray_ogre()],
    );
    let mine = lk::obj(&e, p0, gray_ogre());
    let land = lk::obj(&e, p1, forest());
    reach_their_main_phase(&mut e, p1);
    lk::float(&mut e, p1, &[(ManaColor::Red, 4)]);
    lk::cast_at(&mut e, p1, sol_ring(), &[], &[]);
    let artifact = lk::obj(&e, p1, sol_ring());
    assert!(
        on_battlefield(&e, p1, forest()).is_some(),
        "noncreature entry triggers nothing"
    );
    cast_with_floating(&mut e, p1, gray_ogre());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player, options, ..
    } = e.pending()
    else {
        unreachable!()
    };
    assert_eq!(*player, p1);
    assert!(options.contains(&land));
    assert!(!options.contains(&mine) && !options.contains(&artifact));
    e.apply(
        p1,
        PlayerAction::ChooseObjects {
            objects: vec![land],
        },
    )
    .unwrap();
    lk::settle(&mut e);
    assert!(on_battlefield(&e, p0, gray_ogre()).is_some());
    assert!(on_battlefield(&e, p1, gray_ogre()).is_some());
    assert!(in_graveyard(&e, p1, forest()).is_some());
}
