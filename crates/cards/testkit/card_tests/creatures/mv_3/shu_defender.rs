//! `cards/creatures/mv_3/shu_defender.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn shu_defender() -> CardIndex {
    card_index("0b6625cb-4177-45bd-ba99-2af43dfd6f46")
}

/// Shu Defender ({2}{W} 2/2 Human Soldier):
/// "Whenever this creature blocks, it gets +0/+2 until end of turn."
#[test]
fn shu_defender_gets_plus_zero_plus_two_only_when_blocking_until_end_of_turn() {
    let mut e = lk::game(
        SEED,
        &[shu_defender()],
        &[llanowar_elves()],
        &[index::SHOCK],
        &[],
    );
    let defender = lk::obj(&e, lk::P0, shu_defender());
    let elf = lk::obj(&e, lk::P1, llanowar_elves());

    let _ = lk::attack(&mut e, &[defender]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(pt(&e, defender), (2, 2));

    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    reach_their_main_phase(&mut e, lk::P1);
    let _ = lk::attack(&mut e, &[elf]);
    lk::block(&mut e, &[(defender, elf)]);
    lk::settle(&mut e);
    assert_eq!(pt(&e, defender), (2, 4));

    lk::through_combat(&mut e);
    e.apply(lk::P1, PlayerAction::PassPriority).unwrap();
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, lk::P0, index::SHOCK, &[defender], &[]);
    assert!(on_battlefield(&e, lk::P0, shu_defender()).is_some());

    reach_their_main_phase(&mut e, lk::P0);
    assert_eq!(pt(&e, defender), (2, 2));
}
