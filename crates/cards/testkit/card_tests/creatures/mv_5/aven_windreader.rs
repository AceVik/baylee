//! `cards/creatures/mv_5/aven_windreader.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn aven_windreader() -> CardIndex {
    card_index("8406b724-a824-4905-83e0-a6c02d7f72e3")
}

/// Aven Windreader (8406b724-a824-4905-83e0-a6c02d7f72e3) — `{3}{U}{U}` 3/3
/// Creature — Bird Soldier Wizard:
/// - Flying
/// - `{1}{U}`: Target player reveals the top card of their library.
#[test]
fn aven_windreader_flies_and_reveals_target_player_top_card_without_moving_it() {
    let lands = vec![island(); 7];
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &lands)
        .hand(0, &[aven_windreader()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let top_bolt = hand_to_library_top(&mut engine, P1, lightning_bolt());
    let p1_lib_before = library_size(&engine, P1);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, aven_windreader());
    pass_until(&mut engine, stack_is_empty);

    let windreader =
        on_battlefield(&engine, P0, aven_windreader()).expect("Aven Windreader entered");
    assert_eq!(pt(&engine, windreader), (3, 3));
    assert!(keywords(&engine, windreader).contains(KeywordSet::FLYING));

    activate(&mut engine, P0, aven_windreader(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
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

    assert_eq!(library_size(&engine, P1), p1_lib_before);
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(P1))
            .last()
            .copied(),
        Some(top_bolt),
        "revealed top card stays on top of target player's library"
    );
    assert!(
        engine.state().journal.entries().iter().any(|entry| {
            matches!(
                &entry.event,
                GameEvent::Revealed { player, cards } if *player == P1 && cards == &[top_bolt]
            )
        }),
        "journal records revealing P1's top card"
    );
}
