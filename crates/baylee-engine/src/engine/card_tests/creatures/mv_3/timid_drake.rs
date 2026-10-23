//! `cards/creatures/mv_3/timid_drake.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn timid_drake() -> CardIndex {
    card_index("aa8c5467-190a-4c6a-872c-56b8f8d01a2b")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Timid Drake (aa8c5467-190a-4c6a-872c-56b8f8d01a2b) — "Flying. When another
/// creature enters, return this creature to its owner's hand." A 3/3 flyer;
/// its controller's own Bears entering sends only the Drake home, the Bears stay.
#[test]
fn timid_drake_returns_to_hand_when_another_creature_of_its_controller_enters() {
    let mut e = lk::game(
        SEED,
        &[timid_drake(), forest(), forest()],
        &[],
        &[grizzly_bears()],
        &[],
    );
    let drake = lk::obj(&e, P0, timid_drake());
    assert_eq!(pt(&e, drake), (3, 3));
    assert!(keywords(&e, drake).contains(KeywordSet::FLYING));

    cast_from_hand(&mut e, P0, grizzly_bears());
    pass_until(&mut e, stack_is_empty);

    assert!(
        on_battlefield(&e, P0, timid_drake()).is_none(),
        "the Drake left"
    );
    assert!(
        in_hand(&e, P0, timid_drake()).is_some(),
        "to its owner's hand"
    );
    assert!(
        on_battlefield(&e, P0, grizzly_bears()).is_some(),
        "the entering creature stays"
    );
}

/// Timid Drake — "another creature", whoever controls it: the opponent's Bears
/// entering on their turn sends the caster's Drake back to the caster's hand.
#[test]
fn timid_drake_also_returns_when_an_opponents_creature_enters() {
    let mut e = lk::game(
        SEED,
        &[timid_drake()],
        &[forest(), forest()],
        &[],
        &[grizzly_bears()],
    );
    pass_until(&mut e, |e| {
        e.state().turn.active == P1
            && e.state().turn.step == Step::Main
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P1)
    });

    cast_from_hand(&mut e, P1, grizzly_bears());
    pass_until(&mut e, stack_is_empty);

    assert!(on_battlefield(&e, P0, timid_drake()).is_none());
    assert!(
        in_hand(&e, P0, timid_drake()).is_some(),
        "it goes to its owner's hand"
    );
}

/// Timid Drake — only a **creature** entering matters: a land played by its
/// controller leaves it where it is.
#[test]
fn timid_drake_stays_when_a_land_enters() {
    let mut e = lk::game(SEED, &[timid_drake()], &[], &[forest()], &[]);
    let land = in_hand(&e, P0, forest()).expect("Forest in hand");

    e.apply(P0, PlayerAction::PlayLand { card: land }).unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(
        on_battlefield(&e, P0, timid_drake()).is_some(),
        "a land is no creature"
    );
}

/// Timid Drake — "another": the Drake's own entering does not send it home.
#[test]
fn timid_drake_stays_when_it_is_the_creature_that_enters() {
    let mut e = lk::game(
        SEED,
        &[island(), island(), island()],
        &[],
        &[timid_drake()],
        &[],
    );

    cast_from_hand(&mut e, P0, timid_drake());
    pass_until(&mut e, stack_is_empty);

    assert!(
        on_battlefield(&e, P0, timid_drake()).is_some(),
        "it is not 'another' creature"
    );
}
