//! `cards/creatures/mv_4/lu_xun_scholar_general.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn lu_xun_scholar_general() -> CardIndex {
    card_index("ea658352-abef-4201-b20c-f5c5809d1d3e")
}

fn wu_elite_cavalry() -> CardIndex {
    card_index("7dc5d729-0553-46fb-aff7-2f0d9573837b")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hand_size(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(seat)).len()
}

/// Lu Xun, Scholar General (ea658352-abef-4201-b20c-f5c5809d1d3e) — `{2}{U}{U}` 1/3
/// Legendary Creature — Human Soldier:
/// - Horsemanship
/// - Whenever this creature deals damage to an opponent, you may draw a card.
#[test]
fn lu_xun_scholar_general_has_horsemanship_and_may_draw_on_opponent_damage() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[lu_xun_scholar_general()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, lu_xun_scholar_general());
    pass_until(&mut engine, stack_is_empty);
    let lu_xun = on_battlefield(&engine, P0, lu_xun_scholar_general()).expect("Lu Xun entered");
    assert_eq!(pt(&engine, lu_xun), (1, 3));
    assert!(keywords(&engine, lu_xun).contains(KeywordSet::HORSEMANSHIP));

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    let before = hand_size(&engine, P0);

    let blocks = attack_and_collect_blocks(&mut engine, lu_xun, P1);
    assert!(blocks.is_empty(), "horsemanship evades Grizzly Bears");

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert_eq!(engine.state().players[1].life, 19);
    assert_eq!(hand_size(&engine, P0), before + 1);

    // Next combat: decline the optional draw.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    let before_decline = hand_size(&engine, P0);
    let _ = attack_and_collect_blocks(&mut engine, lu_xun, P1);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(hand_size(&engine, P0), before_decline);
}

#[test]
fn lu_xun_blocked_by_horsemanship_creature_deals_no_opponent_damage_and_does_not_trigger() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[lu_xun_scholar_general()])
        .battlefield(1, &[wu_elite_cavalry()])
        .start();
    keep_mulligans(&mut engine);

    let lu_xun = on_battlefield(&engine, P0, lu_xun_scholar_general()).unwrap();
    let cavalry = on_battlefield(&engine, P1, wu_elite_cavalry()).unwrap();
    let before = hand_size(&engine, P0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(lu_xun, Defender::Player(P1))],
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
                blockers: vec![(cavalry, lu_xun)],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 20);
    assert_eq!(hand_size(&engine, P0), before);
}
