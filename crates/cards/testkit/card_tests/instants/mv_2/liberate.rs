//! `cards/instants/mv_2/liberate.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "f17e6c85-4813-4b0f-a0e9-d295822e9bb5"
fn liberate() -> CardIndex {
    card_index("f17e6c85-4813-4b0f-a0e9-d295822e9bb5")
}

/// Liberate ({1}{W}): targets only a creature you control, exiles it
/// immediately, and returns it to the battlefield at the next end step.
#[test]
fn liberate_exiles_controlled_creature_and_returns_it_at_end_step() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[index::HILL_GIANT],
        &[liberate()],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);

    lk::float(&mut engine, P0, &[(ManaColor::White, 2)]);
    cast_with_floating(&mut engine, P0, liberate());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&bears));
    assert!(
        !options.contains(&giant),
        "cannot target an opponent's creature: {options:?}"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert!(
        on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_none(),
        "exiled immediately during main phase"
    );
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Exile(P0))
            .is_empty(),
        "card sits in exile until the end step"
    );

    reach_their_main_phase(&mut engine, P1);
    assert!(
        on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_some(),
        "returned to battlefield at the beginning of the next end step"
    );
}

/// Liberate returns a stolen creature under its OWNER's control, not the
/// caster's.
#[test]
fn liberate_returns_stolen_creature_under_its_owners_control() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::GRIZZLY_BEARS],
        &[control_magic(), liberate()],
        &[],
    );
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 4)]);
    lk::cast_at(&mut engine, P0, control_magic(), &[bears], &[]);
    assert_eq!(
        on_battlefield(&engine, P0, index::GRIZZLY_BEARS),
        Some(bears),
        "Control Magic gives P0 control of P1's Bears"
    );

    lk::float(&mut engine, P0, &[(ManaColor::White, 2)]);
    lk::cast_at(&mut engine, P0, liberate(), &[bears], &[]);
    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_none());
    assert!(on_battlefield(&engine, P1, index::GRIZZLY_BEARS).is_none());

    reach_their_main_phase(&mut engine, P1);
    assert!(
        on_battlefield(&engine, P1, index::GRIZZLY_BEARS).is_some(),
        "returns under its owner's (P1's) control"
    );
    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_none());
}
