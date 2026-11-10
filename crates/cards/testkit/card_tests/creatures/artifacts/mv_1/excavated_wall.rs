//! `cards/creatures/artifacts/mv_1/excavated_wall.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn excavated_wall() -> CardIndex {
    card_index("16e48b95-f030-400c-a2b7-c3f3111cc165")
}

/// "{1}, {T}: Mill a card." One card moves from the library to the
/// graveyard, the Wall taps, and the tap bars a second activation.
#[test]
fn excavated_wall_mills_one_card_for_a_tap() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[excavated_wall()], &[]);
    let me = on_battlefield(&engine, p0, excavated_wall()).expect("seated");
    assert_eq!(pt(&engine, me), (0, 4));
    let library = library_size(&engine, p0);
    let yard = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();

    float_mana(&mut engine, ManaColor::Colorless, 2);
    activate(&mut engine, p0, excavated_wall(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(library_size(&engine, p0), library - 1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        yard + 1
    );
    assert!(is_tapped(&engine, me));
    assert!(!is_offered(&engine, me, 0), "tapped: not twice");
}

/// "Defender".
#[test]
fn excavated_wall_cannot_attack() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[excavated_wall()], &[]);
    let me = on_battlefield(&engine, p0, excavated_wall()).expect("seated");
    assert!(keywords(&engine, me).contains(KeywordSet::DEFENDER));
    assert!(!attackers_offered(&mut engine).contains(&me));
}
