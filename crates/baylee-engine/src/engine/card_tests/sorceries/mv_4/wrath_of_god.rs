//! `cards/sorceries/mv_4/wrath_of_god.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wrath of God: "Destroy all creatures. They can't be regenerated." A
/// shield on one of them does not save it; the lands beneath both stand.
#[test]
fn wrath_of_god_destroys_all_creatures_through_a_regeneration_shield() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), quiet_creature()],
        )
        .hand(0, &[wrath_of_god()])
        .battlefield(1, &[oboro_envoy()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("seated");
    engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .object_mut(elf)
        .expect("seated")
        .regeneration_shields = 1;

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, wrath_of_god());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, quiet_creature()).is_none(),
        "\"can't be regenerated\" — the shield did not save it"
    );
    assert!(in_graveyard(&engine, p0, quiet_creature()).is_some());
    assert!(on_battlefield(&engine, p1, oboro_envoy()).is_none());
    assert!(in_graveyard(&engine, p1, oboro_envoy()).is_some());
    assert_eq!(
        all_on_battlefield(&engine, p0, plains()).len(),
        4,
        "\"all creatures\" — not lands"
    );
}
