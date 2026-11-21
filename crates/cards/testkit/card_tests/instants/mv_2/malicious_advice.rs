//! `cards/instants/mv_2/malicious_advice.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "14c1adf8-950b-4f64-9c84-956a623448f4"
fn malicious_advice() -> CardIndex {
    card_index("14c1adf8-950b-4f64-9c84-956a623448f4")
}

/// Malicious Advice ({X}{U}{B}): targets X artifacts, creatures, and/or lands
/// (excluding enchantments), taps all X targets, and makes you lose X life.
#[test]
fn malicious_advice_taps_x_artifacts_creatures_lands_and_loses_x_life() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[
            sol_ring(),
            index::GRIZZLY_BEARS,
            plains(),
            their_enchantment(),
        ],
        &[malicious_advice()],
        &[],
    );
    let ring = lk::obj(&engine, P1, sol_ring());
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    let land = lk::obj(&engine, P1, plains());
    let ench = lk::obj(&engine, P1, their_enchantment());

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Blue, 4), (ManaColor::Black, 1)],
    );
    cast_with_floating(&mut engine, P0, malicious_advice());
    engine.apply(P0, PlayerAction::ChooseNumber(3)).unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ring) && options.contains(&bears) && options.contains(&land));
    assert!(
        !options.contains(&ench),
        "enchantment is not an artifact, creature, or land: {options:?}"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![ring, bears, land],
                players: vec![],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert!(is_tapped(&engine, ring) && is_tapped(&engine, bears) && is_tapped(&engine, land));
    assert!(!is_tapped(&engine, ench));
    assert_eq!(lk::life(&engine, P0), 17, "lost X = 3 life");
}

/// For X = 0, Malicious Advice taps nothing and costs 0 life.
#[test]
fn malicious_advice_for_x_zero_loses_zero_life() {
    let mut engine = lk::game(SEED, &[], &[], &[malicious_advice()], &[]);
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Blue, 1), (ManaColor::Black, 1)],
    );
    cast_with_floating(&mut engine, P0, malicious_advice());
    if matches!(engine.pending(), Pending::ChooseNumber { .. }) {
        engine.apply(P0, PlayerAction::ChooseNumber(0)).unwrap();
    }
    lk::settle(&mut engine);

    assert_eq!(lk::life(&engine, P0), 20);
    assert!(in_graveyard(&engine, P0, malicious_advice()).is_some());
}
