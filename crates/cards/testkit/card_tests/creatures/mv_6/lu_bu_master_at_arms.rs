//! `cards/creatures/mv_6/lu_bu_master_at_arms.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn lu_bu_master_at_arms() -> CardIndex {
    card_index("ff959f1f-0982-4761-a6b0-3044dff1d617")
}

fn wu_elite_cavalry() -> CardIndex {
    card_index("7dc5d729-0553-46fb-aff7-2f0d9573837b")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Lu Bu, Master-at-Arms (ff959f1f-0982-4761-a6b0-3044dff1d617) — `{5}{R}` 4/3
/// Legendary Creature — Human Soldier Warrior:
/// "Haste; horsemanship"
#[test]
fn lu_bu_master_at_arms_attacks_immediately_with_haste_and_horsemanship() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[lu_bu_master_at_arms()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, lu_bu_master_at_arms());
    pass_until(&mut engine, stack_is_empty);

    let lu_bu = on_battlefield(&engine, P0, lu_bu_master_at_arms()).expect("Lu Bu entered");
    assert_eq!(pt(&engine, lu_bu), (4, 3));
    let kw = keywords(&engine, lu_bu);
    assert!(kw.contains(KeywordSet::HASTE));
    assert!(kw.contains(KeywordSet::HORSEMANSHIP));

    // Attacks on the very turn it entered thanks to haste, evading Grizzly Bears.
    let blocks = attack_and_collect_blocks(&mut engine, lu_bu, P1);
    assert!(blocks.is_empty());
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 16);

    let mut mirror = Duel::new(SEED, mountain())
        .battlefield(0, &[lu_bu_master_at_arms()])
        .battlefield(1, &[wu_elite_cavalry()])
        .start();
    keep_mulligans(&mut mirror);
    let lb = on_battlefield(&mirror, P0, lu_bu_master_at_arms()).unwrap();
    let cav = on_battlefield(&mirror, P1, wu_elite_cavalry()).unwrap();
    let blocks = attack_and_collect_blocks(&mut mirror, lb, P1);
    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == cav && b.attackers.contains(&lb))
    );
}
