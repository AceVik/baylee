//! `cards/artifacts/equipment/mv_1/viridian_longbow.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn viridian_longbow() -> CardIndex {
    card_index("8b9119b0-9948-497f-aa92-2879d07bed1c")
}

/// Equip {3} really moves the Equipment. Only its current wearer gains the
/// targeted tap-ping; paying {T} deals one to the chosen player, and moving
/// the bow moves the ability rather than leaving a copy on the old wearer.
#[test]
fn longbow_equips_and_moves_its_targeted_tap_ping() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let creature = tragic_poet();
    let mut engine = rig(6, &[], &[viridian_longbow(), creature, creature], &[]);
    let poets = all_on_battlefield(&engine, p0, creature);
    let (first, second) = (poets[0], poets[1]);
    let granted = baylee_engine::choice::granted_ability(0);
    let offered =
        |e: &Engine<RegistryLookup>, who| priority_offer(e).abilities.contains(&(who, granted));
    assert!(!offered(&engine, first), "the bare creature has no ping");

    let bow = equip(&mut engine, viridian_longbow(), 3, first);
    assert!(offered(&engine, first));
    assert!(!offered(&engine, second), "only the wearer gains the ping");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: first,
                ability_index: granted,
            },
        )
        .expect("the granted {T} ability is activated");
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!("the ping asks for any target: {:?}", engine.pending())
    };
    assert!(player_options.contains(&p1), "a player is any target");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, first), "the wearer paid {{T}}");
    assert_eq!(engine.state().players[1].life, 19, "the wearer dealt one");
    assert_eq!(
        engine.state().object(bow).and_then(|o| o.attached_to),
        Some(first),
        "pinging does not sacrifice the bow"
    );

    equip(&mut engine, viridian_longbow(), 3, second);
    assert!(!offered(&engine, first), "the old wearer loses the ability");
    assert!(offered(&engine, second), "the new wearer gains it");
}
