//! `cards/creatures/mv_3/fyndhorn_druid.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn fyndhorn_druid() -> CardIndex {
    card_index("98e51f1a-ed6e-4286-a41b-e4656d08c934")
}

#[test]
fn fyndhorn_druid_gains_four_life_when_it_dies() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), fyndhorn_druid()])
        .hand(0, &[lightning_bolt()])
        .start();

    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let druid = on_battlefield(&engine, p0, fyndhorn_druid()).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![druid],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        24,
        "Fyndhorn Druid should gain 4 life when it dies"
    );
}
