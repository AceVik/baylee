//! `cards/creatures/mv_2/nightscape_familiar.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Nightscape Familiar — {1}{B} Creature — Zombie 1/1
/// "Blue spells and red spells you cast cost {1} less to cast.
/// {1}{B}: Regenerate this creature."
#[test]
fn nightscape_familiar_discounts_blue_and_red_spells() {
    let familiar = card_index("57296ea3-3c0d-49b7-bc08-0d0d8414e9ad");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                familiar,
                island(),
                island(),
                island(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[index::PHANTASMAL_FORCES, index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Exactly 6 mana ({U}{U}{U}{R}{R}{R}) casts both Phantasmal Forces ({3}{U} -> {2}{U})
    // and Hill Giant ({3}{R} -> {2}{R}).
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, index::PHANTASMAL_FORCES);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, index::PHANTASMAL_FORCES).is_some());

    cast_with_floating(&mut engine, p0, index::HILL_GIANT);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, index::HILL_GIANT).is_some());
}

#[test]
fn nightscape_familiar_regenerates_for_one_black() {
    let familiar = card_index("57296ea3-3c0d-49b7-bc08-0d0d8414e9ad");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[familiar, swamp(), swamp(), mountain()])
        .hand(0, &[index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let fam_id = on_battlefield(&engine, p0, familiar).expect("Nightscape Familiar on board");
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: fam_id,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    cast_with_floating(&mut engine, p0, index::LIGHTNING_BOLT);
    aim_at(&mut engine, p0, fam_id);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, familiar).is_some());
    assert!(is_tapped(&engine, fam_id));
}
