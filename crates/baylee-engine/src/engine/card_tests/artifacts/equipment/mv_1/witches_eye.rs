//! `cards/artifacts/equipment/mv_1/witches_eye.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn witches_eye() -> CardIndex {
    card_index("c8d479b8-b96c-4382-97d3-68d85cf371ba")
}

/// Witches' Eye — "Equipped creature has '{1}, {T}: Scry 1.'" and "Equip
/// {1}". Only the equipped creature is offered the ability; activating it
/// taps the creature, costs {1} and asks the scry.
#[test]
fn witches_eye_grants_a_scry_ability() {
    let creature = tragic_poet();
    let mut engine = rig(3, &[], &[witches_eye(), creature, creature], &[]);
    let p0 = PlayerId::new(0);
    let poets = all_on_battlefield(&engine, p0, creature);
    let (host, other) = (poets[0], poets[1]);
    let granted = crate::choice::granted_ability(0);
    let offers = |e: &Engine<RegistryLookup>, who: ObjectId| {
        priority_offer(e).abilities.contains(&(who, granted))
    };

    equip(&mut engine, witches_eye(), 1, host);
    float(&mut engine, 1);
    assert!(offers(&engine, host), "the wearer has the ability");
    assert!(!offers(&engine, other), "the other Human does not");

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: host,
                ability_index: granted,
            },
        )
        .expect("{1}, {T}");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange { cards, prompt, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(prompt, ArrangePrompt::Scry);
    assert_eq!(cards.len(), 1, "scry 1");
    engine
        .apply(p0, look_answer(&cards, &[]))
        .expect("keep it on top");
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, host), "{{T}} is part of the cost");
    assert_eq!(pool_total(&engine), 0, "so is {{1}}");
}
