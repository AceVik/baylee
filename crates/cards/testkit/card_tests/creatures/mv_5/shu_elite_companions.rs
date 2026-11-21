//! `cards/creatures/mv_5/shu_elite_companions.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn shu_elite_companions() -> CardIndex {
    card_index("edf304b8-523e-4233-8d77-1d45d33cbb86")
}

fn wu_elite_cavalry() -> CardIndex {
    card_index("7dc5d729-0553-46fb-aff7-2f0d9573837b")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Shu Elite Companions (edf304b8-523e-4233-8d77-1d45d33cbb86) — `{4}{W}` 3/3
/// Creature — Human Soldier:
/// "Horsemanship"
#[test]
fn shu_elite_companions_casts_as_three_three_with_horsemanship() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains(), plains()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[shu_elite_companions()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, shu_elite_companions());
    pass_until(&mut engine, stack_is_empty);

    let companions =
        on_battlefield(&engine, P0, shu_elite_companions()).expect("Companions entered");
    assert_eq!(pt(&engine, companions), (3, 3));
    assert!(keywords(&engine, companions).contains(KeywordSet::HORSEMANSHIP));

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    assert!(attack_and_collect_blocks(&mut engine, companions, P1).is_empty());
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 17);

    let mut mirror = Duel::new(SEED, plains())
        .battlefield(0, &[shu_elite_companions()])
        .battlefield(1, &[wu_elite_cavalry()])
        .start();
    keep_mulligans(&mut mirror);
    let comp = on_battlefield(&mirror, P0, shu_elite_companions()).unwrap();
    let cav = on_battlefield(&mirror, P1, wu_elite_cavalry()).unwrap();
    let blocks = attack_and_collect_blocks(&mut mirror, comp, P1);
    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == cav && b.attackers.contains(&comp))
    );
}
