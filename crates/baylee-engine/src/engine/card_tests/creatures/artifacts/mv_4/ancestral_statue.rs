//! `cards/creatures/artifacts/mv_4/ancestral_statue.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn ancestral_statue() -> CardIndex {
    card_index("da69fbf7-c894-41dd-85ec-217de23a362f")
}

/// Ancestral Statue — {4} 3/4: "When this creature enters, return a
/// nonland permanent you control to its owner's hand." Cast beside a Sol
/// Ring, the question offers the Ring and the Statue itself and no Forest;
/// choosing the Ring puts it in hand and leaves the Statue out.
#[test]
fn ancestral_statue_returns_a_nonland_permanent() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[sol_ring(), forest(), forest(), forest(), forest()])
        .hand(0, &[ancestral_statue()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ring = on_battlefield(&engine, p0, sol_ring()).expect("seated");

    cast_from_hand(&mut engine, p0, ancestral_statue());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for this");
    };
    let statue = on_battlefield(&engine, p0, ancestral_statue()).expect("it resolved");
    assert!(options.contains(&ring), "the Ring is a nonland permanent");
    assert!(options.contains(&statue), "so is the Statue itself");
    assert_eq!(options.len(), 2, "no Forest is offered: {options:?}");
    assert_eq!((min, max), (1, 1), "exactly one: it is not a \"may\"");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .expect("the Ring is on the menu");
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, p0, sol_ring()).is_some(), "back in hand");
    assert!(on_battlefield(&engine, p0, sol_ring()).is_none());
    assert!(
        on_battlefield(&engine, p0, ancestral_statue()).is_some(),
        "the Statue stayed"
    );
}
