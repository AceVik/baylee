//! `cards/creatures/mv_3/yavimaya_granger.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn yavimaya_granger() -> CardIndex {
    card_index("fac9ff8d-8342-4099-b224-d48a044bcc9a")
}

#[test]
fn yavimaya_granger_searches_library_for_a_basic_land_tapped_on_entry() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[yavimaya_granger()])
        .battlefield(0, &[forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, yavimaya_granger());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCards");
    };
    let picked_land = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![picked_land],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let landed = engine.state().object(picked_land).unwrap();
    assert_eq!(landed.zone, Zone::Battlefield);
    assert!(is_tapped(&engine, picked_land));
    // Its echo upkeep is a real price, even after its entry benefit resolved.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { .. })
    });
    tap_all_mana(&mut engine, p0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, yavimaya_granger()).is_some());
}

#[test]
fn yavimaya_granger_may_find_no_basic_land() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut engine = lk::game(SEED, &[], &[], &[yavimaya_granger()], &[]);
    lk::float(&mut engine, lk::P0, &[(ManaColor::Green, 3)]);
    cast_with_floating(&mut engine, lk::P0, yavimaya_granger());
    let Pending::ChooseCards { min, .. } = pass_to_card_choice(&mut engine) else {
        unreachable!()
    };
    assert_eq!(min, 0);
    engine
        .apply(lk::P0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, lk::P0, forest()).is_none());
    assert!(on_battlefield(&engine, lk::P0, yavimaya_granger()).is_some());
}
