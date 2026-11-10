//! `cards/lands/utility/soulstone_sanctuary.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Soulstone Sanctuary: "{T}: Add {C}." / "{4}: This land becomes a 3/3 creature with vigilance and all creature types. It's still a land."
/// Paid with four Forests, activating ability 1 animates the land indefinitely without tapping it.
/// Upon resolution, the permanent is a 3/3 creature with vigilance and remains an untapped land.
#[test]
fn soulstone_sanctuary_animates_into_creature_with_vigilance() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(140, forest())
        .battlefield(
            0,
            &[
                soulstone_sanctuary(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sanctuary = on_battlefield(&engine, p0, soulstone_sanctuary()).expect("Sanctuary deployed");
    assert!(!types(&engine, sanctuary).contains(TypeSet::CREATURE));

    tap_mana_except(&mut engine, p0, sanctuary);
    activate(&mut engine, p0, soulstone_sanctuary(), 1);

    pass_until(&mut engine, stack_is_empty);

    let t = types(&engine, sanctuary);
    assert!(t.contains(TypeSet::LAND));
    assert!(t.contains(TypeSet::CREATURE));
    assert_eq!(pt(&engine, sanctuary), (3, 3));
    assert!(keywords(&engine, sanctuary).contains(KeywordSet::VIGILANCE));
    assert!(!is_tapped(&engine, sanctuary));
}
