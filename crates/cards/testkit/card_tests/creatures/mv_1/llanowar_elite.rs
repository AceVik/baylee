//! `cards/creatures/mv_1/llanowar_elite.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn llanowar_elite() -> CardIndex {
    card_index("9e112393-962b-461e-a605-9f8d6a1ebaa4")
}

/// Llanowar Elite ({G} 1/1 Elf):
/// "Kicker {8}; Trample; If this creature was kicked, it enters with five
/// +1/+1 counters on it."
#[test]
fn llanowar_elite_unkicked_is_one_one_and_kicked_enters_six_six_with_trample() {
    // Unkicked: declines {8}, enters as 1/1 with 0 counters and trample.
    let mut e_base = lk::game(SEED, &[], &[], &[llanowar_elite()], &[]);
    lk::float(&mut e_base, lk::P0, &[(ManaColor::Green, 9)]);
    cast_with_floating(&mut e_base, lk::P0, llanowar_elite());
    lk::yes_no(&mut e_base, false);
    lk::settle(&mut e_base);
    let base_id = lk::obj(&e_base, lk::P0, llanowar_elite());
    assert_eq!(counters_on(&e_base, base_id, CounterKind::P1P1), 0);
    assert_eq!(pt(&e_base, base_id), (1, 1));
    assert!(keywords(&e_base, base_id).contains(KeywordSet::TRAMPLE));

    // Kicked: pays {8}{G}, enters with 5 +1/+1 counters (6/6), tramples 5 over a 1/1.
    let mut e_kick = lk::game(SEED, &[], &[llanowar_elves()], &[llanowar_elite()], &[]);
    let blocker = lk::obj(&e_kick, lk::P1, llanowar_elves());
    lk::float(&mut e_kick, lk::P0, &[(ManaColor::Green, 9)]);
    cast_with_floating(&mut e_kick, lk::P0, llanowar_elite());
    lk::yes_no(&mut e_kick, true);
    lk::settle(&mut e_kick);
    assert_eq!(lk::pool_total(&e_kick, lk::P0), 0);

    let kicked_id = lk::obj(&e_kick, lk::P0, llanowar_elite());
    assert_eq!(counters_on(&e_kick, kicked_id, CounterKind::P1P1), 5);
    assert_eq!(pt(&e_kick, kicked_id), (6, 6));

    reach_their_main_phase(&mut e_kick, lk::P1);
    reach_their_main_phase(&mut e_kick, lk::P0);
    let _ = lk::attack(&mut e_kick, &[kicked_id]);
    lk::block(&mut e_kick, &[(blocker, kicked_id)]);
    lk::through_combat(&mut e_kick);
    assert_eq!(lk::life(&e_kick, lk::P1), 15);
}
