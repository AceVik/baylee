//! `cards/creatures/mv_5/barbarian_general.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn barbarian_general() -> CardIndex {
    card_index("d4f2e305-bec0-48c8-8040-f85618c71cc9")
}

fn wu_elite_cavalry() -> CardIndex {
    card_index("7dc5d729-0553-46fb-aff7-2f0d9573837b")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Barbarian General (d4f2e305-bec0-48c8-8040-f85618c71cc9) — `{4}{R}` 3/2
/// Creature — Human Barbarian Soldier:
/// "Horsemanship"
#[test]
fn barbarian_general_casts_and_evades_blockers_without_horsemanship() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[barbarian_general()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, barbarian_general());
    pass_until(&mut engine, stack_is_empty);

    let general = on_battlefield(&engine, P0, barbarian_general()).expect("General entered");
    assert_eq!(pt(&engine, general), (3, 2));
    assert!(keywords(&engine, general).contains(KeywordSet::HORSEMANSHIP));

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    assert!(attack_and_collect_blocks(&mut engine, general, P1).is_empty());
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 17);

    let mut mirror = Duel::new(SEED, mountain())
        .battlefield(0, &[barbarian_general()])
        .battlefield(1, &[wu_elite_cavalry()])
        .start();
    keep_mulligans(&mut mirror);
    let barb = on_battlefield(&mirror, P0, barbarian_general()).unwrap();
    let cav = on_battlefield(&mirror, P1, wu_elite_cavalry()).unwrap();
    let blocks = attack_and_collect_blocks(&mut mirror, barb, P1);
    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == cav && b.attackers.contains(&barb))
    );
}
