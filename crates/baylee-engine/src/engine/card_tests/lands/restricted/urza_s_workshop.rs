//! `cards/lands/restricted/urza_s_workshop.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Urza's Workshop: "{T}: Add {C}." / "Metalcraft — {T}: Add {C} for each Urza's land you control. Activate only if you control three or more artifacts."
/// Under `Coverage::Implemented`, controlling three artifacts enables the dynamic metalcraft mana ability.
/// With three Urza's lands in play, activating ability 1 adds three colorless mana to the pool.
#[test]
fn urza_s_workshop_adds_mana_for_each_urza_land_with_metalcraft() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(110, forest())
        .battlefield(
            0,
            &[
                urza_s_workshop(),
                urza_s_mine(),
                urza_s_tower(),
                quiet_artifact(),
                quiet_artifact(),
                quiet_artifact(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let workshop = on_battlefield(&engine, p0, urza_s_workshop()).expect("Workshop deployed");
    activate(&mut engine, p0, urza_s_workshop(), 1);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 3);
    assert!(is_tapped(&engine, workshop));
}
