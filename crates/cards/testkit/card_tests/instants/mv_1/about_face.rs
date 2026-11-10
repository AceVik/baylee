#[allow(clippy::wildcard_imports)]
use super::*;

fn about_face() -> CardIndex {
    card_index("8a810017-06ac-4824-bc2f-a7d303d58ad7")
}

#[test]
fn about_face_switches_power_and_toughness_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[about_face()])
        .battlefield(0, &[earth_elemental(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elem = on_battlefield(&engine, p0, earth_elemental()).unwrap();
    assert_eq!(pt(&engine, elem), (4, 5));

    cast_from_hand(&mut engine, p0, about_face());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elem],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, elem), (5, 4));
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert_eq!(pt(&engine, elem), (4, 5));
}
