//! `cards/creatures/mv_1/initiates_of_the_ebon_hand.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// The Initiates on player 0's battlefield, with exactly one red mana
/// floating: every activation pays {1} with whatever is in the pool and
/// puts {B} back, so one mana is enough to chain them.
fn table() -> (Engine<RegistryLookup>, ObjectId) {
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[card_index("0bf359ef-669f-41dc-8ab5-ba8e58b2f398")])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    e.state.players[0].mana_pool.add(ManaColor::Red, 1);
    e.refresh_offer();
    let src = on_battlefield(&e, P0, card_index("0bf359ef-669f-41dc-8ab5-ba8e58b2f398"))
        .expect("the Initiates are out");
    (e, src)
}

fn make_black(e: &mut Engine<RegistryLookup>, src: ObjectId) {
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: src,
            ability_index: 0,
        },
    )
    .expect("{1} is in the pool");
}

/// Initiates of the Ebon Hand (0bf359ef-669f-41dc-8ab5-ba8e58b2f398) — "{1}:
/// Add {B}. If this ability has been activated four or more times this turn,
/// sacrifice this creature at the beginning of the next end step."
#[test]
fn initiates_of_the_ebon_hand_turn_one_mana_into_black() {
    let (mut e, src) = table();
    assert_eq!(e.state().players[0].mana_pool.available(ManaColor::Red), 1);
    make_black(&mut e, src);
    let pool = &e.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 0, "the red paid the {{1}}");
    assert_eq!(pool.available(ManaColor::Black), 1, "and {{B}} came out");
    assert_eq!(pool.total(), 1);
}

#[test]
fn initiates_of_the_ebon_hand_activated_three_times_survive_the_turn() {
    let (mut e, src) = table();
    for _ in 0..3 {
        make_black(&mut e, src);
    }
    assert_eq!(
        e.state().players[0].mana_pool.available(ManaColor::Black),
        1
    );
    pass_until(&mut e, |e| e.state().turn.active == P1);
    assert!(
        on_battlefield(&e, P0, card_index("0bf359ef-669f-41dc-8ab5-ba8e58b2f398")).is_some(),
        "three activations are not enough"
    );
    assert!(in_graveyard(&e, P0, card_index("0bf359ef-669f-41dc-8ab5-ba8e58b2f398")).is_none());
}

#[test]
fn initiates_of_the_ebon_hand_activated_four_times_are_sacrificed_at_the_end_step() {
    let (mut e, src) = table();
    for _ in 0..4 {
        make_black(&mut e, src);
    }
    assert!(
        on_battlefield(&e, P0, card_index("0bf359ef-669f-41dc-8ab5-ba8e58b2f398")).is_some(),
        "not sacrificed on the spot"
    );

    pass_until(&mut e, |e| e.state().turn.step == Step::End);
    assert!(
        !stack_is_empty(&e),
        "the delayed sacrifice trigger waits on the stack"
    );
    assert!(on_battlefield(&e, P0, card_index("0bf359ef-669f-41dc-8ab5-ba8e58b2f398")).is_some());
    pass_until(&mut e, stack_is_empty);
    assert!(
        on_battlefield(&e, P0, card_index("0bf359ef-669f-41dc-8ab5-ba8e58b2f398")).is_none(),
        "sacrificed at the beginning of the end step"
    );
    assert!(in_graveyard(&e, P0, card_index("0bf359ef-669f-41dc-8ab5-ba8e58b2f398")).is_some());
}

#[test]
fn initiates_of_the_ebon_hand_count_starts_over_next_turn() {
    let (mut e, src) = table();
    for _ in 0..3 {
        make_black(&mut e, src);
    }
    pass_until(&mut e, |e| {
        e.state().turn.active == P0
            && e.state().turn.step == Step::Main
            && e.state().turn.number > 1
    });
    e.state.players[0].mana_pool.add(ManaColor::Red, 1);
    e.refresh_offer();
    make_black(&mut e, src);
    pass_until(&mut e, |e| e.state().turn.step == Step::End);
    pass_until(&mut e, stack_is_empty);
    assert!(
        on_battlefield(&e, P0, card_index("0bf359ef-669f-41dc-8ab5-ba8e58b2f398")).is_some(),
        "three last turn plus one this turn is not four in a turn"
    );
}
