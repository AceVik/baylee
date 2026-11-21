//! `cards/instants/mv_4/mystic_melting.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;
use baylee_engine::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "850fe8ad-f6b8-4d01-a94b-2b69adc3d8f0"
fn mystic_melting() -> CardIndex {
    card_index("850fe8ad-f6b8-4d01-a94b-2b69adc3d8f0")
}

/// Mystic Melting ({3}{G}): destroys target artifact or enchantment (excluding
/// non-artifact creatures), does not draw immediately, and draws a card at the
/// beginning of the next turn's upkeep.
#[test]
fn mystic_melting_destroys_artifact_or_enchantment_and_draws_next_upkeep() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[sol_ring(), index::GLORIOUS_ANTHEM, index::GRIZZLY_BEARS],
        &[mystic_melting()],
        &[],
    );
    let ring = lk::obj(&engine, P1, sol_ring());
    let anthem = lk::obj(&engine, P1, index::GLORIOUS_ANTHEM);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(&mut engine, P0, &[(ManaColor::Green, 4)]);
    cast_with_floating(&mut engine, P0, mystic_melting());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ring) && options.contains(&anthem));
    assert!(!options.contains(&bears), "creature is excluded");
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![ring],
                players: vec![],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert!(in_graveyard(&engine, P1, sol_ring()).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        0,
        "does not draw immediately"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.step == Step::Upkeep
    });
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        1,
        "drew 1 card at the beginning of next turn's upkeep"
    );
}
