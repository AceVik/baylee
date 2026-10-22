//! `cards/instants/mv_3/fevered_strength.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn fevered_strength() -> CardIndex {
    card_index("52512d7a-9b1c-4475-80aa-c95f4efa1015")
}

#[test]
fn fevered_strength_pumps_power_by_two_and_draws_on_next_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), grizzly_bears()])
        .hand(0, &[fevered_strength()])
        .start();

    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, fevered_strength());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, bears),
        (4, 2),
        "Fevered Strength should give target creature +2/+0 until end of turn"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1
    );

    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::Upkeep
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "Fevered Strength should draw a card at the beginning of the next turn's upkeep"
    );
}
