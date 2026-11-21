//! `cards/instants/mv_2/perilous_research.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "9243b539-5f9e-4bb3-9973-3d48366aa7cc"
fn perilous_research() -> CardIndex {
    card_index("9243b539-5f9e-4bb3-9973-3d48366aa7cc")
}

/// Perilous Research ({1}{U}): draws two cards, then makes you sacrifice any
/// permanent you control (offered from all your permanents, never opponent's);
/// if you control no permanents, you still draw two cards cleanly.
#[test]
fn perilous_research_draws_two_and_sacrifices_any_controlled_permanent() {
    let mut engine = lk::game(
        SEED,
        &[island(), sol_ring(), index::GRIZZLY_BEARS],
        &[index::HILL_GIANT],
        &[perilous_research()],
        &[],
    );
    let land = lk::obj(&engine, P0, island());
    let ring = lk::obj(&engine, P0, sol_ring());
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 2)]);
    cast_with_floating(&mut engine, P0, perilous_research());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected sacrifice choice");
    };
    assert!(options.contains(&land) && options.contains(&ring) && options.contains(&bears));
    assert!(
        !options.contains(&giant),
        "cannot sacrifice opponent's permanent"
    );

    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        2,
        "drew 2 cards"
    );
    assert!(in_graveyard(&engine, P0, sol_ring()).is_some());
    assert!(on_battlefield(&engine, P0, island()).is_some());
    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_some());

    // Boundary: with 0 permanents controlled, still draws 2 cards cleanly.
    let mut empty = lk::game(SEED, &[], &[], &[perilous_research()], &[]);
    lk::float(&mut empty, P0, &[(ManaColor::Blue, 2)]);
    cast_with_floating(&mut empty, P0, perilous_research());
    lk::settle(&mut empty);
    assert_eq!(empty.state().zones.list(ZoneLocation::Hand(P0)).len(), 2);
}
