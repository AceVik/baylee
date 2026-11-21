//! `cards/creatures/mv_4/yellow_scarves_general.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn yellow_scarves_general() -> CardIndex {
    card_index("6fbade4d-502d-45d7-a79e-3fd9e3e7ccdf")
}

fn wu_elite_cavalry() -> CardIndex {
    card_index("7dc5d729-0553-46fb-aff7-2f0d9573837b")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Yellow Scarves General (6fbade4d-502d-45d7-a79e-3fd9e3e7ccdf) — `{3}{R}` 2/2
/// Creature — Human Soldier:
/// - Horsemanship
/// - This creature can't block.
#[test]
fn yellow_scarves_general_has_horsemanship_and_cannot_block() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .battlefield(1, &[grizzly_bears(), wu_elite_cavalry()])
        .hand(0, &[yellow_scarves_general()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, yellow_scarves_general());
    pass_until(&mut engine, stack_is_empty);

    let general =
        on_battlefield(&engine, P0, yellow_scarves_general()).expect("Yellow Scarves entered");
    assert_eq!(pt(&engine, general), (2, 2));
    let kw = keywords(&engine, general);
    assert!(kw.contains(KeywordSet::HORSEMANSHIP));
    assert!(kw.contains(KeywordSet::CANT_BLOCK));

    // P1's turn: neither Grizzly Bears nor a horsemanship attacker can be blocked by it.
    reach_their_main_phase(&mut engine, P1);
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    let cavalry = on_battlefield(&engine, P1, wu_elite_cavalry()).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (bears, Defender::Player(P0)),
                    (cavalry, Defender::Player(P0)),
                ],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(general, bears)],
                },
            )
            .is_err(),
        "Yellow Scarves General cannot block"
    );
    engine
        .apply(P0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    // P0's next turn: attacks past tapped cavalry and untapped Grizzly Bears (lacks horsemanship).
    reach_their_main_phase(&mut engine, P0);
    let blocks = attack_and_collect_blocks(&mut engine, general, P1);
    assert!(
        blocks.is_empty(),
        "untapped Grizzly Bears lacks horsemanship and tapped Wu Elite Cavalry cannot block"
    );
}
