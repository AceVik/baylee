//! `cards/creatures/mv_5/lu_meng_wu_general.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn lu_meng_wu_general() -> CardIndex {
    card_index("a1e2a966-6aad-4325-b644-3ced319bdd7d")
}

fn wu_elite_cavalry() -> CardIndex {
    card_index("7dc5d729-0553-46fb-aff7-2f0d9573837b")
}

fn air_elemental() -> CardIndex {
    card_index("7744bae4-a8b7-44a5-9b4c-0048ad4cc448")
}

/// Lu Meng, Wu General (a1e2a966-6aad-4325-b644-3ced319bdd7d) — `{3}{U}{U}` 4/4
/// Legendary Creature — Human Soldier:
/// "Horsemanship"
#[test]
fn lu_meng_wu_general_casts_as_four_four_with_horsemanship() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .battlefield(1, &[air_elemental()])
        .hand(0, &[lu_meng_wu_general()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, lu_meng_wu_general());
    pass_until(&mut engine, stack_is_empty);

    let lu_meng = on_battlefield(&engine, P0, lu_meng_wu_general()).expect("Lu Meng entered");
    assert_eq!(pt(&engine, lu_meng), (4, 4));
    assert!(keywords(&engine, lu_meng).contains(KeywordSet::HORSEMANSHIP));

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    assert!(attack_and_collect_blocks(&mut engine, lu_meng, P1).is_empty());
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 16);

    let mut mirror = Duel::new(SEED, island())
        .battlefield(0, &[lu_meng_wu_general()])
        .battlefield(1, &[wu_elite_cavalry()])
        .start();
    keep_mulligans(&mut mirror);
    let lm = on_battlefield(&mirror, P0, lu_meng_wu_general()).unwrap();
    let cav = on_battlefield(&mirror, P1, wu_elite_cavalry()).unwrap();
    let blocks = attack_and_collect_blocks(&mut mirror, lm, P1);
    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == cav && b.attackers.contains(&lm))
    );
}
