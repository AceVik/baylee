//! `cards/creatures/mv_6/sun_quan_lord_of_wu.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn sun_quan_lord_of_wu() -> CardIndex {
    card_index("70503280-6bdf-431d-8213-367a9783463a")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn murder() -> CardIndex {
    card_index("938b4e2c-88d9-4637-bc00-e228920c9a78")
}

/// Sun Quan, Lord of Wu (70503280-6bdf-431d-8213-367a9783463a) — `{4}{U}{U}` 4/4
/// Legendary Creature — Human Soldier:
/// "Creatures you control have horsemanship."
#[test]
fn sun_quan_lord_of_wu_grants_horsemanship_to_all_creatures_you_control_while_on_board() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                grizzly_bears(),
            ],
        )
        .battlefield(1, &[grizzly_bears(), swamp(), swamp(), swamp()])
        .hand(0, &[sun_quan_lord_of_wu()])
        .hand(1, &[murder()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let p0_bear = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    let p1_bear = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    assert!(!keywords(&engine, p0_bear).contains(KeywordSet::HORSEMANSHIP));

    cast_from_hand(&mut engine, P0, sun_quan_lord_of_wu());
    pass_until(&mut engine, stack_is_empty);

    let sun_quan = on_battlefield(&engine, P0, sun_quan_lord_of_wu()).expect("Sun Quan entered");
    assert_eq!(pt(&engine, sun_quan), (4, 4));
    assert!(keywords(&engine, sun_quan).contains(KeywordSet::HORSEMANSHIP));
    assert!(keywords(&engine, p0_bear).contains(KeywordSet::HORSEMANSHIP));
    assert!(
        !keywords(&engine, p1_bear).contains(KeywordSet::HORSEMANSHIP),
        "opponent's creatures do not gain horsemanship"
    );

    // P0's Grizzly Bears attacks unblockably past P1's Grizzly Bears.
    let blocks = attack_and_collect_blocks(&mut engine, p0_bear, P1);
    assert!(blocks.is_empty());
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 18);

    // Destroying Sun Quan removes horsemanship from P0's Grizzly Bears immediately.
    reach_their_main_phase(&mut engine, P1);
    cast_from_hand(&mut engine, P1, murder());
    aim_at(&mut engine, P1, sun_quan);
    pass_until(&mut engine, stack_is_empty);
    assert!(!keywords(&engine, p0_bear).contains(KeywordSet::HORSEMANSHIP));
}
