#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

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

#[test]
fn defense_grid_taxes_one_opponent_during_another_opponents_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let p2 = PlayerId::new(2);
    let mut engine = Duel::table(SEED, forest(), 3)
        .battlefield(0, &[defense_grid()])
        .hand(1, &[lightning_bolt()])
        .hand(2, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    lk::float(&mut engine, p1, &[(ManaColor::Red, 1)]);
    assert!(
        lk::castable(&engine, p1, lightning_bolt()),
        "active player pays no tax"
    );
    lk::pass_once(&mut engine);
    lk::float(&mut engine, p2, &[(ManaColor::Red, 1)]);
    assert!(
        !lk::castable(&engine, p2, lightning_bolt()),
        "nonactive opponent owes three more"
    );
    lk::float(&mut engine, p2, &[(ManaColor::Colorless, 3)]);
    assert!(lk::castable(&engine, p2, lightning_bolt()));
    lk::cast_at(&mut engine, p2, lightning_bolt(), &[], &[p0]);
    assert_eq!(lk::life(&engine, p0), 17);
    assert_eq!(lk::pool_total(&engine, p2), 0);
}
