//! `cards/lands/utility/plaza_of_heroes.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn plaza_of_heroes_produces_restricted_legendary_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(339, forest())
        .battlefield(0, &[plaza_of_heroes()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    activate(&mut engine, p0, plaza_of_heroes(), 1);
    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::White);
    assert_eq!(pool.available(ManaColor::White), 0);
    let land = on_battlefield(&engine, p0, plaza_of_heroes()).expect("land on battlefield");
    assert!(is_tapped(&engine, land));
}

#[test]
fn plaza_adds_unrestricted_mana_from_current_legendary_permanents_only() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for phased in [false, true] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(
                0,
                &[
                    index::PLAZA_OF_HEROES,
                    index::ARCHANGEL_AVACYN,
                    index::GRIZZLY_BEARS,
                ],
            )
            .battlefield(1, &[index::BIRGI_GOD_OF_STORYTELLING])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let dog = on_battlefield(&e, p0, index::ARCHANGEL_AVACYN).unwrap();
        assert!(on_battlefield(&e, p1, index::BIRGI_GOD_OF_STORYTELLING).is_some());
        if phased {
            e.dev_state_mut(p0).unwrap().phase_out(&[dog]);
            e.refresh_offer();
        }
        activate(&mut e, p0, index::PLAZA_OF_HEROES, 3);
        let pool = &e.state().players[0].mana_pool;
        assert_eq!(pool.total(), u64::from(!phased));
        assert_eq!(pool.available(ManaColor::White), u32::from(!phased));
        assert!(pool.restricted().is_empty());
        assert!(stack_is_empty(&e));
    }
}
