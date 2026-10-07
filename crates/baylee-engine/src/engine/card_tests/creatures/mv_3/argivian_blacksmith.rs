//! `cards/creatures/mv_3/argivian_blacksmith.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Argivian Blacksmith — "{T}: Prevent the next 2 damage that would be dealt
/// to target artifact creature this turn." Two words make the menu: an
/// artifact creature, so the Elf (a creature, no artifact) and the Pendant
/// (an artifact, no creature) are both absent; and the shield is the *next*
/// two points (CR 615.7), so Lightning Bolt's 3 leave the 0/2 Thopter marked
/// with exactly 1 and alive.
#[test]
fn argivian_blacksmith_shields_only_an_artifact_creature_for_two() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                argivian_blacksmith(),
                ornithopter(),
                llanowar_elves(),
                darksteel_pendant(),
                mountain(),
            ],
        )
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let smith = on_battlefield(&engine, p0, argivian_blacksmith()).expect("seated");
    let thopter = on_battlefield(&engine, p0, ornithopter()).expect("seated");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("seated");
    let pendant = on_battlefield(&engine, p0, darksteel_pendant()).expect("seated");

    activate(&mut engine, p0, argivian_blacksmith(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected the shield's target, got {:?}", engine.pending())
    };
    assert_eq!((player, min, max), (p0, 1, 1));
    assert_eq!(
        options,
        vec![thopter],
        "the only artifact creature is the whole menu"
    );
    assert!(!options.contains(&elf), "a creature that is no artifact");
    assert!(
        !options.contains(&pendant),
        "an artifact that is no creature"
    );
    assert!(
        !options.contains(&smith),
        "and the source itself is neither"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![thopter],
                players: vec![],
            },
        )
        .expect("the offered Thopter answers the offer");
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, smith), "{{T}} is the printed price");

    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, thopter);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(thopter).map(|o| o.damage),
        Some(1),
        "3 dealt, 2 prevented by the shield (CR 615.7)"
    );
    assert!(
        on_battlefield(&engine, p0, ornithopter()).is_some(),
        "1 damage does not kill a 0/2"
    );
}
