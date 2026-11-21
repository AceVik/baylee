//! `cards/sorceries/mv_3/riding_red_hare.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn riding_red_hare() -> CardIndex {
    card_index("ddd45d52-20c3-4731-8860-142af1ccaaa0")
}

fn barbarian_general() -> CardIndex {
    card_index("d4f2e305-bec0-48c8-8040-f85618c71cc9")
}

/// Riding Red Hare (ddd45d52-20c3-4731-8860-142af1ccaaa0) — {2}{W} Sorcery:
/// "Target creature gets +3/+3 and gains horsemanship until end of turn."
#[test]
fn riding_red_hare_grants_three_three_and_horsemanship_until_end_of_turn() {
    let mut engine = lk::game(
        SEED,
        &[plains(), plains(), plains(), aurochs()],
        &[fangren_hunter(), barbarian_general()],
        &[riding_red_hare()],
        &[],
    );
    let my_beast = lk::obj(&engine, P0, aurochs());
    let ground_blocker = lk::obj(&engine, P1, fangren_hunter());
    let horse_blocker = lk::obj(&engine, P1, barbarian_general());

    cast_from_hand(&mut engine, P0, riding_red_hare());
    aim_at(&mut engine, P0, my_beast);
    lk::settle(&mut engine);

    assert_eq!(pt(&engine, my_beast), (5, 6), "2/3 + 3/3 = 5/6");
    assert!(keywords(&engine, my_beast).contains(KeywordSet::HORSEMANSHIP));

    let offer = lk::attack(&mut engine, &[my_beast]);
    assert!(
        !lk::may_block(&offer, ground_blocker, my_beast),
        "creature without horsemanship cannot block it"
    );
    assert!(
        lk::may_block(&offer, horse_blocker, my_beast),
        "creature with horsemanship may block it"
    );
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, my_beast), (2, 3));
    assert!(!keywords(&engine, my_beast).contains(KeywordSet::HORSEMANSHIP));
}
