//! `cards/instants/mv_3/reverse_damage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Reverse Damage: "The next time a source of your choice would deal damage
/// to you this turn, prevent that damage. If damage is prevented this way,
/// you gain that much life instead." Untargeted — the source is chosen as
/// this resolves. Cast in response to p1's Bolt, naming the Bolt itself,
/// still on the stack underneath it, as that source.
#[test]
fn reverse_damage_prevents_damage_from_the_chosen_source_and_gains_that_life() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), forest()])
        .hand(0, &[reverse_damage()])
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, lightning_bolt());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .expect("the Bolt at p0");
    let bolt = on_stack(&engine, lightning_bolt()).expect("the Bolt is on the stack");

    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, reverse_damage());
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    let Pending::ChooseDamageSource {
        player,
        options,
        choice,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected the chosen-source question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0);
    let source = baylee_core::ids::DamageSourceRef {
        object: bolt,
        version: engine
            .state()
            .object(bolt)
            .expect("Bolt is on the stack")
            .version,
    };
    assert!(
        options.contains(&source),
        "the Bolt, still on the stack, is a legal source to name: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseDamageSource { choice, source })
        .expect("naming the Bolt");

    let before = life_of(&engine, p0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        life_of(&engine, p0),
        before + 3,
        "the Bolt's 3 damage prevented, and that much life gained instead"
    );
}
