//! `cards/lands/utility/emergence_zone.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Emergence Zone: "{T}: Add {C}." / "{1}, {T}, Sacrifice this land: You may cast spells this turn as though they had flash."
/// Under `Coverage::Partial`, the timing grant applies to sorceries rather than all spell types.
/// Activating ability 1 sacrifices Emergence Zone, paying {1} from a Forest and sending the land to the graveyard.
#[test]
fn emergence_zone_sacrifices_to_grant_flash() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(127, forest())
        .battlefield(0, &[emergence_zone(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let zone = on_battlefield(&engine, p0, emergence_zone()).expect("Zone deployed");
    tap_mana_except(&mut engine, p0, zone);

    activate(&mut engine, p0, emergence_zone(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, emergence_zone()).is_none());
    assert!(in_graveyard(&engine, p0, emergence_zone()).is_some());
}
