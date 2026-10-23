#[allow(clippy::wildcard_imports)]
use super::*;

fn defense_grid() -> CardIndex {
    card_index("22ba474e-f6e6-450d-a4cf-0739f0b51079")
}

#[test]
fn defense_grid_taxes_spells_cast_during_opponents_turn_by_three() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[defense_grid()])
        .hand(1, &[lightning_bolt()])
        .battlefield(0, &[mountain(), mountain()])
        .battlefield(1, &[mountain(), mountain(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, defense_grid());
    pass_until(&mut engine, stack_is_empty);

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    let bolt = in_hand(&engine, p1, lightning_bolt()).unwrap();
    engine
        .apply(p1, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 17);
}
