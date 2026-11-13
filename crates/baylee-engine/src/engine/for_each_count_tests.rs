//! "For each creature, its controller sacrifices a permanent of their choice
//! unless they pay {1}" (Fade Away), with real cards.
//!
//! The creatures are counted once, as the spell resolves (CR 608.2h); the
//! active player makes all of their choices, then each other player in turn
//! order (CR 101.4); and every permanent chosen is sacrificed at the same
//! time, after the last choice. Fade Away's ruling of 2004-10-04 says the
//! same, and draws the consequence: "you can't sacrifice one creature in
//! order to pay for another and thereby avoid payment for the first."
//!
//! Carbon's ladder game seg2 attempt-000000033 met the engine as it was
//! twice (`for_each::ONE_AT_A_TIME` switches it back on, for each test's
//! control): on turn 12 the nonactive player answered before and between the
//! active player's choices, and on turn 22 a player with two creatures and no
//! mana sacrificed one of them for the other and lost one permanent, not two.

use super::testkit::{
    Duel, RegistryLookup, card_index, cast_from_hand, keep_mulligans, on_battlefield, pass_until,
    reach_their_main_phase,
};
use super::*;
use crate::choice::PaymentStep;
use crate::zone::Zone;
use baylee_core::generated::index;
use baylee_core::ids::{CardIndex, ObjectId};

const SEED: u64 = 0xfade_0033;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
const P2: PlayerId = PlayerId::new(2);

type Game = Engine<RegistryLookup>;

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

/// What was asked of whom, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Asked {
    /// "Pay {1}?"
    Pay(PlayerId),
    /// "Which permanent?"
    Sacrifice(PlayerId),
}

/// Runs `f` on the engine as it was before the count.
fn as_before<T>(f: impl FnOnce() -> T) -> T {
    let set = |on: bool| crate::resolve::for_each::ONE_AT_A_TIME.with(|s| s.set(on));
    set(true);
    let out = f();
    set(false);
    out
}

/// A table of `seats` on seat 1's first main phase, seat 1 holding Fade Away
/// over three Islands, and the boards `boards` (seat by seat; seat 1's
/// Islands come first on its own). Fade Away is cast and both passes made:
/// the first question of its resolution is pending.
fn resolving(seats: usize, boards: &[&[CardIndex]]) -> Game {
    let mut table = Duel::table(SEED, island(), seats).hand(1, &[index::FADE_AWAY]);
    for (seat, board) in boards.iter().enumerate() {
        let mut cards = if seat == 1 {
            vec![island(), island(), island()]
        } else {
            Vec::new()
        };
        cards.extend_from_slice(board);
        table = table.battlefield(seat, &cards);
    }
    let mut engine = table.start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);
    cast_from_hand(&mut engine, P1, index::FADE_AWAY);
    pass_until(&mut engine, |e| {
        !matches!(e.pending(), Pending::Priority { .. })
    });
    engine
}

/// Answers Fade Away's questions until it has resolved: `pay` says who pays
/// (by answering yes and paying with `payment`), `choose` picks each
/// permanent. Returns what was asked, in order.
fn answer(
    engine: &mut Game,
    pay: impl Fn(PlayerId) -> Option<Vec<PaymentStep>>,
    mut choose: impl FnMut(PlayerId, &[ObjectId]) -> ObjectId,
) -> Vec<Asked> {
    let mut asked = Vec::new();
    loop {
        match engine.pending().clone() {
            Pending::YesNo { player, .. } => {
                asked.push(Asked::Pay(player));
                match pay(player) {
                    Some(steps) => {
                        engine.apply(player, PlayerAction::YesNo(true)).unwrap();
                        engine
                            .apply(player, PlayerAction::PayWindow { steps })
                            .expect("the payment covers {1}");
                    }
                    None => engine.apply(player, PlayerAction::YesNo(false)).unwrap(),
                }
            }
            Pending::ChooseCards {
                player, options, ..
            } => {
                asked.push(Asked::Sacrifice(player));
                let pick = choose(player, &options);
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![pick],
                        },
                    )
                    .unwrap();
            }
            _ => return asked,
        }
    }
}

fn nobody_pays(_: PlayerId) -> Option<Vec<PaymentStep>> {
    None
}

fn zone(engine: &Game, id: ObjectId) -> Option<Zone> {
    engine.state().object(id).map(|o| o.zone)
}

/// `seat`'s permanents that are `card`, in battlefield order.
fn all_of(engine: &Game, seat: PlayerId, card: CardIndex) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.controller == seat && o.card.is_some_and(|c| c.index == card))
        })
        .collect()
}

/// Seat 0's creature stands before seat 1's on the battlefield, so the
/// order of objects asks seat 0 first. Seat 1 is the active player and
/// makes its choices first: whether it pays, then which permanent it gives
/// up, and only then is seat 0 asked anything (CR 101.4).
#[test]
fn the_active_player_chooses_first_whatever_the_battlefield_order() {
    let play = || {
        let mut engine = resolving(
            2,
            &[&[index::GRIZZLY_BEARS, forest()], &[index::HILL_GIANT]],
        );
        let land = all_of(&engine, P0, forest())[0];
        let tapped = all_of(&engine, P1, island())[0];
        let asked = answer(&mut engine, nobody_pays, |player, _| {
            if player == P0 { land } else { tapped }
        });
        (engine, asked, land, tapped)
    };
    let (engine, asked, land, tapped) = play();
    assert_eq!(
        asked,
        [
            Asked::Pay(P1),
            Asked::Sacrifice(P1),
            Asked::Pay(P0),
            Asked::Sacrifice(P0)
        ],
        "the active player's choices first, then the other player's"
    );
    assert_eq!(zone(&engine, land), Some(Zone::Graveyard));
    assert_eq!(zone(&engine, tapped), Some(Zone::Graveyard));
    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&engine, P1, index::HILL_GIANT).is_some());

    let (_, before, _, _) = as_before(play);
    assert_eq!(
        before,
        [
            Asked::Pay(P0),
            Asked::Sacrifice(P0),
            Asked::Pay(P1),
            Asked::Sacrifice(P1)
        ],
        "the control: the engine as it was asked in battlefield order"
    );
}

