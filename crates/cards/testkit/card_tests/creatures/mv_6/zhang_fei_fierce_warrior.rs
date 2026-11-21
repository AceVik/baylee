//! `cards/creatures/mv_6/zhang_fei_fierce_warrior.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn zhang_fei_fierce_warrior() -> CardIndex {
    card_index("f3f815b8-75c6-40e0-9716-872f0f5c9d46")
}

fn wu_elite_cavalry() -> CardIndex {
    card_index("7dc5d729-0553-46fb-aff7-2f0d9573837b")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Zhang Fei, Fierce Warrior (f3f815b8-75c6-40e0-9716-872f0f5c9d46) — `{4}{W}{W}` 4/4
/// Legendary Creature — Human Soldier Warrior:
/// "Vigilance; horsemanship"
#[test]
fn zhang_fei_fierce_warrior_attacks_untapped_with_vigilance_and_horsemanship() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), plains(), plains()],
        )
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[zhang_fei_fierce_warrior()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, zhang_fei_fierce_warrior());
    pass_until(&mut engine, stack_is_empty);

    let zf = on_battlefield(&engine, P0, zhang_fei_fierce_warrior()).expect("Zhang Fei entered");
    assert_eq!(pt(&engine, zf), (4, 4));
    let kw = keywords(&engine, zf);
    assert!(kw.contains(KeywordSet::VIGILANCE));
    assert!(kw.contains(KeywordSet::HORSEMANSHIP));

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    assert!(attack_and_collect_blocks(&mut engine, zf, P1).is_empty());
    assert!(
        !engine
            .state()
            .object(zf)
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "vigilance keeps Zhang Fei untapped"
    );

    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 16);

    let mut mirror = Duel::new(SEED, plains())
        .battlefield(0, &[zhang_fei_fierce_warrior()])
        .battlefield(1, &[wu_elite_cavalry()])
        .start();
    keep_mulligans(&mut mirror);
    let zf2 = on_battlefield(&mirror, P0, zhang_fei_fierce_warrior()).unwrap();
    let cav = on_battlefield(&mirror, P1, wu_elite_cavalry()).unwrap();
    let blocks = attack_and_collect_blocks(&mut mirror, zf2, P1);
    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == cav && b.attackers.contains(&zf2))
    );
}
