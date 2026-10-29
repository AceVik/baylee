//! `cards/instants/mv_1/singe.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

/// Singe — {R} Instant
/// "Singe deals 1 damage to target creature. That creature becomes black until end of turn."
#[test]
fn singe_deals_one_damage_to_target_creature_and_makes_it_black_until_end_of_turn() {
    let singe = card_index("e7fbdebf-2275-4bf6-b364-6a1849765f94");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), grizzly_bears()])
        .hand(0, &[singe])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears_id = on_battlefield(&engine, p0, grizzly_bears()).expect("Grizzly Bears on board");
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, singe);
    aim_at(&mut engine, p0, bears_id);
    pass_until(&mut engine, stack_is_empty);

    let obj = engine.state().object(bears_id).unwrap();
    assert_eq!(obj.damage, 1);
    assert_eq!(
        obj.characteristics().colors,
        ColorSet::from_slice(&[Color::Black])
    );
}