/// At a table of three the order is the turn's: the active player (seat 1),
/// then seat 2, then seat 0, each with all of its choices.
#[test]
fn the_other_players_follow_in_turn_order() {
    let mut engine = resolving(
        3,
        &[
            &[index::GRIZZLY_BEARS, forest()],
            &[index::HILL_GIANT],
            &[index::GRIZZLY_BEARS, forest()],
        ],
    );
    let asked = answer(&mut engine, nobody_pays, |_, options| options[0]);
    assert_eq!(
        asked,
        [
            Asked::Pay(P1),
            Asked::Sacrifice(P1),
            Asked::Pay(P2),
            Asked::Sacrifice(P2),
            Asked::Pay(P0),
            Asked::Sacrifice(P0)
        ]
    );
}

/// Two creatures and no mana: two permanents, here both creatures. The
/// count is taken as Fade Away begins, so the second creature still counts
/// after it was chosen for the first, and it is not offered twice.
#[test]
fn a_creature_chosen_for_another_still_counts() {
    let play = || {
        let mut engine = resolving(2, &[&[index::GRIZZLY_BEARS, index::GRIZZLY_BEARS], &[]]);
        let bears = all_of(&engine, P0, index::GRIZZLY_BEARS);
        let mut offered = Vec::new();
        let asked = answer(&mut engine, nobody_pays, |_, options| {
            offered.push(options.to_vec());
            options[options.len() - 1]
        });
        (engine, asked, bears, offered)
    };
    let (engine, asked, bears, offered) = play();
    assert_eq!(
        asked,
        [
            Asked::Pay(P0),
            Asked::Pay(P0),
            Asked::Sacrifice(P0),
            Asked::Sacrifice(P0)
        ],
        "both payments, then both sacrifices"
    );
    assert_eq!(
        offered,
        [vec![bears[0], bears[1]], vec![bears[0]]],
        "a permanent already chosen is not offered again"
    );
    for bear in &bears {
        assert_eq!(zone(&engine, *bear), Some(Zone::Graveyard));
    }

    let (engine, before, bears, _) = as_before(play);
    assert_eq!(before, [Asked::Pay(P0), Asked::Sacrifice(P0)]);
    assert_eq!(
        bears
            .iter()
            .filter(|&&bear| zone(&engine, bear) == Some(Zone::Battlefield))
            .count(),
        1,
        "the control: as it was, the second Bears was skipped and survived"
    );
}

/// A creature sacrificed for mana to pay for another still counts: Blood
/// Pet pays for the Bears and is gone, and its own {1} is still owed.
/// Unpaid, it costs the Bears.
#[test]
fn a_creature_sacrificed_for_mana_still_counts() {
    let play = || {
        let mut engine = resolving(2, &[&[index::GRIZZLY_BEARS, index::BLOOD_PET], &[]]);
        let bears = all_of(&engine, P0, index::GRIZZLY_BEARS)[0];
        let pet = all_of(&engine, P0, index::BLOOD_PET)[0];
        let paid = std::cell::Cell::new(false);
        let asked = answer(
            &mut engine,
            |_| {
                (!paid.replace(true)).then(|| {
                    // Its only ability; a cost other than {T} is offered as an ability.
                    vec![PaymentStep::ActivateAbility {
                        source: pet,
                        ability_index: 0,
                    }]
                })
            },
            |_, options| options[0],
        );
        (engine, asked, bears, pet)
    };
    let (engine, asked, bears, pet) = play();
    assert_eq!(
        asked,
        [Asked::Pay(P0), Asked::Pay(P0), Asked::Sacrifice(P0)],
        "the Bears are paid for with the Pet, and the Pet is still owed for"
    );
    assert_eq!(zone(&engine, pet), Some(Zone::Graveyard));
    assert_eq!(zone(&engine, bears), Some(Zone::Graveyard));

    let (engine, before, bears, _) = as_before(play);
    assert_eq!(before, [Asked::Pay(P0)]);
    assert_eq!(
        zone(&engine, bears),
        Some(Zone::Battlefield),
        "the control: as it was, the Pet was skipped and the Bears survived"
    );
}

/// The sacrifices wait for the last choice: while seat 0 decides, the
/// Island seat 1 chose is still on the battlefield, and then both go at
/// once.
#[test]
fn the_sacrifices_wait_for_every_player() {
    let mut engine = resolving(
        2,
        &[&[index::GRIZZLY_BEARS, forest()], &[index::HILL_GIANT]],
    );
    let land = all_of(&engine, P0, forest())[0];
    let tapped = all_of(&engine, P1, island())[0];
    engine.apply(P1, PlayerAction::YesNo(false)).unwrap();
    engine
        .apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![tapped],
            },
        )
        .unwrap();
    assert!(
        matches!(engine.pending(), Pending::YesNo { player, .. } if *player == P0),
        "seat 0 is asked next: {:?}",
        engine.pending()
    );
    assert_eq!(
        zone(&engine, tapped),
        Some(Zone::Battlefield),
        "seat 1's Island waits for seat 0's choices"
    );
    answer(&mut engine, nobody_pays, |_, _| land);
    assert_eq!(zone(&engine, tapped), Some(Zone::Graveyard));
    assert_eq!(zone(&engine, land), Some(Zone::Graveyard));
}
