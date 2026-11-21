//! `cards/creatures/mv_4/yuan_shao_s_infantry.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn yuan_shao_s_infantry() -> CardIndex {
    card_index("8ad91015-db7f-43ea-97c9-bf2bf5656858")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Yuan Shao's Infantry (8ad91015-db7f-43ea-97c9-bf2bf5656858) — `{3}{R}` 2/2
/// Creature — Human Soldier:
/// "Whenever this creature attacks alone, it can't be blocked this combat."
#[test]
fn yuan_shao_s_infantry_is_unblockable_when_attacking_alone_but_blockable_with_ally() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                grizzly_bears(),
            ],
        )
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[yuan_shao_s_infantry()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, yuan_shao_s_infantry());
    pass_until(&mut engine, stack_is_empty);

    let infantry = on_battlefield(&engine, P0, yuan_shao_s_infantry()).expect("Infantry entered");
    let ally = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    let blocker = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    assert_eq!(pt(&engine, infantry), (2, 2));

    // Combat 1: attack alone -> unblockable this combat.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    let blocks_alone = attack_and_collect_blocks(&mut engine, infantry, P1);
    assert!(
        blocks_alone.is_empty(),
        "attacking alone makes it unblockable"
    );
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 18);
    assert!(
        !keywords(&engine, infantry).contains(KeywordSet::UNBLOCKABLE),
        "unblockable lasts only until end of combat"
    );

    // Combat 2: attack alongside Grizzly Bears -> `AttacksAlone` does not trigger.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (infantry, Defender::Player(P1)),
                    (ally, Defender::Player(P1)),
                ],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, infantry)],
            },
        )
        .expect("blocking Infantry is legal when it did not attack alone");
}
