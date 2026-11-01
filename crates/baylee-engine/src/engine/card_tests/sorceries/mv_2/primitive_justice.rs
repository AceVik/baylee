//! `cards/sorceries/mv_2/primitive_justice.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn primitive_justice() -> CardIndex {
    card_index("cf38ac6e-e370-4ca5-90cd-26a1b812c63a")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Primitive Justice ({1}{R} Sorcery, Partial):
/// "Destroy target artifact."
#[test]
fn primitive_justice_destroys_target_artifact() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .battlefield(1, &[sol_ring(), grizzly_bears()])
        .hand(0, &[primitive_justice()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = on_battlefield(&engine, p1, sol_ring()).unwrap();
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, primitive_justice());
    // "Target artifact": a creature is no target.
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![bears],
                    players: vec![],
                },
            )
            .is_err()
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![ring],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, sol_ring()).is_some());
    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_some());
}
