//! `cards/creatures/mv_5/lady_zhurong_warrior_queen.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn lady_zhurong_warrior_queen() -> CardIndex {
    card_index("531e6ffb-28e0-4e8e-ad88-2f3a243789cf")
}

fn wu_elite_cavalry() -> CardIndex {
    card_index("7dc5d729-0553-46fb-aff7-2f0d9573837b")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Lady Zhurong, Warrior Queen (531e6ffb-28e0-4e8e-ad88-2f3a243789cf) — `{4}{G}` 4/3
/// Legendary Creature — Human Soldier Warrior:
/// "Horsemanship"
#[test]
fn lady_zhurong_warrior_queen_casts_as_legendary_four_three_with_horsemanship() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .battlefield(1, &[hill_giant()])
        .hand(0, &[lady_zhurong_warrior_queen()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, lady_zhurong_warrior_queen());
    pass_until(&mut engine, stack_is_empty);

    let lady =
        on_battlefield(&engine, P0, lady_zhurong_warrior_queen()).expect("Lady Zhurong entered");
    assert_eq!(pt(&engine, lady), (4, 3));
    assert!(keywords(&engine, lady).contains(KeywordSet::HORSEMANSHIP));
    assert!(
        engine
            .state()
            .object(lady)
            .unwrap()
            .characteristics()
            .supertypes
            .contains(SupertypeSet::LEGENDARY)
    );

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    assert!(attack_and_collect_blocks(&mut engine, lady, P1).is_empty());
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 16);

    let mut mirror = Duel::new(SEED, forest())
        .battlefield(0, &[lady_zhurong_warrior_queen()])
        .battlefield(1, &[wu_elite_cavalry()])
        .start();
    keep_mulligans(&mut mirror);
    let lz = on_battlefield(&mirror, P0, lady_zhurong_warrior_queen()).unwrap();
    let cav = on_battlefield(&mirror, P1, wu_elite_cavalry()).unwrap();
    let blocks = attack_and_collect_blocks(&mut mirror, lz, P1);
    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == cav && b.attackers.contains(&lz))
    );
}
