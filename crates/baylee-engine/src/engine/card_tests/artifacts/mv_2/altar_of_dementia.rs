//! `cards/artifacts/mv_2/altar_of_dementia.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn altar_of_dementia() -> CardIndex {
    card_index("d64e9152-ef24-4394-aeb0-9c3befc56549")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// p0 has the Altar, a Hill Giant (power 3) and Grizzly Bears (power 2); p1
/// has a Hill Giant of their own. p0's first main phase.
fn an_altar_over_two_creatures() -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[altar_of_dementia(), hill_giant(), grizzly_bears()])
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// Activates the Altar at `victim`, answers the sacrifice with `sacrificed`
/// and checks the offer held p0's creatures only; the ability is left on the
/// stack.
#[track_caller]
fn sacrifice_at(engine: &mut Engine<RegistryLookup>, victim: PlayerId, sacrificed: CardIndex) {
    let mine = on_battlefield(engine, P0, sacrificed).expect("the creature is p0's");
    let their_giant = on_battlefield(engine, P1, hill_giant()).expect("p1 has a Hill Giant");

    activate(engine, P0, altar_of_dementia(), 0);
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![victim],
            },
        )
        .unwrap();
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!(
            "expected the sacrifice question, got {:?}",
            engine.pending()
        )
    };
    assert!(options.contains(&mine));
    assert!(
        !options.contains(&their_giant),
        "only a creature you control can be sacrificed: {options:?}"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
}

/// Altar of Dementia (d64e9152-ef24-4394-aeb0-9c3befc56549) — {2} Artifact:
/// "Sacrifice a creature: Target player mills cards equal to the sacrificed creature's power."
#[test]
fn altar_of_dementia_mills_the_target_player_by_the_sacrificed_creatures_power() {
    let mut engine = an_altar_over_two_creatures();
    let lib_p1 = library_size(&engine, P1);
    let lib_p0 = library_size(&engine, P0);

    sacrifice_at(&mut engine, P1, grizzly_bears());
    assert!(
        in_graveyard(&engine, P0, grizzly_bears()).is_some(),
        "the sacrifice is a cost, paid before the ability resolves"
    );
    assert_eq!(library_size(&engine, P1), lib_p1, "nothing milled yet");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, P1),
        lib_p1 - 2,
        "Grizzly Bears has power 2, not the 3 of the other creature"
    );
    assert_eq!(library_size(&engine, P0), lib_p0, "p0 is not the target");
    assert!(
        on_battlefield(&engine, P0, hill_giant()).is_some(),
        "only the chosen creature was sacrificed"
    );
}

#[test]
fn altar_of_dementia_can_mill_its_own_controller_by_the_sacrificed_power() {
    let mut engine = an_altar_over_two_creatures();
    let lib_p1 = library_size(&engine, P1);
    let lib_p0 = library_size(&engine, P0);

    sacrifice_at(&mut engine, P0, hill_giant());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(library_size(&engine, P0), lib_p0 - 3);
    assert_eq!(library_size(&engine, P1), lib_p1, "p1 is not the target");
    assert!(in_graveyard(&engine, P0, hill_giant()).is_some());
}
