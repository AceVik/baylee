#[allow(clippy::wildcard_imports)]
use super::*;

fn urza_s_blueprints() -> CardIndex {
    card_index("ad75efdb-3e36-4253-be47-1df5a3cf8204")
}

#[test]
fn urza_s_blueprints_taps_to_draw_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[urza_s_blueprints()])
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), plains(), plains()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, urza_s_blueprints());
    pass_until(&mut engine, stack_is_empty);

    let blueprints = on_battlefield(&engine, p0, urza_s_blueprints()).unwrap();
    let before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: blueprints,
                ability_index: 1,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        before + 1
    );
}
