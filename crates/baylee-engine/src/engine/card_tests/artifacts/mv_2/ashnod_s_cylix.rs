//! `cards/artifacts/mv_2/ashnod_s_cylix.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn ashnod_s_cylix() -> CardIndex {
    card_index("fdf4e4c3-fe17-4098-9f76-e98f3f00bd3e")
}

/// `Ashnod's Cylix` ({2}, Artifact, `Coverage::Partial`) activates for
/// `{3}, {T}` to look at the top three cards of target player's library.
#[test]
fn ashnods_cylix_taps_and_looks_at_top_three_of_target_players_library() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(); 5])
        .hand(0, &[ashnod_s_cylix()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, ashnod_s_cylix());
    pass_until(&mut engine, stack_is_empty);
    let cylix = on_battlefield(&engine, p0, ashnod_s_cylix()).expect("Ashnod's Cylix enters");

    let lib_before = library_size(&engine, p1);
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: cylix,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(options.len(), 3);
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, cylix));
    assert_eq!(library_size(&engine, p1), lib_before);
}
