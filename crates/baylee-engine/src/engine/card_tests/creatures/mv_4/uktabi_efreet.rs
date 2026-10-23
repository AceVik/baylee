//! `cards/creatures/mv_4/uktabi_efreet.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_cards_dsl::counters;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn uktabi_efreet() -> CardIndex {
    card_index("2fd9a37f-151a-4501-9d8e-bc67cc902592")
}

fn upkeep(e: &mut Engine<RegistryLookup>, turn: u32, price: baylee_core::mana::ManaCost) {
    pass_until(e, |e| {
        e.state().turn.number == turn && matches!(e.pending(), Pending::YesNo { .. })
    });
    assert!(matches!(e.pending(), Pending::YesNo {
        player, prompt: YesNoPrompt::PayMana { cost }, ..
    } if *player == P0 && *cost == price));
}

/// Uktabi Efreet — cumulative upkeep {G}: one green, then two greens,
/// then three greens; the opponent's upkeep adds no counter. The first
/// payments consume mana, and declining the third sacrifices the Efreet.
#[test]
fn uktabi_efreet_requires_one_green_per_age_counter_on_its_own_upkeep() {
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[uktabi_efreet(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut e);
    let efreet = lk::obj(&e, P0, uktabi_efreet());
    assert_eq!(pt(&e, efreet), (5, 4));
    for (turn, price, count) in [
        (1, baylee_core::mana!("{G}"), 1),
        (3, baylee_core::mana!("{G}{G}"), 2),
    ] {
        upkeep(&mut e, turn, price);
        assert_eq!(counters_on(&e, efreet, counters::AGE), count);
        e.apply(P0, PlayerAction::YesNo(true)).unwrap();
        assert_eq!(tap_all_mana(&mut e, P0), 3);
        e.apply(P0, PlayerAction::PassPriority).unwrap();
        lk::settle(&mut e);
        assert_eq!(lk::pool_total(&e, P0), 3 - u64::from(count));
        assert!(on_battlefield(&e, P0, uktabi_efreet()).is_some());
        reach_their_main_phase(&mut e, P1);
        assert_eq!(counters_on(&e, efreet, counters::AGE), count);
    }
    upkeep(&mut e, 5, baylee_core::mana!("{G}{G}{G}"));
    assert_eq!(counters_on(&e, efreet, counters::AGE), 3);
    e.apply(P0, PlayerAction::YesNo(false)).unwrap();
    lk::settle(&mut e);
    assert!(in_graveyard(&e, P0, uktabi_efreet()).is_some());
}
