//! `cards/enchantments/auras/mv_2/regeneration.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Regeneration: "Enchant creature" / "{G}: Regenerate enchanted
/// creature." A shield bought off the Aura's own ability stands over the
/// creature it enchants and saves it from a destroy effect.
#[test]
fn regeneration_shields_the_enchanted_creature_from_being_destroyed() {
    let p0 = PlayerId::new(0);
    let regeneration = regeneration();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), quiet_creature()])
        .hand(0, &[regeneration])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let creature = on_battlefield(&engine, p0, quiet_creature()).expect("creature seated");

    let forests = all_on_battlefield(&engine, p0, forest());
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: forests[0] })
        .unwrap();
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: forests[1] })
        .unwrap();
    cast_with_floating(&mut engine, p0, regeneration);
    aim_at(&mut engine, p0, creature);
    pass_until(&mut engine, stack_is_empty);
    let aura = on_battlefield(&engine, p0, regeneration).expect("Regeneration is attached");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(creature)
    );

    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: forests[2] })
        .unwrap();
    activate(&mut engine, p0, regeneration, 1);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        engine
            .state()
            .object(creature)
            .expect("the creature is still there")
            .regeneration_shields,
        1,
        "one shield, standing over the creature it enchants"
    );

    kill(&mut engine, creature);
    assert_eq!(
        engine.state().object(creature).map(|o| o.zone),
        Some(Zone::Battlefield),
        "regenerated instead of dying"
    );
    assert!(
        is_tapped(&engine, creature),
        "regeneration taps the creature"
    );
    assert_eq!(
        engine
            .state()
            .object(creature)
            .unwrap()
            .regeneration_shields,
        0,
        "the shield was used"
    );
}
