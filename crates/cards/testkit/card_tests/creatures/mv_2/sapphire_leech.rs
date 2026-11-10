//! Sapphire Leech — {1}{U} — Creature — Leech
//! Oracle: Flying
//! Oracle: Blue spells you cast cost {U} more to cast.

use super::*;
use baylee_core::generated::index;

#[test]
fn sapphire_leech_flies_and_increases_your_blue_spell_costs_by_one_blue_mana() {
    let leech = card_index("500aff17-0d69-46ac-a4f9-2fc6db061ce7");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), plains(), plains()])
        .battlefield(1, &[island()])
        .hand(0, &[leech, index::ANCESTRAL_RECALL, index::SAVANNAH_LIONS])
        .hand(1, &[index::ANCESTRAL_RECALL])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, leech);
    pass_until(&mut engine, stack_is_empty);

    let leech_id = on_battlefield(&engine, p0, leech).expect("Sapphire Leech on board");
    assert_eq!(pt(&engine, leech_id), (2, 2));
    assert!(keywords(&engine, leech_id).contains(KeywordSet::FLYING));

    // Remaining mana: 1 Island + 1 Plains ({U}{W}). Ancestral Recall ({U}) now costs {U}{U}, so it cannot be cast.
    tap_all_mana(&mut engine, p0);
    let recall_hand = in_hand(&engine, p0, index::ANCESTRAL_RECALL).unwrap();
    assert!(!priority_offer(&engine).castable.contains(&recall_hand));

    // Non-blue spells are unaffected: Savannah Lions ({W}) can be cast.
    let lions_hand = in_hand(&engine, p0, index::SAVANNAH_LIONS).unwrap();
    assert!(priority_offer(&engine).castable.contains(&lions_hand));

    // Opponent's blue spells are not taxed.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    let opp_recall_hand = in_hand(&engine, p1, index::ANCESTRAL_RECALL).unwrap();
    assert!(priority_offer(&engine).castable.contains(&opp_recall_hand));
}
