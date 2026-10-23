//! `cards/enchantments/mv_2/goblin_bomb.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_cards_dsl::counters;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn goblin_bomb() -> CardIndex {
    card_index("061e2ef6-ce8d-4319-b525-96b9c2824001")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// P0's Goblin Bomb and P1's Bears (P1 at 30 life), run through P0's first
/// upkeep (the "you may flip" is declined) to P0's first main phase.
fn bomb_in_main_phase() -> (Engine<RegistryLookup>, ObjectId) {
    let mut e = Duel::new(SEED, mountain())
        .battlefield(0, &[goblin_bomb()])
        .battlefield(1, &[grizzly_bears()])
        .life(1, 30)
        .start();
    keep_mulligans(&mut e);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(P0, PlayerAction::YesNo(false)).unwrap();
    reach_main_phase(&mut e, P0);
    let bomb = on_battlefield(&e, P0, goblin_bomb()).expect("the Bomb is out");
    (e, bomb)
}

/// Puts `n` fuse counters on the Bomb, the harness way.
fn set_fuse(e: &mut Engine<RegistryLookup>, bomb: ObjectId, n: u16) {
    e.state
        .object_mut(bomb)
        .expect("the Bomb is an object")
        .counters
        .set(counters::FUSE, n);
    e.refresh_offer();
}

/// Goblin Bomb (061e2ef6-ce8d-4319-b525-96b9c2824001) — "Remove five fuse
/// counters from this enchantment and sacrifice it: It deals 20 damage to
/// target player or planeswalker." Four counters are not enough; five are;
/// a creature is not a legal target; the chosen player takes 20 and the Bomb
/// is gone.
#[test]
fn goblin_bomb_needs_five_fuse_counters_and_deals_twenty_to_a_player() {
    let (mut e, bomb) = bomb_in_main_phase();

    set_fuse(&mut e, bomb, 4);
    assert!(
        !lk::offered(&e, goblin_bomb(), P0, 1),
        "four counters are one short"
    );
    set_fuse(&mut e, bomb, 5);
    assert!(lk::offered(&e, goblin_bomb(), P0, 1), "five are enough");

    activate(&mut e, P0, goblin_bomb(), 1);
    let theirs = on_battlefield(&e, P1, grizzly_bears()).expect("their Bears");
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("a target is asked for, got {:?}", e.pending());
    };
    assert!(
        !options.contains(&theirs),
        "a creature is not a player or planeswalker: {options:?}"
    );
    assert!(
        e.apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![theirs],
                players: vec![],
            },
        )
        .is_err(),
        "the Bears are refused"
    );
    e.apply(
        P0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![P1],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(in_graveyard(&e, P0, goblin_bomb()).is_some(), "sacrificed");
    assert_eq!(lk::life(&e, P1), 10, "20 of 30");
    assert_eq!(lk::life(&e, P0), 20);
    assert_eq!(lk::damage_on(&e, theirs), 0);
}

/// Goblin Bomb — "At the beginning of your upkeep, you may flip a coin. If you
/// win … put a fuse counter … If you lose … remove a fuse counter": starting
/// at two, the flip leaves one or three; P1's upkeep in between asked
/// nothing (the count was still two at P1's main phase).
#[test]
fn goblin_bomb_flip_adds_or_removes_exactly_one_fuse_counter_on_its_controllers_upkeep() {
    let (mut e, bomb) = bomb_in_main_phase();
    set_fuse(&mut e, bomb, 2);

    reach_their_main_phase(&mut e, P1);
    assert_eq!(
        counters_on(&e, bomb, counters::FUSE),
        2,
        "P1's upkeep is not yours"
    );

    pass_until(&mut e, |e| {
        e.state().turn.number == 3 && matches!(e.pending(), Pending::YesNo { .. })
    });
    e.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut e, stack_is_empty);

    let after = counters_on(&e, bomb, counters::FUSE);
    assert!(
        after == 1 || after == 3,
        "won or lost the flip, one counter either way: {after}"
    );
}

/// Goblin Bomb — "you *may* flip": declined, the count is untouched.
#[test]
fn goblin_bomb_flip_declined_changes_nothing() {
    let (mut e, bomb) = bomb_in_main_phase();
    set_fuse(&mut e, bomb, 2);

    pass_until(&mut e, |e| {
        e.state().turn.number == 3 && matches!(e.pending(), Pending::YesNo { .. })
    });
    e.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut e, stack_is_empty);

    assert_eq!(counters_on(&e, bomb, counters::FUSE), 2);
}
