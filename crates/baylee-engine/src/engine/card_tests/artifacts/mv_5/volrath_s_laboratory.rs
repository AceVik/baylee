//! `cards/artifacts/mv_5/volrath_s_laboratory.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn volrath_s_laboratory() -> CardIndex {
    card_index("f2a6e26c-6bd6-4fca-ba9f-d52d01fc03ef")
}

/// Volrath's Laboratory (`f2a6e26c-6bd6-4fca-ba9f-d52d01fc03ef`) — {5} Artifact (Partial):
/// "{5}, {T}: Create a 2/2 creature token of the chosen color and type."
#[test]
fn volrath_s_laboratory_activates_to_create_token() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                volrath_s_laboratory(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    assert!(walk_to_own_main(&mut engine, P0));

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, volrath_s_laboratory(), 0);
    pass_until(&mut engine, stack_is_empty);

    let tokens: Vec<_> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| engine.state().object(id).unwrap().card.is_none())
        .collect();
    assert_eq!(tokens.len(), 1);
    assert_eq!(pt(&engine, tokens[0]), (2, 2));
}
