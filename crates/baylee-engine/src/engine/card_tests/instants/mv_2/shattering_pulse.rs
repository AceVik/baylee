use super::*;

fn shattering_pulse() -> CardIndex {
    card_index("77953b7a-813e-45f5-ad7f-ab72a79a0112")
}

#[test]
fn shattering_pulse_destroys_target_artifact() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .battlefield(1, &[sol_ring(), gray_ogre()])
        .hand(0, &[shattering_pulse()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = on_battlefield(&engine, p1, sol_ring()).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, shattering_pulse());
    let creature = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("targets")
    };
    assert!(!options.contains(&creature));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![ring],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, sol_ring()).is_none());
    assert!(in_graveyard(&engine, p1, sol_ring()).is_some());
    assert!(in_graveyard(&engine, p0, shattering_pulse()).is_some());
}
