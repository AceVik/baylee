//! `cards/artifacts/equipment/mv_2/dowsing_dagger.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Dowsing Dagger` // `Lost Vale` (`Coverage::Partial`): "When this Equipment enters, target
/// opponent creates two 0/2 green Plant creature tokens with defender. Equipped creature gets
/// +2/+1. Whenever equipped creature deals combat damage to a player, you may transform this
/// Equipment. Equip {2} // {T}: Add three mana of any one color."
///
/// Under `Coverage::Partial`, the enters-trigger is omitted, but the static +2/+1 pump, the
/// `Trigger::DealsCombatDamageToPlayer` transform trigger, and equip {2} are implemented. The test
/// equips `Dowsing Dagger` to an elf, confirms the +2/+1 pump, attacks an opponent with the
/// equipped creature to deal combat damage, and verifies that `Dowsing Dagger` becomes
/// `Lost Vale` as a land on face 1.
///
/// It is the same permanent, turned over (CR 701.27a, CR 712.18), where the stand-in (#206)
/// exiled it and returned a new object. And the Equipment it was is gone with the face: Lost
/// Vale prints no static, so nothing of "equipped creature gets +2/+1" is left in the effect
/// table (CR 604.2), and a land attached to a creature becomes unattached (CR 704.5p), so the
/// elf is a 1/1 again.
#[test]
fn dowsing_dagger_pumps_equipped_creature_and_transforms_on_combat_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(235, forest())
        .battlefield(0, &[forest(), forest(), dowsing_dagger(), llanowar_elves()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("p0 controls the elf");
    let dagger = on_battlefield(&engine, p0, dowsing_dagger()).expect("dagger on battlefield");
    let dagger_was = identity(&engine, dagger);
    assert_eq!(pt(&engine, elf), (1, 1), "elf starts as a 1/1");

    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    // Ability index 2 corresponds to `equip!("{2}")` (after static and triggered abilities).
    activate(&mut engine, p0, dowsing_dagger(), 2);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected equip target prompt, got {:?}", engine.pending())
    };
    assert!(options.contains(&elf), "elf is a legal equip target");

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(dagger)
            .is_some_and(|o| o.attached_to == Some(elf))
    });

    assert_eq!(pt(&engine, elf), (3, 2), "equipped creature receives +2/+1");

    // Advance to combat and declare the equipped elf as an attacker against p1.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elf, Defender::Player(p1))],
            },
        )
        .unwrap();

    // Not `stack_is_empty`: the stack is already empty the moment attackers
    // are declared, so that predicate stops the walk *before* the combat
    // damage step and every life total still reads 20. The end step is past
    // damage (CR 510.2) and is what the assertion below needs.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(
        engine.state().players[1].life,
        17,
        "opponent took 3 combat damage from the equipped elf"
    );

    let vale = on_battlefield(&engine, p0, dowsing_dagger()).expect("Lost Vale on battlefield");
    assert_eq!(
        engine.state().object(vale).map(|o| o.face_index),
        Some(1),
        "dagger transformed to face 1 (Lost Vale)"
    );
    assert_eq!(
        identity(&engine, vale),
        dagger_was,
        "the same object, turned over: a transform changes no zone (CR 712.18)"
    );
    assert_eq!(
        engine.state().object(vale).and_then(|o| o.attached_to),
        None,
        "a land attached to a creature becomes unattached (CR 704.5p)"
    );
    assert!(
        engine
            .state()
            .effects
            .iter()
            .all(|fx| fx.source != Some(vale)),
        "the Equipment's static turned away with its face: {:?}",
        engine
            .state()
            .effects
            .iter()
            .filter(|fx| fx.source == Some(vale))
            .collect::<Vec<_>>()
    );
    assert_eq!(pt(&engine, elf), (1, 1), "the elf is a 1/1 again");
    let t = types(&engine, vale);
    assert!(
        t.contains(TypeSet::LAND),
        "Lost Vale is a land after transforming"
    );
    assert!(
        !t.contains(TypeSet::ARTIFACT),
        "Lost Vale is not an artifact"
    );
}
