#[allow(clippy::wildcard_imports)]
use super::*;

fn sunder() -> CardIndex {
    card_index("fddc93a2-912e-402a-9d0e-34f778e659ea")
}

#[test]
fn sunder_returns_all_lands_to_owners_hands() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[sunder()])
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .battlefield(1, &[forest(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, sunder());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, island()).is_none());
    assert!(on_battlefield(&engine, p1, forest()).is_none());
    assert!(in_hand(&engine, p1, forest()).is_some());
    assert!(on_battlefield(&engine, p1, gray_ogre()).is_some());
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len(),
        5
    );
}
