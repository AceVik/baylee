//! `cards/creatures/mv_4/glimmering_angel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Glimmering Angel` is a 2/2 creature costing `{3}{W}` under `Coverage::Implemented`.
/// It prints the flying keyword and "{U}: This creature gains shroud until end of turn."
/// When activated off an Island for `{U}`, it gains shroud until end of turn without tapping.
#[test]
fn glimmering_angel_has_flying_and_gains_shroud() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[glimmering_angel(), island()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let angel = on_battlefield(&engine, p0, glimmering_angel())
        .expect("Glimmering Angel is on the battlefield");
    assert_eq!(pt(&engine, angel), (2, 2), "base body is 2/2");
    assert!(
        keywords(&engine, angel).contains(KeywordSet::FLYING),
        "Glimmering Angel has flying"
    );
    assert!(
        !keywords(&engine, angel).contains(KeywordSet::SHROUD),
        "does not have shroud initially"
    );

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, glimmering_angel(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, angel).contains(KeywordSet::SHROUD),
        "Glimmering Angel gains shroud until end of turn"
    );
    assert!(
        !is_tapped(&engine, angel),
        "activation cost did not tap the creature"
    );
}
