//! Marauding Knight — {2}{B}{B} — Creature — Phyrexian Zombie Knight
//! Oracle: Protection from white
//! Oracle: This creature gets +1/+1 for each Plains your opponents control.

use super::*;
use baylee_core::generated::index;

#[test]
fn marauding_knight_has_protection_from_white_and_scales_with_opponents_plains() {
    let knight = card_index("8745b061-d5c6-46c1-a3fc-4d71d717f19e");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), plains()])
        .battlefield(1, &[plains(), plains(), plains()])
        .hand(0, &[knight, index::SWORDS_TO_PLOWSHARES])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, knight);
    pass_until(&mut engine, stack_is_empty);

    let knight_id = on_battlefield(&engine, p0, knight).expect("Marauding Knight on battlefield");
    // 2/2 base + 3 opponent Plains (own Plains does not count) = 5/5.
    assert_eq!(pt(&engine, knight_id), (5, 5));

    // Swords to Plowshares ({W}) cannot target Marauding Knight because of protection from white.
    let offer = priority_offer(&engine);
    let stp_id = in_hand(&engine, p0, index::SWORDS_TO_PLOWSHARES).unwrap();
    assert!(!offer.castable.contains(&stp_id));
}
