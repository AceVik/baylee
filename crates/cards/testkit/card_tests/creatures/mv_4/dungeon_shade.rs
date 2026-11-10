//! `cards/creatures/mv_4/dungeon_shade.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Dungeon Shade` is a 1/1 creature costing `{3}{B}` under `Coverage::Implemented`.
/// It prints the flying keyword and "{B}: This creature gets +1/+1 until end of turn."
/// When activated off a Swamp for `{B}`, it grows from 1/1 to 2/2 until end of turn.
#[test]
fn dungeon_shade_has_flying_and_pumps_power_toughness() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dungeon_shade(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let shade =
        on_battlefield(&engine, p0, dungeon_shade()).expect("Dungeon Shade is on the battlefield");
    assert_eq!(pt(&engine, shade), (1, 1), "base body is 1/1");
    assert!(
        keywords(&engine, shade).contains(KeywordSet::FLYING),
        "Dungeon Shade has flying"
    );

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, dungeon_shade(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, shade),
        (2, 2),
        "Dungeon Shade gets +1/+1 until end of turn"
    );
    assert!(
        !is_tapped(&engine, shade),
        "activation cost did not require tapping"
    );
}
