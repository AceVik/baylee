//! `cards/creatures/artifacts/mv_3/assembly_worker.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn assembly_worker() -> CardIndex {
    card_index("751321ac-af07-4b7e-976f-5bf4e4281470")
}

/// Assembly-Worker — "{T}: Target Assembly-Worker creature gets +1/+1 until
/// end of turn." The menu is the Assembly-Workers and nothing else; aimed at
/// the other one, that one grows and the source is tapped.
#[test]
fn assembly_worker_pumps_another_assembly_worker() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(
        &[],
        &[assembly_worker(), assembly_worker(), quiet_creature()],
    );
    let workers = all_on_battlefield(&engine, p0, assembly_worker());
    assert_eq!(workers.len(), 2);
    let plain = on_battlefield(&engine, p0, quiet_creature()).expect("seated");
    let (source, ability_index) = priority_offer(&engine)
        .abilities
        .iter()
        .copied()
        .find(|(s, _)| workers.contains(s))
        .expect("the ability is offered");
    let other = *workers.iter().find(|w| **w != source).unwrap();
    assert_eq!(pt(&engine, other), (2, 2));

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .unwrap();
    let options = aim_at(&mut engine, p0, other);
    assert!(options.contains(&other));
    assert!(!options.contains(&plain), "only Assembly-Workers");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, other), (3, 3));
    assert_eq!(
        pt(&engine, source),
        (2, 2),
        "the source did not pump itself"
    );
    assert!(is_tapped(&engine, source), "{{T}} is the cost");
}
