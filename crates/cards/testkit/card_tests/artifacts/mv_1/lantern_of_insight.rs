//! `cards/artifacts/mv_1/lantern_of_insight.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn lantern_of_insight() -> CardIndex {
    card_index("e4d9c9c9-90e4-4397-9a88-9457fc6854fa")
}

/// Lantern of Insight (e4d9c9c9-90e4-4397-9a88-9457fc6854fa) — {1} Artifact:
/// "Players play with the top card of their libraries revealed."
/// "{T}, Sacrifice this artifact: Target player shuffles."
///
/// Reveals both players' library tops while on the battlefield, stops revealing
/// once sacrificed, and shuffles the targeted player's library.
#[test]
fn lantern_of_insight_reveals_all_tops_and_sacrifices_to_shuffle_target_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[lantern_of_insight()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Before Lantern enters, neither library top is revealed.
    assert!(!engine.state().library_top_revealed(p0));
    assert!(!engine.state().library_top_revealed(p1));

    // Cast Lantern of Insight -> both players' library tops become revealed.
    cast_from_hand(&mut engine, p0, lantern_of_insight());
    pass_until(&mut engine, stack_is_empty);
    let lantern = on_battlefield(&engine, p0, lantern_of_insight()).unwrap();
    assert!(
        engine.state().library_top_revealed(p0) && engine.state().library_top_revealed(p1),
        "both players play with the top card of their libraries revealed"
    );

    let shuffles_before = engine.state().journal.entries().len();

    // Activate {T}, Sacrifice targeting opponent p1 -> p1 shuffles, reveal ends.
    activate(&mut engine, p0, lantern_of_insight(), 1);
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets, got {:?}", engine.pending())
    };
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "either player is a legal target: {player_options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().object(lantern).unwrap().zone,
        Zone::Graveyard,
        "Lantern sacrificed as cost"
    );
    assert!(
        !engine.state().library_top_revealed(p0) && !engine.state().library_top_revealed(p1),
        "library tops stop being revealed once Lantern leaves"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        engine.state().journal.entries()[shuffles_before..]
            .iter()
            .any(|e| matches!(
                e.event,
                baylee_engine::event::GameEvent::Shuffled {
                    player,
                    zone: Zone::Library
                } if player == p1
            )),
        "target player p1 shuffled their library"
    );
}
