//! `cards/artifacts/mv_4/jester_s_cap.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn jester_s_cap() -> CardIndex {
    card_index("8783a94a-d0b2-4291-a550-dbcf8e16bf4b")
}

/// Jester's Cap (8783a94a-d0b2-4291-a550-dbcf8e16bf4b) — {4} Artifact:
/// - "{2}, {T}, Sacrifice this artifact: Search target player's library for three cards and exile them. Then that player shuffles."
///   (Partial: pays {2}, {T}, sacrifices Jester's Cap, and shuffles target player's library.)
#[test]
fn jesters_cap_pays_cost_sacrifices_itself_and_shuffles_target_players_library() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[jester_s_cap(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let lib_before = engine.state().zones.list(ZoneLocation::Library(P1)).clone();
    let own_lib_before = engine.state().zones.list(ZoneLocation::Library(P0)).clone();

    tap_all_mana_but(&mut engine, P0, None);
    activate(&mut engine, P0, jester_s_cap(), 0);
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets target prompt, got {:?}",
            engine.pending()
        );
    };
    assert!(
        player_options.contains(&P0) && player_options.contains(&P1),
        "TargetSpec::AnyPlayer offers both P0 and P1"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, P0, jester_s_cap()).is_some(),
        "Jester's Cap was sacrificed to pay the activation cost"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{2}} was spent"
    );
    assert_ne!(
        engine.state().zones.list(ZoneLocation::Library(P1)),
        &lib_before,
        "target player P1's library was shuffled"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(P1)).len(),
        lib_before.len(),
        "no card was searched out and exiled (Partial: the search-and-exile half is missing)"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(P0)),
        &own_lib_before,
        "only the target player's library is shuffled, not the activator's"
    );
}
