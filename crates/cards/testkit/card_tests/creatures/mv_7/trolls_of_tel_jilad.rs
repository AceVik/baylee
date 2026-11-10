//! `cards/creatures/mv_7/trolls_of_tel_jilad.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn trolls_of_tel_jilad() -> CardIndex {
    card_index("4478e843-fd25-4ad1-93f8-2dee605386fc")
}

/// Trolls of Tel-Jilad — a 5/6 for {5}{G}{G}.
#[test]
fn trolls_of_tel_jilad_costs_seven_and_is_a_five_six() {
    let mut engine = table7(&lots(forest(), 7), &[trolls_of_tel_jilad()], &[]);
    let trolls = cast_and_resolve(&mut engine, trolls_of_tel_jilad());
    assert_body(&engine, trolls, (5, 6), KeywordSet::default());
}

/// "{1}{G}: Regenerate target green creature." The menu is the green
/// creatures (the Trolls, not a red Ogre); with the shield up, destruction
/// regenerates the Trolls instead.
#[test]
fn trolls_of_tel_jilad_regenerate_a_green_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = table7(
        &[trolls_of_tel_jilad(), forest(), forest(), gray_ogre()],
        &[],
        &[],
    );
    let trolls = on_battlefield(&engine, p0, trolls_of_tel_jilad()).expect("out");
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("out");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, trolls_of_tel_jilad(), 0);
    let (options, _) = target_menu(&engine);
    assert!(options.contains(&trolls), "green");
    assert!(!options.contains(&ogre), "red");
    point_at(&mut engine, p0, &[trolls], &[]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(trolls)
            .expect("out")
            .regeneration_shields,
        1
    );
    kill(&mut engine, trolls);
    assert_eq!(
        on_battlefield(&engine, p0, trolls_of_tel_jilad()),
        Some(trolls)
    );
    assert!(is_tapped(&engine, trolls), "regeneration taps it");
}
