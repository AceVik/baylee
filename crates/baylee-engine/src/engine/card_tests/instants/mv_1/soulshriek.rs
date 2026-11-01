//! `cards/instants/mv_1/soulshriek.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn soulshriek() -> CardIndex {
    card_index("77ae796e-fc56-47b9-b8a0-97e24fbd60f7")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn hill_giant() -> CardIndex {
    baylee_core::generated::index::HILL_GIANT
}

/// Soulshriek (`{B}`, `Coverage::Implemented`): target creature you control
/// gets +X/+0 until end of turn, where X is the number of creature cards in
/// your graveyard, and is sacrificed at the beginning of the next end step.
#[test]
fn soulshriek_pumps_by_graveyard_creatures_and_sacrifices_at_end_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                swamp(),
                grizzly_bears(),
                hill_giant(),
                gray_ogre(),
                giant_growth(),
            ],
        )
        .battlefield(1, &[savannah_lions()])
        .hand(0, &[soulshriek()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, p0, hill_giant()).unwrap();
    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    let growth = on_battlefield(&engine, p0, giant_growth()).unwrap();
    let lions = on_battlefield(&engine, p1, savannah_lions()).unwrap();
    bury(&mut engine, &[giant, ogre, growth]);

    cast_from_hand(&mut engine, p0, soulshriek());
    let menu = aim_at(&mut engine, p0, bear);
    assert!(menu.contains(&bear));
    assert!(
        !menu.contains(&lions),
        "only a creature you control may be named"
    );
    pass_until(&mut engine, stack_is_empty);

    // 2 creature cards in graveyard (Hill Giant, Gray Ogre) -> +2/+0 (4/2).
    assert_eq!(pt(&engine, bear), (4, 2));

    // At the beginning of the next end step, Grizzly Bears is sacrificed.
    reach_their_main_phase(&mut engine, p1);
    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_none());
    assert!(in_graveyard(&engine, p0, grizzly_bears()).is_some());
    assert!(
        on_battlefield(&engine, p1, savannah_lions()).is_some(),
        "the opponent's creature is not sacrificed"
    );
}
