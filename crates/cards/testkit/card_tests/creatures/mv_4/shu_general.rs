//! `cards/creatures/mv_4/shu_general.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn shu_general() -> CardIndex {
    card_index("0fef6729-b044-4daf-834a-ac8c9711ad8a")
}

fn wu_elite_cavalry() -> CardIndex {
    card_index("7dc5d729-0553-46fb-aff7-2f0d9573837b")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Shu General (0fef6729-b044-4daf-834a-ac8c9711ad8a) — `{3}{W}` 2/2
/// Creature — Human Soldier:
/// "Vigilance; horsemanship"
#[test]
fn shu_general_attacks_untapped_with_vigilance_and_evades_without_horsemanship() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[shu_general()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, shu_general());
    pass_until(&mut engine, stack_is_empty);

    let general = on_battlefield(&engine, P0, shu_general()).expect("Shu General entered");
    assert_eq!(pt(&engine, general), (2, 2));
    let kw = keywords(&engine, general);
    assert!(kw.contains(KeywordSet::VIGILANCE));
    assert!(kw.contains(KeywordSet::HORSEMANSHIP));

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);

    let blocks = attack_and_collect_blocks(&mut engine, general, P1);
    assert!(blocks.is_empty(), "Grizzly Bears lacks horsemanship");
    assert!(
        !engine
            .state()
            .object(general)
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "vigilance keeps Shu General untapped when attacking"
    );

    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 18);
}

#[test]
fn shu_general_can_be_blocked_by_horsemanship_creature() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[shu_general()])
        .battlefield(1, &[wu_elite_cavalry()])
        .start();
    keep_mulligans(&mut engine);

    let general = on_battlefield(&engine, P0, shu_general()).unwrap();
    let cavalry = on_battlefield(&engine, P1, wu_elite_cavalry()).unwrap();
    let blocks = attack_and_collect_blocks(&mut engine, general, P1);
    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == cavalry && b.attackers.contains(&general))
    );
}
