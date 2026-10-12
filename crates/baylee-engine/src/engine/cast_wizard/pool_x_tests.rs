//! The X question of a cast paid from the floating pool offers only an X the
//! pool pays for the whole cost.
//!
//! A card in `LegalActions::castable` is paid by the mana already floating
//! (its cost at X = 0 is covered), and when nothing besides the pool pays
//! any of it the greatest X is known before X is asked. These tests play real
//! cards through the casting wizard and read that number off
//! `Pending::ChooseNumber`.
//!
//! Before the fix the question offered the number of floating units and
//! ignored the fixed part of the cost, so an X the fixed part left no mana
//! for was taken, and the cast was then reversed at payment (CR 601.2h,
//! 732.1) with the caster back at priority and no word why.

use super::*;
use crate::engine::PlayerAction;
use crate::engine::testkit::{
    Duel, RegistryLookup, SEED, card_index, in_hand, keep_mulligans, on_battlefield, on_stack,
    pass_until, pt, reach_main_phase, stack_is_empty, tap_mana_where,
};
use crate::object::Status;
use baylee_core::ids::CardIndex;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

fn llanowar_elves() -> CardIndex {
    card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3")
}

/// `{X}{W}` sorcery.
fn prismatic_ending() -> CardIndex {
    card_index("2cb98ca9-d7bb-416b-a17e-ee5f8e4d78f2")
}

/// `{X}{X}` artifact creature.
fn walking_ballista() -> CardIndex {
    card_index("4b515bb0-f275-4400-8032-3173b799ab40")
}

/// Seat 0 in its first main phase with `board` in play, `hand` in hand and
/// `opposing` on seat 1's side of the table. Nothing is tapped.
fn main_phase(
    board: &[CardIndex],
    hand: &[CardIndex],
    opposing: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, board)
        .battlefield(1, opposing)
        .hand(0, hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// Taps `n` of seat 0's untapped `land`s for mana and leaves every other
/// source standing, so the pool holds exactly what those `n` made.
#[track_caller]
fn float(engine: &mut Engine<RegistryLookup>, land: CardIndex, n: usize) {
    let chosen: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine.state().object(*id).is_some_and(|o| {
                o.controller == P0
                    && !o.status.contains(Status::TAPPED)
                    && o.card.is_some_and(|c| c.index == land)
            })
        })
        .take(n)
        .collect();
    assert_eq!(chosen.len(), n, "the board holds {n} untapped lands to tap");
    let taken = tap_mana_where(engine, P0, |id| chosen.contains(&id));
    assert_eq!(taken, n, "each chosen land made its mana");
    assert_eq!(
        usize::try_from(engine.state().players[0].mana_pool.total()),
        Ok(n),
        "the pool holds exactly what the tapped lands made"
    );
}

/// Which of the offer's two lists names `card`, asserted so a test says
/// which cast it is playing: `castable` (mana already floating) or
/// `payable` (made in a window after the choices). The lists never overlap.
#[track_caller]
fn assert_offered(engine: &Engine<RegistryLookup>, card: CardIndex, castable: bool) {
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let spell = in_hand(engine, P0, card).expect("the spell is in hand");
    assert_eq!(
        (
            legal.castable.contains(&spell),
            legal.payable.contains(&spell)
        ),
        (castable, !castable),
        "the offer names the spell under `castable` or `payable`, never both"
    );
}

/// Casts `card` from seat 0's hand and returns the bounds of the X question
/// that comes first.
#[track_caller]
fn x_question(engine: &mut Engine<RegistryLookup>, card: CardIndex) -> (u32, u32) {
    let spell = in_hand(engine, P0, card).expect("the spell is in hand");
    engine
        .apply(P0, PlayerAction::CastSpell { card: spell })
        .expect("the offer listed the cast");
    let Pending::ChooseNumber {
        player,
        min,
        max,
        reason: NumberPrompt::X,
    } = engine.pending().clone()
    else {
        panic!("X is asked first, got {:?}", engine.pending())
    };
    assert_eq!(player, P0, "the caster names X");
    (min, max)
}

/// Answers the open X question with `x`, which the question offered.
#[track_caller]
fn choose_x(engine: &mut Engine<RegistryLookup>, x: u32) {
    engine
        .apply(P0, PlayerAction::ChooseNumber(x))
        .expect("an X the question offered");
}

/// Answers `x` above the question's `max` and checks the engine refuses it
/// and keeps asking the same question.
#[track_caller]
fn assert_refuses_above(engine: &mut Engine<RegistryLookup>, max: u32) {
    assert!(
        engine
            .apply(P0, PlayerAction::ChooseNumber(max + 1))
            .is_err(),
        "X = {} is more than the pool pays for",
        max + 1
    );
    assert!(
        matches!(
            engine.pending(),
            Pending::ChooseNumber { max: asked, reason: NumberPrompt::X, .. } if *asked == max
        ),
        "the refusal leaves the X question where it was: {:?}",
        engine.pending()
    );
}

/// Points the Prismatic Ending being cast at seat 1's Elves and checks the
/// cast went through: the spell stands on the stack and has left the hand.
#[track_caller]
fn aim_at_the_elves(engine: &mut Engine<RegistryLookup>) {
    let elves = on_battlefield(engine, P1, llanowar_elves()).expect("seat 1's Elves");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the target is chosen next, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&elves),
        "the Elves are a nonland permanent"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![elves],
                players: vec![],
            },
        )
        .expect("a target on the menu");
    assert!(
        on_stack(engine, prismatic_ending()).is_some(),
        "the cast completed: {:?}",
        engine.pending()
    );
    assert!(in_hand(engine, P0, prismatic_ending()).is_none());
}

