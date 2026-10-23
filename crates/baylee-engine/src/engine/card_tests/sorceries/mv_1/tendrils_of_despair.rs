//! `cards/sorceries/mv_1/tendrils_of_despair.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn tendrils_of_despair() -> CardIndex {
    card_index("3b00f415-2496-4333-be89-601fd837fe7c")
}

fn llanowar_elves() -> CardIndex {
    card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Whether the spell may be cast now, mana already made or still to be made.
fn may_cast(e: &Engine<RegistryLookup>) -> bool {
    let id = in_hand(e, P0, tendrils_of_despair()).expect("the spell is in hand");
    let Pending::Priority { legal, .. } = e.pending() else {
        return false;
    };
    legal.castable.contains(&id) || legal.payable.contains(&id)
}

fn hand_size(e: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    e.state().zones.list(ZoneLocation::Hand(seat)).len()
}

/// Casts the Tendrils off one floating Black: names the opponent, then
/// returns the menu of creatures the sacrifice cost offers.
fn cast_and_name_the_opponent(e: &mut Engine<RegistryLookup>) -> Vec<ObjectId> {
    lk::float(e, P0, &[(ManaColor::Black, 1)]);
    cast_with_floating(e, P0, tendrils_of_despair());
    e.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    let Pending::ChooseCards {
        player, options, ..
    } = e.pending().clone()
    else {
        panic!(
            "the cost asks which creature to sacrifice, got {:?}",
            e.pending()
        )
    };
    assert_eq!(player, P0, "the caster picks the sacrifice");
    options
}

/// Tendrils of Despair (3b00f415-2496-4333-be89-601fd837fe7c) — "As an
/// additional cost to cast this spell, sacrifice a creature." With no creature
/// it cannot be cast, and a land is no substitute.
#[test]
fn tendrils_of_despair_cannot_be_cast_without_a_creature_to_sacrifice() {
    let without = lk::game(SEED, &[swamp()], &[], &[tendrils_of_despair()], &[swamp()]);
    assert!(!may_cast(&without), "nothing to sacrifice");

    let with = lk::game(
        SEED,
        &[swamp(), llanowar_elves()],
        &[],
        &[tendrils_of_despair()],
        &[swamp()],
    );
    assert!(may_cast(&with), "a creature pays the cost");
}

/// Tendrils of Despair — the caster sacrifices the creature of their choice
/// (the Elves, not the Bears; the Swamp is not on the menu), and "target
/// opponent discards two cards": the OPPONENT picks which two of their three
/// go, from their own hand; the caster's hand is untouched.
#[test]
fn tendrils_of_despair_sacrifices_your_choice_and_the_opponent_picks_two_discards() {
    let mut e = lk::game(
        SEED,
        &[swamp(), llanowar_elves(), grizzly_bears()],
        &[],
        &[tendrils_of_despair()],
        &[swamp(), island(), forest()],
    );
    let elves = lk::obj(&e, P0, llanowar_elves());
    let bears = lk::obj(&e, P0, grizzly_bears());
    let swamp_in_play = lk::obj(&e, P0, swamp());
    let their_swamp = in_hand(&e, P1, swamp()).expect("Swamp in hand");
    let their_island = in_hand(&e, P1, island()).expect("Island in hand");
    let their_forest = in_hand(&e, P1, forest()).expect("Forest in hand");

    let menu = cast_and_name_the_opponent(&mut e);
    assert!(
        menu.contains(&elves) && menu.contains(&bears),
        "any creature of yours: {menu:?}"
    );
    assert!(
        !menu.contains(&swamp_in_play),
        "a land is no creature: {menu:?}"
    );
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![elves],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player, options, ..
    } = e.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(player, P1, "the discarding player chooses");
    assert!(
        options.contains(&their_swamp)
            && options.contains(&their_island)
            && options.contains(&their_forest),
        "from their own hand: {options:?}"
    );
    e.apply(
        P1,
        PlayerAction::ChooseObjects {
            objects: vec![their_island, their_forest],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(
        in_graveyard(&e, P0, llanowar_elves()).is_some(),
        "sacrificed as the cost"
    );
    assert!(
        on_battlefield(&e, P0, grizzly_bears()).is_some(),
        "the other creature stays"
    );
    assert!(in_graveyard(&e, P1, island()).is_some() && in_graveyard(&e, P1, forest()).is_some());
    assert!(in_hand(&e, P1, swamp()).is_some(), "the third card stays");
    assert_eq!(hand_size(&e, P1), 1);
    assert_eq!(hand_size(&e, P0), 0, "the caster discards nothing");
}

/// Tendrils of Despair — "discards two cards" with only one in hand: that one
/// goes and the spell finishes without complaint.
#[test]
fn tendrils_of_despair_takes_what_the_opponent_has_when_it_is_less_than_two() {
    let mut e = lk::game(
        SEED,
        &[swamp(), llanowar_elves()],
        &[],
        &[tendrils_of_despair()],
        &[forest()],
    );
    let elves = lk::obj(&e, P0, llanowar_elves());

    cast_and_name_the_opponent(&mut e);
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![elves],
        },
    )
    .unwrap();
    for _ in 0..10 {
        match e.pending().clone() {
            Pending::ChooseCards {
                player, options, ..
            } => {
                e.apply(player, PlayerAction::ChooseObjects { objects: options })
                    .unwrap();
            }
            Pending::Priority { player, .. } if stack_is_empty(&e) => {
                let _ = player;
                break;
            }
            Pending::Priority { player, .. } => {
                e.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected question: {other:?}"),
        }
    }

    assert_eq!(hand_size(&e, P1), 0, "their only card was discarded");
    assert!(in_graveyard(&e, P1, forest()).is_some());
}
