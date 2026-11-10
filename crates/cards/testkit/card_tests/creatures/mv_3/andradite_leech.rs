//! Andradite Leech — {2}{B} — Creature — Leech
//! Oracle: Black spells you cast cost {B} more to cast.
//! Oracle: {B}: This creature gets +1/+1 until end of turn.

use super::*;
use baylee_core::generated::index;

#[test]
fn andradite_leech_taxes_your_black_spells_and_pumps_self_for_black() {
    let leech = card_index("09bfc3b1-cc40-4cb9-87e4-86b0018caa75");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[leech, index::DARK_RITUAL])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    // Tap 3 Swamps to cast Andradite Leech ({2}{B}), leaving 1 untapped Swamp.
    let swamps = engine.state().zones.list(ZoneLocation::Battlefield).clone();
    for &s in &swamps[..3] {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: s })
            .unwrap();
    }
    cast_with_floating(&mut engine, p0, leech);
    pass_until(&mut engine, stack_is_empty);

    let leech_id = on_battlefield(&engine, p0, leech).expect("Andradite Leech on board");
    assert_eq!(pt(&engine, leech_id), (2, 2));

    // Tap the 4th Swamp: Dark Ritual ({B}) now costs {B}{B}, so it cannot be cast with 1 {B}.
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: swamps[3] })
        .unwrap();
    let ritual_hand = in_hand(&engine, p0, index::DARK_RITUAL).unwrap();
    assert!(!priority_offer(&engine).castable.contains(&ritual_hand));

    // Spend the {B} to activate Andradite Leech's {B}: +1/+1 until end of turn ability.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: leech_id,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, leech_id), (3, 3));
}
