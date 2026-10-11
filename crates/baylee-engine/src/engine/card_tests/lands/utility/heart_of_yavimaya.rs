//! `cards/lands/utility/heart_of_yavimaya.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Pay the Forest entry replacement, then pump a creature until end of turn.
#[test]
fn heart_of_yavimaya_sacrifices_a_forest_then_pumps_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[llanowar_elves(), forest()])
        .hand(0, &[heart_of_yavimaya()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf on battlefield");
    assert_eq!(pt(&engine, elf), (1, 1));

    let land = play_land(&mut engine, p0, heart_of_yavimaya());
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    let payment = on_battlefield(&engine, p0, forest()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![payment],
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().object(payment).unwrap().zone,
        Zone::Graveyard
    );
    assert!(!is_tapped(&engine, land), "enters untapped");
    assert!(
        on_battlefield(&engine, p0, heart_of_yavimaya()).is_some(),
        "Heart arrived after the Forest sacrifice"
    );

    activate(&mut engine, p0, heart_of_yavimaya(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&elf));

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, elf), (2, 2), "elf received +1/+1 pump");
    assert!(is_tapped(&engine, land));
}
