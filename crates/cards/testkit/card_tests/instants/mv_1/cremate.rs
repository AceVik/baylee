//! Cremate — {B} — Instant
//! Oracle: Exile target card from a graveyard.
//! Oracle: Draw a card.

use super::*;

#[test]
fn cremate_exiles_target_card_from_a_graveyard_and_draws_a_card() {
    let cremate = card_index("c6a2e410-b182-48d1-aeb2-bc8de27e9cd2");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, grizzly_bears())
        .battlefield(0, &[swamp()])
        .hand(0, &[cremate])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));
    seed_graveyard(&mut engine, p1, 1);

    let bears_gy = in_graveyard(&engine, p1, grizzly_bears()).expect("Bears in graveyard");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p0, cremate);
    aim_at(&mut engine, p0, bears_gy);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_none());
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p1))
            .contains(&bears_gy)
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
}
