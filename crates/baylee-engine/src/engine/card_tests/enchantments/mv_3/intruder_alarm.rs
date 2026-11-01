//! `cards/enchantments/mv_3/intruder_alarm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn intruder_alarm() -> CardIndex {
    card_index("1e943e04-e213-4781-b1a7-935aad8790e1")
}

/// Intruder Alarm (`1e943e04-e213-4781-b1a7-935aad8790e1`) — {2}{U} Enchantment (Implemented):
/// "Creatures don't untap during their controllers' untap steps.
/// Whenever a creature enters, untap all creatures."
#[test]
fn intruder_alarm_untaps_all_creatures_when_a_creature_enters() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[intruder_alarm(), index::LLANOWAR_ELVES, forest(), forest()],
        )
        .hand(0, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let elf = on_battlefield(&engine, P0, index::LLANOWAR_ELVES).unwrap();
    engine
        .dev_state_mut(P0)
        .unwrap()
        .object_mut(elf)
        .unwrap()
        .status
        .insert(Status::TAPPED);
    assert!(is_tapped(&engine, elf));

    cast_from_hand(&mut engine, P0, index::GRIZZLY_BEARS);
    pass_until(&mut engine, stack_is_empty);

    assert!(!is_tapped(&engine, elf));
}

#[test]
fn creatures_skip_their_untap_but_both_sides_untap_for_opponent_creature_entry() {
    let mut e = lk::game(
        SEED,
        &[intruder_alarm(), index::GRIZZLY_BEARS, forest()],
        &[index::GRIZZLY_BEARS],
        &[],
        &[index::GRIZZLY_BEARS, island()],
    );
    let mine = lk::obj(&e, P0, index::GRIZZLY_BEARS);
    let theirs = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);
    let land = lk::obj(&e, P0, forest());
    for id in [mine, theirs, land] {
        e.dev_state_mut(P0)
            .unwrap()
            .object_mut(id)
            .unwrap()
            .status
            .insert(Status::TAPPED);
    }
    reach_their_main_phase(&mut e, lk::P1);
    assert!(lk::tapped(&e, mine));
    assert!(
        lk::tapped(&e, theirs),
        "their own untap step cannot untap them"
    );
    play_land(&mut e, lk::P1, island());
    lk::settle(&mut e);
    assert!(lk::tapped(&e, mine));
    assert!(lk::tapped(&e, theirs));
    lk::float(&mut e, lk::P1, &[(ManaColor::Green, 2)]);
    cast_with_floating(&mut e, lk::P1, index::GRIZZLY_BEARS);
    lk::settle(&mut e);
    assert!(!lk::tapped(&e, mine));
    assert!(!lk::tapped(&e, theirs));
    assert!(lk::tapped(&e, land), "entry trigger does not untap lands");
}
