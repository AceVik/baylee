//! Crusading Knight — {2}{W}{W} — Creature — Human Knight
//! Oracle: Protection from black
//! Oracle: This creature gets +1/+1 for each Swamp your opponents control.

use super::*;
use baylee_core::generated::index;

#[test]
fn crusading_knight_has_protection_from_black_and_scales_with_opponents_swamps() {
    let knight = card_index("59385809-3223-4bc2-a824-9bab257c917a");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains(), swamp()])
        .battlefield(1, &[swamp(), swamp(), swamp()])
        .hand(0, &[knight, index::DISFIGURE])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, knight);
    pass_until(&mut engine, stack_is_empty);

    let knight_id = on_battlefield(&engine, p0, knight).expect("Crusading Knight on battlefield");
    // 2/2 base + 3 opponent Swamps (own Swamp does not count) = 5/5.
    assert_eq!(pt(&engine, knight_id), (5, 5));

    // Disfigure ({B}) cannot target Crusading Knight because of protection from black.
    let offer = priority_offer(&engine);
    let disfigure_id = in_hand(&engine, p0, index::DISFIGURE).unwrap();
    assert!(!offer.castable.contains(&disfigure_id));
}
