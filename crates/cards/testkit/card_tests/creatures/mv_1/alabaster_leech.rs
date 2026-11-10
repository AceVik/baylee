//! Alabaster Leech — {W} — Creature — Leech
//! Oracle: White spells you cast cost {W} more to cast.

use super::*;
use baylee_core::generated::index;

#[test]
fn alabaster_leech_increases_your_white_spell_costs_by_one_white_mana() {
    let leech = card_index("8a8b032e-90b7-4782-b854-b575f316d641");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1, plains())
        .battlefield(0, &[plains(), plains(), forest(), forest()])
        .hand(0, &[leech, index::SAVANNAH_LIONS, grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, leech);
    pass_until(&mut engine, stack_is_empty);

    let leech_id = on_battlefield(&engine, p0, leech).expect("Alabaster Leech on board");
    assert_eq!(pt(&engine, leech_id), (1, 3));

    // Tap remaining lands (1 Plains, 2 Forest): Savannah Lions ({W}) now costs {W}{W}, so it cannot be cast.
    tap_all_mana(&mut engine, p0);
    let lions_hand = in_hand(&engine, p0, index::SAVANNAH_LIONS).unwrap();
    assert!(!priority_offer(&engine).castable.contains(&lions_hand));

    // Non-white spells are unaffected: Grizzly Bears ({1}{G}) can be cast.
    let bears_hand = in_hand(&engine, p0, grizzly_bears()).unwrap();
    assert!(priority_offer(&engine).castable.contains(&bears_hand));
}

#[test]
fn alabaster_leech_does_not_tax_opponent_white_spells() {
    let leech = card_index("8a8b032e-90b7-4782-b854-b575f316d641");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1, plains())
        .battlefield(0, &[leech])
        .battlefield(1, &[plains()])
        .hand(1, &[index::HEALING_SALVE])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    let salve_hand = in_hand(&engine, p1, index::HEALING_SALVE).unwrap();
    assert!(priority_offer(&engine).castable.contains(&salve_hand));
}
