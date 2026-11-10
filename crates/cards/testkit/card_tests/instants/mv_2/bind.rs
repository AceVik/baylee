//! Bind — {1}{G} — Instant
//! Oracle: Counter target activated ability. (Mana abilities can't be targeted.)
//! Oracle: Draw a card.

use super::*;
use baylee_core::generated::index;

#[test]
fn bind_counters_target_activated_ability_and_draws_a_card() {
    let bind = card_index("2b9557df-7158-4ada-be77-5c346851a568");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .battlefield(1, &[index::PRODIGAL_SORCERER])
        .hand(0, &[bind])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    activate(&mut engine, p1, index::PRODIGAL_SORCERER, 0);
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    let ability_id = engine.state().zones.list(ZoneLocation::Stack)[0];
    cast_from_hand(&mut engine, p0, bind);
    aim_at(&mut engine, p0, ability_id);
    let hand_after_cast = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_after_cast + 1
    );
}
