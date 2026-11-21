//! `cards/creatures/mv_4/wu_elite_cavalry.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn wu_elite_cavalry() -> CardIndex {
    card_index("7dc5d729-0553-46fb-aff7-2f0d9573837b")
}

fn shu_general() -> CardIndex {
    card_index("0fef6729-b044-4daf-834a-ac8c9711ad8a")
}

fn air_elemental() -> CardIndex {
    card_index("7744bae4-a8b7-44a5-9b4c-0048ad4cc448")
}

/// Wu Elite Cavalry (7dc5d729-0553-46fb-aff7-2f0d9573837b) — `{3}{U}` 2/3
/// Creature — Human Soldier:
/// "Horsemanship"
#[test]
fn wu_elite_cavalry_evades_non_horsemanship_blockers_and_interacts_with_horsemanship() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .battlefield(1, &[air_elemental()])
        .hand(0, &[wu_elite_cavalry()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, wu_elite_cavalry());
    pass_until(&mut engine, stack_is_empty);

    let cavalry = on_battlefield(&engine, P0, wu_elite_cavalry()).expect("Cavalry entered");
    assert_eq!(pt(&engine, cavalry), (2, 3));
    assert!(keywords(&engine, cavalry).contains(KeywordSet::HORSEMANSHIP));

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    let blocks = attack_and_collect_blocks(&mut engine, cavalry, P1);
    assert!(blocks.is_empty(), "Air Elemental lacks horsemanship");
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 18);

    // A creature with horsemanship can block Wu Elite Cavalry.
    let mut mirror = Duel::new(SEED, island())
        .battlefield(0, &[wu_elite_cavalry()])
        .battlefield(1, &[shu_general()])
        .start();
    keep_mulligans(&mut mirror);
    let cav = on_battlefield(&mirror, P0, wu_elite_cavalry()).unwrap();
    let shu = on_battlefield(&mirror, P1, shu_general()).unwrap();
    let blocks2 = attack_and_collect_blocks(&mut mirror, cav, P1);
    assert!(
        blocks2
            .iter()
            .any(|b| b.blocker == shu && b.attackers.contains(&cav))
    );
}
