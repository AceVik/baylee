//! `cards/creatures/mv_2/nantuko_tracer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn nantuko_tracer_etb_puts_target_card_from_a_graveyard_on_the_bottom_of_its_owners_library() {
    let tracer = card_index("4fd567cb-0b8e-41dd-a2b7-87a196de294a");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[tracer])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let p1_bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    bury(&mut engine, &[p1_bear]);
    let gy_bear = in_graveyard(&engine, p1, grizzly_bears()).expect("Grizzly Bears in graveyard");
    let lib_before = engine.state().zones.list(ZoneLocation::Library(p1)).len();

    cast_from_hand(&mut engine, p0, tracer);
    let options = pass_until_targets(&mut engine, p0);
    assert!(options.contains(&gy_bear));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![gy_bear],
                players: vec![],
            },
        )
        .expect("target Grizzly Bears in graveyard");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine
        .apply(p0, PlayerAction::YesNo(true))
        .expect("accept may put on bottom of library");
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_none());
    let p1_lib = engine.state().zones.list(ZoneLocation::Library(p1));
    assert_eq!(p1_lib.len(), lib_before + 1);
    assert_eq!(engine.state().object(gy_bear).unwrap().zone, Zone::Library);
}
