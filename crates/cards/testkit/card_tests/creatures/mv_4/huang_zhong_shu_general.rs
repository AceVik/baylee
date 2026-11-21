//! `cards/creatures/mv_4/huang_zhong_shu_general.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn huang_zhong_shu_general() -> CardIndex {
    card_index("7d6c4290-d46d-4b98-805c-3f537462c4c8")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Huang Zhong, Shu General (7d6c4290-d46d-4b98-805c-3f537462c4c8) — `{2}{W}{W}` 2/3
/// Legendary Creature — Human Soldier:
/// "This creature can't be blocked by more than one creature."
#[test]
fn huang_zhong_shu_general_cannot_be_blocked_by_more_than_one_creature() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), grizzly_bears()],
        )
        .battlefield(1, &[grizzly_bears(), grizzly_bears()])
        .hand(0, &[huang_zhong_shu_general()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, huang_zhong_shu_general());
    pass_until(&mut engine, stack_is_empty);

    let hz = on_battlefield(&engine, P0, huang_zhong_shu_general()).expect("Huang Zhong entered");
    let ally = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    assert_eq!(pt(&engine, hz), (2, 3));

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);

    let p1_bears: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            let o = engine.state().object(id).unwrap();
            o.controller == P1 && o.card.is_some_and(|c| c.index == grizzly_bears())
        })
        .collect();
    assert_eq!(p1_bears.len(), 2);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(hz, Defender::Player(P1)), (ally, Defender::Player(P1))],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        engine
            .apply(
                P1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(p1_bears[0], hz), (p1_bears[1], hz)],
                },
            )
            .is_err(),
        "Huang Zhong cannot be blocked by two creatures"
    );

    // A single blocker on Huang Zhong is legal, while the ally Bears has no cap.
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(p1_bears[0], hz), (p1_bears[1], ally)],
            },
        )
        .expect("one blocker on Huang Zhong and one on Grizzly Bears is legal");

    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(
        on_battlefield(&engine, P0, huang_zhong_shu_general()).is_some(),
        "2/3 Huang Zhong survives blocking by one 2/2 Grizzly Bears"
    );
}
