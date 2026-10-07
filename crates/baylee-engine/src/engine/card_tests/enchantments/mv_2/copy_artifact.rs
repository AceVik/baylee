//! `cards/enchantments/mv_2/copy_artifact.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Copy Artifact: "You may have this enchantment enter as a copy of any
/// artifact on the battlefield, except it's an enchantment in addition to
/// its other types." Copying Sol Ring, it enters as both an artifact and an
/// enchantment, and its own ability array is Sol Ring's: index 0 is Sol
/// Ring's `{T}: Add {C}{C}` (there is no separate "except it has" clause
/// here, so nothing rides the `GRANTED_ABILITY` slot).
#[test]
fn copy_artifact_enters_as_an_artifact_and_enchantment_copy_of_sol_ring() {
    let p0 = PlayerId::new(0);
    let copy_artifact = copy_artifact();
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), sol_ring()])
        .hand(0, &[copy_artifact])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ring = on_battlefield(&engine, p0, sol_ring()).expect("Sol Ring seated");

    // Tap only the Islands (not Sol Ring, which must stay put as the copy
    // target and whose own mana would otherwise float unspent, muddying the
    // {C}{C} the activated copy produces later).
    let islands = all_on_battlefield(&engine, p0, island());
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: islands[0] })
        .unwrap();
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: islands[1] })
        .unwrap();
    cast_with_floating(&mut engine, p0, copy_artifact);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets prompt for CopyOnEnter, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&ring),
        "Sol Ring is an offered copy target"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let copy = on_battlefield(&engine, p0, copy_artifact).expect("Copy Artifact resolved");
    assert_ne!(copy, ring, "a new object, not the original Sol Ring");
    let chars = engine.state().object(copy).unwrap().characteristics();
    assert!(chars.types.contains(TypeSet::ARTIFACT), "it is an artifact");
    assert!(
        chars.types.contains(TypeSet::ENCHANTMENT),
        "and, as printed, also an enchantment"
    );

    activate(&mut engine, p0, copy_artifact, 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        2,
        "Sol Ring's copied {{T}}: Add {{C}}{{C}}"
    );
}