/// Seat 0 has four Plains, has tapped `floating` of them, and holds
/// Prismatic Ending (`{X}{W}`); seat 1 has an Elves to point it at.
fn ending_with_white_floating(floating: usize) -> Engine<RegistryLookup> {
    let mut engine = main_phase(&[plains(); 4], &[prismatic_ending()], &[llanowar_elves()]);
    float(&mut engine, plains(), floating);
    assert_offered(&engine, prismatic_ending(), true);
    engine
}

/// One {W} floating and three Plains still untapped: `{X}{W}` leaves
/// nothing for X, so the question offers X = 0 and nothing else. Casting
/// from `castable` is paid by the pool alone, so the untapped Plains do not
/// count. The old question offered the pool's size, 1; choosing it found no
/// mana for the {W} on top of X = 1 at payment, and the cast was reversed. It
/// is refused now.
#[test]
fn one_white_floating_pays_for_no_x_under_x_and_white() {
    let mut engine = ending_with_white_floating(1);
    let (min, max) = x_question(&mut engine, prismatic_ending());
    assert_eq!((min, max), (0, 0), "the pool pays the {{W}} and no {{X}}");
    assert_refuses_above(&mut engine, max);

    choose_x(&mut engine, 0);
    aim_at_the_elves(&mut engine);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}

/// Two {W} floating: `{1}{W}` is the most they pay, so X tops out at 1. The
/// old question offered the pool's size, 2.
#[test]
fn two_white_floating_pay_for_an_x_of_one() {
    let mut engine = ending_with_white_floating(2);
    let (min, max) = x_question(&mut engine, prismatic_ending());
    assert_eq!((min, max), (0, 1), "X plus the fixed {{W}} is the pool");
    assert_refuses_above(&mut engine, max);

    choose_x(&mut engine, max);
    aim_at_the_elves(&mut engine);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the chosen X and the {{W}} took the whole pool"
    );
}

/// Every X the question offers is one the cast completes with: with three
/// {W} floating that is 0, 1 and 2, and what X does not spend stays in the
/// pool. The old question offered 3 as well, which did not.
#[test]
fn every_x_the_pool_offers_completes_the_cast() {
    for x in 0..=2_u32 {
        let mut engine = ending_with_white_floating(3);
        let (_, max) = x_question(&mut engine, prismatic_ending());
        assert_eq!(max, 2, "three floating pay {{X}}{{W}} for X up to 2");
        assert_refuses_above(&mut engine, max);

        choose_x(&mut engine, x);
        aim_at_the_elves(&mut engine);
        assert_eq!(
            engine.state().players[0].mana_pool.total(),
            u64::from(2 - x),
            "X = {x} spent {x} and the {{W}}, and left the rest"
        );
    }
}

/// `{X}{X}` takes two mana for each point of X, so with `n` mana floating
/// the greatest X is `n / 2`. The old question offered `n`: five for the
/// five Forests below, where four mana is the most a pair of X pays.
#[test]
fn a_double_x_cost_halves_what_the_pool_offers() {
    for floating in 1..=5_usize {
        let mut engine = main_phase(&[forest(); 5], &[walking_ballista()], &[]);
        float(&mut engine, forest(), floating);
        assert_offered(&engine, walking_ballista(), true);
        let (min, max) = x_question(&mut engine, walking_ballista());
        assert_eq!(
            (min, max),
            (0, u32::try_from(floating / 2).expect("a small count")),
            "{floating} floating pay {{X}}{{X}} for X up to {}",
            floating / 2
        );
    }

    let mut engine = main_phase(&[forest(); 5], &[walking_ballista()], &[]);
    float(&mut engine, forest(), 5);
    let (_, max) = x_question(&mut engine, walking_ballista());
    assert_eq!(max, 2);
    assert_refuses_above(&mut engine, max);
    choose_x(&mut engine, max);
    assert!(
        on_stack(&engine, walking_ballista()).is_some(),
        "the cast completed: {:?}",
        engine.pending()
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "X = 2 took four of the five"
    );
    pass_until(&mut engine, stack_is_empty);
    let ballista = on_battlefield(&engine, P0, walking_ballista()).expect("the Ballista resolved");
    assert_eq!(pt(&engine, ballista), (2, 2), "it entered with X = 2");
}

/// Regression guard for the other road. A card with nothing floating is in
/// `payable`, not `castable`: its mana is made in a window after the
/// choices, so no pool bounds X and the question stays unbounded. The pool
/// check applies to casts paid from the pool alone, and not to this one.
#[test]
fn a_cast_paid_in_a_window_after_its_choices_keeps_an_unbounded_x() {
    let mut engine = main_phase(&[plains(); 3], &[prismatic_ending()], &[llanowar_elves()]);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_offered(&engine, prismatic_ending(), false);

    let (min, max) = x_question(&mut engine, prismatic_ending());
    assert_eq!((min, max), (0, u32::MAX), "the window pays, not the pool");

    choose_x(&mut engine, 2);
    let elves = on_battlefield(&engine, P1, llanowar_elves()).expect("seat 1's Elves");
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![elves],
                players: vec![],
            },
        )
        .expect("a target on the menu");
    assert!(
        matches!(engine.payment_window(), Some((seat, _)) if seat == P0),
        "the payment window opens after the choices: {:?}",
        engine.pending()
    );
    tap_mana_where(&mut engine, P0, |_| true);
    engine
        .apply(P0, PlayerAction::PassPriority)
        .expect("three Plains pay {2}{W}");
    assert!(
        on_stack(&engine, prismatic_ending()).is_some(),
        "the spell was cast: {:?}",
        engine.pending()
    );
}
