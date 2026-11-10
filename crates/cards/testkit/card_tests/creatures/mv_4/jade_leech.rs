//! Jade Leech — {2}{G}{G} — Creature — Leech
//! Oracle: Green spells you cast cost {G} more to cast.

use super::*;
use baylee_core::generated::index;

#[test]
fn jade_leech_increases_your_green_spell_costs_by_one_green_mana() {
    let leech = card_index("495e08a8-e654-4a0b-a25f-1e344ae2a0fa");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), plains(), plains(), plains()],
        )
        .battlefield(1, &[forest()])
        .hand(0, &[leech, index::LLANOWAR_ELVES, index::SAVANNAH_LIONS])
        .hand(1, &[index::GIANT_GROWTH])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, leech);
    pass_until(&mut engine, stack_is_empty);

    let leech_id = on_battlefield(&engine, p0, leech).expect("Jade Leech on board");
    assert_eq!(pt(&engine, leech_id), (5, 5));

    // Remaining mana: 1 Forest + 1 Plains ({G}{W}). Llanowar Elves ({G}) now costs {G}{G}, so it cannot be cast.
    tap_all_mana(&mut engine, p0);
    let elves_hand = in_hand(&engine, p0, index::LLANOWAR_ELVES).unwrap();
    assert!(!priority_offer(&engine).castable.contains(&elves_hand));

    // Non-green spells are unaffected: Savannah Lions ({W}) can be cast.
    let lions_hand = in_hand(&engine, p0, index::SAVANNAH_LIONS).unwrap();
    assert!(priority_offer(&engine).castable.contains(&lions_hand));

    // Opponent's green spells are not taxed.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    let growth_hand = in_hand(&engine, p1, index::GIANT_GROWTH).unwrap();
    assert!(priority_offer(&engine).castable.contains(&growth_hand));
}
