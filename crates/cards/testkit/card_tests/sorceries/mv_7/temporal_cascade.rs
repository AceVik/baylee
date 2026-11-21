//! `cards/sorceries/mv_7/temporal_cascade.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn temporal_cascade() -> CardIndex {
    card_index("c65dc9cb-3aad-4d15-a8c0-c0c142058d45")
}

/// Temporal Cascade (c65dc9cb-3aad-4d15-a8c0-c0c142058d45) — {5}{U}{U} Sorcery:
/// Mode 0: Each player shuffles their hand and graveyard into their library;
/// Mode 1: Each player draws seven cards; Entwine {2}.
#[test]
fn temporal_cascade_runs_shuffle_mode_draw_mode_and_entwined_timetwister() {
    let mut m0 = lk::game(
        SEED,
        &[island(); 7],
        &[],
        &[temporal_cascade(), sol_ring()],
        &[aurochs()],
    );
    seed_graveyard(&mut m0, P0, 2);
    seed_graveyard(&mut m0, P1, 1);
    tap_all_mana(&mut m0, P0);
    cast_with_floating(&mut m0, P0, temporal_cascade());
    let slot0 = choose_cast_kind(&m0, CastModeKind::Mode(0));
    m0.apply(P0, PlayerAction::ChooseMode(slot0)).unwrap();
    lk::settle(&mut m0);
    assert!(m0.state().zones.list(ZoneLocation::Hand(P0)).is_empty());
    assert!(m0.state().zones.list(ZoneLocation::Hand(P1)).is_empty());
    assert!(
        m0.state()
            .zones
            .list(ZoneLocation::Graveyard(P1))
            .is_empty()
    );
    assert_eq!(
        m0.state().zones.list(ZoneLocation::Graveyard(P0)).len(),
        1,
        "only Temporal Cascade itself enters graveyard after resolution"
    );

    let mut entwined = lk::game(
        SEED,
        &[island(); 9],
        &[],
        &[temporal_cascade(), sol_ring()],
        &[aurochs(), fangren_hunter()],
    );
    seed_graveyard(&mut entwined, P0, 2);
    seed_graveyard(&mut entwined, P1, 2);
    tap_all_mana(&mut entwined, P0);
    cast_with_floating(&mut entwined, P0, temporal_cascade());
    let both = choose_cast_kind(&entwined, CastModeKind::Modes(3));
    entwined.apply(P0, PlayerAction::ChooseMode(both)).unwrap();
    assert_eq!(lk::pool_total(&entwined, P0), 0, "spent {{7}}{{U}}{{U}}");
    lk::settle(&mut entwined);

    assert_eq!(
        entwined.state().zones.list(ZoneLocation::Hand(P0)).len(),
        7,
        "shuffled hand+gy then drew 7"
    );
    assert_eq!(entwined.state().zones.list(ZoneLocation::Hand(P1)).len(), 7);
    assert!(
        entwined
            .state()
            .zones
            .list(ZoneLocation::Graveyard(P1))
            .is_empty()
    );
}
