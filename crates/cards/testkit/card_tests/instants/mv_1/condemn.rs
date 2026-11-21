//! `cards/instants/mv_1/condemn.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "377d31b5-0a48-4d5b-91bd-b5a62d4f7899"
fn condemn() -> CardIndex {
    card_index("377d31b5-0a48-4d5b-91bd-b5a62d4f7899")
}

/// Condemn ({W}): targets ONLY an attacking creature (excluding non-attacking
/// creatures), puts it on the bottom of its owner's library, and its controller
/// gains life equal to its last-known toughness (including pumps).
#[test]
fn condemn_tucks_attacking_creature_and_grants_controller_lki_toughness_life() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::HILL_GIANT, index::GRIZZLY_BEARS],
        &[condemn()],
        &[index::GIANT_GROWTH],
    );
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    reach_their_main_phase(&mut engine, P1);

    // P1 attacks with Hill Giant (3/3) only, keeping Grizzly Bears back.
    let _ = lk::attack(&mut engine, &[giant]);
    lk::block(&mut engine, &[]);

    // P1 pumps Hill Giant with Giant Growth -> 6/6 before Condemn resolves.
    lk::float(&mut engine, P1, &[(ManaColor::Green, 1)]);
    cast_with_floating(&mut engine, P1, index::GIANT_GROWTH);
    lk::aim(&mut engine, &[giant], &[]);
    lk::settle(&mut engine);
    assert_eq!(pt(&engine, giant), (6, 6));

    // P1 passes priority to P0 in Declare Blockers; P0 casts Condemn.
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    lk::float(&mut engine, P0, &[(ManaColor::White, 1)]);
    cast_with_floating(&mut engine, P0, condemn());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&giant));
    assert!(
        !options.contains(&bears),
        "non-attacking Bears is not a legal target: {options:?}"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert!(on_battlefield(&engine, P1, index::HILL_GIANT).is_none());
    let lib1 = engine.state().zones.list(ZoneLocation::Library(P1));
    assert_eq!(
        lib1.first().copied(),
        Some(giant),
        "Hill Giant is on the bottom of its owner's library"
    );
    assert_eq!(
        lk::life(&engine, P1),
        26,
        "controller gained life equal to LKI toughness (6)"
    );
    lk::through_combat(&mut engine);
    assert_eq!(lk::life(&engine, P0), 20, "tucked attacker dealt 0 damage");
}
