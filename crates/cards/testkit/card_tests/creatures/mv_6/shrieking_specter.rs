//! `cards/creatures/mv_6/shrieking_specter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn shrieking_specter() -> CardIndex {
    card_index("f6adfb5f-a21c-4e86-bd16-a9c1e355af34")
}

fn air_elemental() -> CardIndex {
    card_index("7744bae4-a8b7-44a5-9b4c-0048ad4cc448")
}

/// Shrieking Specter (f6adfb5f-a21c-4e86-bd16-a9c1e355af34) — `{5}{B}` 2/2
/// Creature — Specter:
/// - Flying
/// - Whenever this creature attacks, defending player discards a card.
#[test]
fn shrieking_specter_flies_and_forces_defending_player_to_discard_on_attack_declaration() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[air_elemental()])
        .hand(0, &[shrieking_specter()])
        .hand(1, &[forest(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, shrieking_specter());
    pass_until(&mut engine, stack_is_empty);

    let specter =
        on_battlefield(&engine, P0, shrieking_specter()).expect("Shrieking Specter entered");
    let blocker = on_battlefield(&engine, P1, air_elemental()).unwrap();
    assert_eq!(pt(&engine, specter), (2, 2));
    assert!(keywords(&engine, specter).contains(KeywordSet::FLYING));

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    let keep = in_hand(&engine, P1, forest()).unwrap();
    let pitch = in_hand(&engine, P1, island()).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(specter, Defender::Player(P1))],
            },
        )
        .unwrap();

    // `Attacks` triggers in `DeclareAttackers` before blockers are chosen.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    assert_eq!(engine.state().turn.step, Step::DeclareAttackers);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseCards, got {:?}", engine.pending());
    };
    assert_eq!(player, P1);
    assert_eq!((min, max), (1, 1));
    assert!(options.contains(&keep) && options.contains(&pitch));

    engine
        .apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![pitch],
            },
        )
        .unwrap();

    // Even when subsequently blocked and killed by Air Elemental, the discard already resolved.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(in_graveyard(&engine, P1, island()).is_some());
    assert!(in_hand(&engine, P1, forest()).is_some());

    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, specter)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(in_graveyard(&engine, P0, shrieking_specter()).is_some());
}
