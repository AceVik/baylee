//! `cards/creatures/mv_2/kavu_titan.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn kavu_titan() -> CardIndex {
    card_index("df166d0e-f944-404d-83b9-8cf53e220774")
}

/// Kavu Titan ({1}{G} 2/2 Kavu):
/// "Kicker {2}{G}; If this creature was kicked, it enters with three +1/+1
/// counters on it and with trample."
#[test]
fn kavu_titan_unkicked_is_two_two_without_trample_and_kicked_is_five_five_with_trample() {
    let mut e_base = lk::game(SEED, &[], &[llanowar_elves()], &[kavu_titan()], &[]);
    let elf_base = lk::obj(&e_base, lk::P1, llanowar_elves());
    lk::float(&mut e_base, lk::P0, &[(ManaColor::Green, 5)]);
    cast_with_floating(&mut e_base, lk::P0, kavu_titan());
    lk::yes_no(&mut e_base, false);
    lk::settle(&mut e_base);
    let base_id = lk::obj(&e_base, lk::P0, kavu_titan());
    assert_eq!(pt(&e_base, base_id), (2, 2));
    assert!(!keywords(&e_base, base_id).contains(KeywordSet::TRAMPLE));

    reach_their_main_phase(&mut e_base, lk::P1);
    reach_their_main_phase(&mut e_base, lk::P0);
    let _ = lk::attack(&mut e_base, &[base_id]);
    lk::block(&mut e_base, &[(elf_base, base_id)]);
    lk::through_combat(&mut e_base);
    assert_eq!(lk::life(&e_base, lk::P1), 20);

    // Kicked: 5/5 with 3 counters and persistent trample.
    let mut e_kick = lk::game(SEED, &[], &[llanowar_elves()], &[kavu_titan()], &[]);
    let elf_kick = lk::obj(&e_kick, lk::P1, llanowar_elves());
    lk::float(&mut e_kick, lk::P0, &[(ManaColor::Green, 5)]);
    cast_with_floating(&mut e_kick, lk::P0, kavu_titan());
    lk::yes_no(&mut e_kick, true);
    lk::settle(&mut e_kick);
    let kick_id = lk::obj(&e_kick, lk::P0, kavu_titan());
    assert_eq!(counters_on(&e_kick, kick_id, CounterKind::P1P1), 3);
    assert_eq!(pt(&e_kick, kick_id), (5, 5));
    assert!(keywords(&e_kick, kick_id).contains(KeywordSet::TRAMPLE));

    reach_their_main_phase(&mut e_kick, lk::P1);
    reach_their_main_phase(&mut e_kick, lk::P0);
    let _ = lk::attack(&mut e_kick, &[kick_id]);
    lk::block(&mut e_kick, &[(elf_kick, kick_id)]);
    lk::through_combat(&mut e_kick);
    assert_eq!(lk::life(&e_kick, lk::P1), 16);
}
