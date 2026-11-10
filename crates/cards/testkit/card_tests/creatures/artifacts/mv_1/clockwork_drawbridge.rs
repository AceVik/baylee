//! `cards/creatures/artifacts/mv_1/clockwork_drawbridge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn clockwork_drawbridge() -> CardIndex {
    card_index("e49eb21c-2a0d-4fd0-9f69-1bff6e94f9e7")
}

/// "{2}{W}, {T}: Tap target creature." The white is required, the target is
/// an opposing creature here, and the Drawbridge taps as the cost.
#[test]
fn clockwork_drawbridge_taps_target_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[clockwork_drawbridge()], &[gray_ogre()]);
    let wall = on_battlefield(&engine, p0, clockwork_drawbridge()).expect("seated");
    let ogre = on_battlefield(&engine, PlayerId::new(1), gray_ogre()).expect("ogre");
    assert_eq!(pt(&engine, wall), (0, 3));
    assert!(!is_tapped(&engine, ogre));

    float_mana(&mut engine, ManaColor::Colorless, 3);
    assert!(!is_offered(&engine, wall, 0), "{{W}} is required");
    float_mana(&mut engine, ManaColor::White, 1);
    assert!(is_offered(&engine, wall, 0));

    activate(&mut engine, p0, clockwork_drawbridge(), 0);
    let options = aim_at(&mut engine, p0, ogre);
    assert!(options.contains(&ogre), "the Ogre is a creature");
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, ogre), "the Ogre was tapped");
    assert!(is_tapped(&engine, wall), "{{T}} was the cost");
}

/// "Defender": the Drawbridge is not among the creatures that may attack.
#[test]
fn clockwork_drawbridge_has_defender_and_cannot_attack() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[clockwork_drawbridge()], &[]);
    let wall = on_battlefield(&engine, p0, clockwork_drawbridge()).expect("seated");
    assert!(keywords(&engine, wall).contains(KeywordSet::DEFENDER));
    assert!(!attackers_offered(&mut engine).contains(&wall));
}
