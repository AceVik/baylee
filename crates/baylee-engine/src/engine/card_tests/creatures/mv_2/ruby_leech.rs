//! Ruby Leech — {1}{R} — Creature — Leech
//! Oracle: First strike
//! Oracle: Red spells you cast cost {R} more to cast.

use super::*;
use baylee_core::generated::index;

#[test]
fn ruby_leech_has_first_strike_and_increases_your_red_spell_costs_by_one_red_mana() {
    let leech = card_index("ec6195a0-2701-40a4-882f-eb563668791a");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), plains(), plains()])
        .battlefield(1, &[mountain()])
        .hand(0, &[leech, index::LIGHTNING_BOLT, index::SAVANNAH_LIONS])
        .hand(1, &[index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, leech);
    pass_until(&mut engine, stack_is_empty);

    let leech_id = on_battlefield(&engine, p0, leech).expect("Ruby Leech on board");
    assert_eq!(pt(&engine, leech_id), (2, 2));
    assert!(keywords(&engine, leech_id).contains(KeywordSet::FIRST_STRIKE));

    // Remaining mana: 1 Mountain + 1 Plains ({R}{W}). Lightning Bolt ({R}) now costs {R}{R}, so it cannot be cast.
    tap_all_mana(&mut engine, p0);
    let bolt_hand = in_hand(&engine, p0, index::LIGHTNING_BOLT).unwrap();
    assert!(!priority_offer(&engine).castable.contains(&bolt_hand));

    // Non-red spells are unaffected: Savannah Lions ({W}) can be cast.
    let lions_hand = in_hand(&engine, p0, index::SAVANNAH_LIONS).unwrap();
    assert!(priority_offer(&engine).castable.contains(&lions_hand));

    // Opponent's red spells are not taxed.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    let opp_bolt_hand = in_hand(&engine, p1, index::LIGHTNING_BOLT).unwrap();
    assert!(priority_offer(&engine).castable.contains(&opp_bolt_hand));
}
