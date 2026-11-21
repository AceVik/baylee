//! `cards/instants/mv_1/ghastly_demise.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "e02e7239-304b-4755-b913-bb65bc693e86"
fn ghastly_demise() -> CardIndex {
    card_index("e02e7239-304b-4755-b913-bb65bc693e86")
}

/// Ghastly Demise ({B}): excludes black creatures; does not destroy a creature
/// whose toughness exceeds your graveyard count (Ghastly Demise itself is on
/// the stack, not in your graveyard); destroys once graveyard count >= toughness.
#[test]
fn ghastly_demise_checks_nonblack_and_graveyard_size_boundary() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::GRIZZLY_BEARS, index::DRUDGE_SKELETONS],
        &[ghastly_demise(), ghastly_demise(), shock()],
        &[],
    );
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    let skel = lk::obj(&engine, P1, index::DRUDGE_SKELETONS);

    // Put 1 card in P0's graveyard by casting Shock at P1.
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Red, 1), (ManaColor::Black, 2)],
    );
    lk::cast_at(&mut engine, P0, shock(), &[], &[P1]);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(P0)).len(),
        1
    );

    // 1 card in GY vs toughness 2 -> legal target, but survives resolution.
    cast_with_floating(&mut engine, P0, ghastly_demise());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&bears));
    assert!(
        !options.contains(&skel),
        "black creature is not a legal target: {options:?}"
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

    assert_eq!(
        on_battlefield(&engine, P1, index::GRIZZLY_BEARS),
        Some(bears),
        "with 1 card in GY during resolution, toughness 2 > 1 so Bears survives"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(P0)).len(),
        2,
        "first Ghastly Demise is now in graveyard -> 2 cards in GY"
    );

    // Second Ghastly Demise with 2 cards in GY vs toughness 2 -> destroys Bears.
    lk::cast_at(&mut engine, P0, ghastly_demise(), &[bears], &[]);
    assert!(
        in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some(),
        "with 2 cards in GY during resolution, toughness 2 <= 2 destroys Bears"
    );
}
