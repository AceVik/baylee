//! `cards/creatures/mv_5/ma_chao_western_warrior.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn ma_chao_western_warrior() -> CardIndex {
    card_index("023e26d8-b67d-43dd-89b1-8905b1183094")
}

fn wu_elite_cavalry() -> CardIndex {
    card_index("7dc5d729-0553-46fb-aff7-2f0d9573837b")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Ma Chao, Western Warrior (023e26d8-b67d-43dd-89b1-8905b1183094) — `{3}{R}{R}` 3/3
/// Legendary Creature — Human Soldier Warrior:
/// - Horsemanship
/// - Whenever this creature attacks alone, it can't be blocked this combat.
#[test]
fn ma_chao_western_warrior_evades_even_horsemanship_blockers_when_attacking_alone() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                grizzly_bears(),
            ],
        )
        .battlefield(1, &[wu_elite_cavalry(), grizzly_bears()])
        .hand(0, &[ma_chao_western_warrior()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, ma_chao_western_warrior());
    pass_until(&mut engine, stack_is_empty);

    let ma_chao = on_battlefield(&engine, P0, ma_chao_western_warrior()).expect("Ma Chao entered");
    let ally = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    let p1_cav = on_battlefield(&engine, P1, wu_elite_cavalry()).unwrap();
    let p1_bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    assert_eq!(pt(&engine, ma_chao), (3, 3));
    assert!(keywords(&engine, ma_chao).contains(KeywordSet::HORSEMANSHIP));

    // Attacking alone: `AttacksAlone` makes Ma Chao unblockable even by Wu Elite Cavalry.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    let blocks_alone = attack_and_collect_blocks(&mut engine, ma_chao, P1);
    assert!(
        blocks_alone.is_empty(),
        "attacking alone makes Ma Chao unblockable even by a creature with horsemanship"
    );
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 17);

    // Attacking with an ally: `AttacksAlone` does not trigger; horsemanship still stops
    // Grizzly Bears, while Wu Elite Cavalry (with horsemanship) CAN block Ma Chao.
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
                    (ma_chao, Defender::Player(P1)),
                    (ally, Defender::Player(P1)),
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
                P1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(p1_bears, ma_chao)],
                },
            )
            .is_err(),
        "Grizzly Bears lacks horsemanship and still cannot block Ma Chao"
    );
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(p1_cav, ma_chao)],
            },
        )
        .expect("horsemanship blocker can block Ma Chao when Ma Chao did not attack alone");
}
