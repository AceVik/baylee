//! `cards/instants/mv_2/foreshadow.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn foreshadow() -> CardIndex {
    card_index("4caa97df-1f6c-4806-bf14-e589cc1badb5")
}

/// `Foreshadow` ({1}{U}, Instant, `Coverage::Partial`): chooses a card name,
/// causes target opponent to mill a card, and draws a card at the beginning of
/// the next turn's upkeep.
#[test]
fn foreshadow_names_card_mills_target_opponent_and_draws_next_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[foreshadow()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let p1_gy_before = engine.state().zones.list(ZoneLocation::Graveyard(p1)).len();
    let p0_gy_before = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();
    let p1_hand_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, foreshadow());
    match engine.pending().clone() {
        Pending::ChoosePlayer { .. } => {
            engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
        }
        Pending::ChooseTargets { .. } => {
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![p1],
                    },
                )
                .unwrap();
        }
        other => panic!("expected target opponent choice, got {other:?}"),
    }
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCardName { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseCardName {
                card: grizzly_bears(),
                face: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        p1_gy_before + 1,
        "the opponent mills exactly one card"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        p0_gy_before + 1,
        "the caster mills nothing: only the spell itself lands in the graveyard"
    );

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "the caster draws one card at the next upkeep (the milled card was not named)"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        p1_hand_before + 1,
        "the opponent draws only its own draw step's card"
    );
}
