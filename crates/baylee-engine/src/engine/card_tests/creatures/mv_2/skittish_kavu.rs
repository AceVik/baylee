//! Skittish Kavu — {1}{R} — Creature — Kavu
//! Oracle: This creature gets +1/+1 as long as no opponent controls a white or blue creature.

use super::*;
use baylee_core::generated::index;

#[test]
fn skittish_kavu_gets_plus_one_plus_one_only_while_no_opponent_controls_white_or_blue_creature() {
    let kavu = card_index("55722e42-a729-4289-895f-be913398c434");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), index::SAVANNAH_LIONS])
        .battlefield(1, &[plains(), index::GRIZZLY_BEARS])
        .hand(0, &[kavu, index::LIGHTNING_BOLT])
        .hand(1, &[index::SAVANNAH_LIONS])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, kavu);
    pass_until(&mut engine, stack_is_empty);

    // P0 controls a white creature (Savannah Lions) and P1 controls a green creature (Grizzly Bears):
    // no opponent controls a white or blue creature, so Skittish Kavu is 2/2.
    let kavu_id = on_battlefield(&engine, p0, kavu).expect("Skittish Kavu on board");
    assert_eq!(pt(&engine, kavu_id), (2, 2));

    // On P1's turn, P1 casts Savannah Lions (white creature): Skittish Kavu drops to 1/1.
    assert!(walk_to_own_main(&mut engine, p1));
    cast_from_hand(&mut engine, p1, index::SAVANNAH_LIONS);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, kavu_id), (1, 1));

    // When P0 destroys P1's Savannah Lions, Skittish Kavu returns to 2/2.
    let opp_lions = on_battlefield(&engine, p1, index::SAVANNAH_LIONS).unwrap();
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    crate::engine::card_tests::legends_kit::float(&mut engine, p0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut engine, p0, index::LIGHTNING_BOLT);
    aim_at(&mut engine, p0, opp_lions);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, kavu_id), (2, 2));
}
