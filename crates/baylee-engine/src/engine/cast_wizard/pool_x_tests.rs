//! The X question of a cast paid from the floating pool, and the payment
//! window an X the pool does not cover leads to.
//!
//! A card in `LegalActions::castable` is paid by the mana already floating
//! (its cost at X = 0 is covered). The caster announces X (CR 601.2b) before
//! paying, and may then still activate mana abilities (CR 601.2g), so:
//!
//! - while the seat has a mana source left to tap, the question is open
//!   (`u32::MAX`), and an X the pool does not cover opens the payment
//!   window after the cast's choices; a window closed short reverses the
//!   whole cast (CR 601.2h, 732.1) and says `CastCancelReason::Unpaid`;
//! - with nothing left to tap the pool is all that pays, and the question
//!   offers only the greatest X the pool pays for the whole cost.
//!
//! These tests play real cards through the casting wizard and read the bound
//! off `Pending::ChooseNumber`. Two earlier answers were both wrong: offering
//! the number of floating units ignored the fixed part of the cost, so an X
//! it left no mana for was taken and the cast reversed at payment with no
//! word why; and capping X at what floats, even with lands untapped, kept a
//! caster from announcing an X their own lands would have paid.

use super::*;
use crate::engine::PlayerAction;
use crate::engine::testkit::{
    Duel, RegistryLookup, SEED, card_index, in_hand, keep_mulligans, on_battlefield, on_stack,
    pass_until, pt, reach_main_phase, stack_is_empty, tap_mana_where,
};
use crate::event::CastCancelReason;
use crate::object::Status;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaColor;

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

/// Names seat 1's Elves as the target of the Prismatic Ending being cast.
#[track_caller]
fn name_the_elves(engine: &mut Engine<RegistryLookup>) {
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
}

/// Points the Prismatic Ending being cast at seat 1's Elves and checks the
/// cast went through: the spell stands on the stack and has left the hand.
#[track_caller]
fn aim_at_the_elves(engine: &mut Engine<RegistryLookup>) {
    name_the_elves(engine);
    assert!(
        on_stack(engine, prismatic_ending()).is_some(),
        "the cast completed: {:?}",
        engine.pending()
    );
    assert!(in_hand(engine, P0, prismatic_ending()).is_none());
}

/// Seat 0 has `floating + spare` Plains, has tapped `floating` of them, and
/// holds Prismatic Ending (`{X}{W}`); seat 1 has an Elves to point it at.
/// With no `spare` Plains the seat has nothing left to tap.
fn ending_with_white_floating(floating: usize, spare: usize) -> Engine<RegistryLookup> {
    let mut engine = main_phase(
        &vec![plains(); floating + spare],
        &[prismatic_ending()],
        &[llanowar_elves()],
    );
    float(&mut engine, plains(), floating);
    assert_offered(&engine, prismatic_ending(), true);
    engine
}

/// Seat 0's untapped Plains.
fn untapped_plains(engine: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine.state().object(*id).is_some_and(|o| {
                o.controller == P0
                    && !o.status.contains(Status::TAPPED)
                    && o.card.is_some_and(|c| c.index == plains())
            })
        })
        .collect()
}

/// Checks the CR 601.2g payment window is open for seat 0 with the spell not
/// yet cast: the window is the one question, and it offers mana abilities
/// and nothing else.
#[track_caller]
fn assert_window_open(engine: &Engine<RegistryLookup>) {
    assert!(
        matches!(engine.payment_window(), Some((seat, _)) if seat == P0),
        "the payment window is open for the caster: {:?}",
        engine.pending()
    );
    let Pending::Priority { player, legal } = engine.pending() else {
        panic!("the window is a priority offer, got {:?}", engine.pending())
    };
    assert_eq!(*player, P0);
    assert!(
        legal.castable.is_empty() && legal.payable.is_empty() && legal.lands.is_empty(),
        "nothing but mana abilities is offered in the window: {legal:?}"
    );
    assert!(legal.has_mana_source(), "the window offers a mana source");
    assert!(
        on_stack(engine, prismatic_ending()).is_none(),
        "the spell is not cast until the window is paid"
    );
}

/// The journal's `CastCancelled` reasons for `card`, oldest first.
fn cancel_reasons(engine: &Engine<RegistryLookup>, card: ObjectId) -> Vec<CastCancelReason> {
    engine
        .journal()
        .entries()
        .iter()
        .filter_map(|e| match e.event {
            GameEvent::CastCancelled {
                player,
                card: c,
                reason,
            } if player == P0 && c == card => Some(reason),
            _ => None,
        })
        .collect()
}

/// One {W} floating off the only Plains, so nothing is left to tap: the pool
/// is all that pays, and `{X}{W}` leaves nothing for X, so the question offers
/// X = 0 and nothing else. The question that offered the pool's size, 1, let
/// X = 1 through and found no mana for the {W} on top of it at payment, so the
/// cast was reversed; it is refused. A guard: it passes both before and after
/// the change that left X open while the seat has a source to tap (CR 601.2g,
/// `a_plains_left_to_tap_leaves_x_open_and_the_window_pays_for_it`), because
/// the seat has none here.
#[test]
fn one_white_floating_pays_for_no_x_under_x_and_white() {
    let mut engine = ending_with_white_floating(1, 0);
    let (min, max) = x_question(&mut engine, prismatic_ending());
    assert_eq!((min, max), (0, 0), "the pool pays the {{W}} and no {{X}}");
    assert_refuses_above(&mut engine, max);

    choose_x(&mut engine, 0);
    aim_at_the_elves(&mut engine);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}

/// Two {W} floating off the only two Plains, nothing left to tap: `{1}{W}` is
/// the most they pay, so X tops out at 1. The question that offered the pool's
/// size offered 2.
#[test]
fn two_white_floating_pay_for_an_x_of_one() {
    let mut engine = ending_with_white_floating(2, 0);
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
/// {W} floating off the only three Plains that is 0, 1 and 2, and what X does
/// not spend stays in the pool. The question that offered the pool's size
/// offered 3 as well, which did not complete.
#[test]
fn every_x_the_pool_offers_completes_the_cast() {
    for x in 0..=2_u32 {
        let mut engine = ending_with_white_floating(3, 0);
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

/// `{X}{X}` takes two mana for each point of X, so with `n` mana floating and
/// nothing left to tap the greatest X is `n / 2`. The question that offered
/// `n` offered five for the five Forests below, where four mana is the most a
/// pair of X pays. Each board is exactly the Forests tapped, so the exact
/// bound applies.
#[test]
fn a_double_x_cost_halves_what_the_pool_offers() {
    for floating in 1..=5_usize {
        let mut engine = main_phase(&vec![forest(); floating], &[walking_ballista()], &[]);
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
/// choices, so no pool bounds X and the question stays unbounded. The exact
/// bound is for a cast off the pool whose seat has nothing left to tap, and
/// not for this one.
#[test]
fn a_cast_paid_in_a_window_after_its_choices_keeps_an_unbounded_x() {
    let mut engine = main_phase(&[plains(); 3], &[prismatic_ending()], &[llanowar_elves()]);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_offered(&engine, prismatic_ending(), false);

    let (min, max) = x_question(&mut engine, prismatic_ending());
    assert_eq!((min, max), (0, u32::MAX), "the window pays, not the pool");

    choose_x(&mut engine, 2);
    name_the_elves(&mut engine);
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

/// The rules bound: the caster announces X (CR 601.2b) and may then make the
/// mana for it (CR 601.2g). One {W} floating and three Plains untapped, so
/// `{X}{W}` is asked with an open X, not the X = 0 the pool alone pays. X = 2
/// is taken, the target is chosen, and the cast is not yet paid: the window
/// opens. Two Plains tapped in it and a pass pay the {2}{W} with the floating
/// {W}, and the spell stands on the stack with X = 2 and an empty pool. The
/// version that capped X at what floats gave 0 here and never let this
/// answer through.
#[test]
fn a_plains_left_to_tap_leaves_x_open_and_the_window_pays_for_it() {
    let mut engine = ending_with_white_floating(1, 3);
    let (min, max) = x_question(&mut engine, prismatic_ending());
    assert_eq!(
        (min, max),
        (0, u32::MAX),
        "untapped Plains can still make the mana for X"
    );

    choose_x(&mut engine, 2);
    name_the_elves(&mut engine);
    assert_window_open(&engine);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{W}} floated before the cast waits in the pool"
    );

    let spare = untapped_plains(&engine);
    assert_eq!(spare.len(), 3, "the window opens with the Plains untapped");
    let paying = &spare[..2];
    assert_eq!(
        tap_mana_where(&mut engine, P0, |id| paying.contains(&id)),
        2
    );
    engine
        .apply(P0, PlayerAction::PassPriority)
        .expect("three mana pay {2}{W}");

    let spell = on_stack(&engine, prismatic_ending()).expect("the spell was cast");
    assert_eq!(engine.state().object(spell).map(|o| o.x_value), Some(2));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the floating {{W}} and the two from the window paid the whole cost"
    );
    assert!(engine.payment_window().is_none(), "the window closed");
    assert_eq!(
        untapped_plains(&engine).len(),
        1,
        "the Plains not needed stays untapped"
    );
    assert!(
        cancel_reasons(&engine, spell).is_empty(),
        "nothing was undone"
    );
}

/// The same cast with the window closed short. X = 2 costs `{2}{W}`, the
/// pool holds one {W} and only one Plains is tapped in the window, so three
/// mana are not there: partial payment is not allowed (CR 601.2h) and the
/// whole cast is reversed (CR 732.1). The card is back in hand, the Plains
/// tapped in the window are untapped again, the {W} floated before the cast
/// still floats, `CastCancelled` is journaled with
/// `CastCancelReason::Unpaid`, and the caster holds priority with no window
/// open. The cast can then be begun again. Without the window the question
/// could not be answered at X = 2 in the first place.
#[test]
fn a_window_closed_short_reverses_the_cast_and_says_unpaid() {
    let mut engine = ending_with_white_floating(1, 3);
    let before = untapped_plains(&engine);
    let (_, max) = x_question(&mut engine, prismatic_ending());
    assert_eq!(max, u32::MAX);
    choose_x(&mut engine, 2);
    name_the_elves(&mut engine);
    assert_window_open(&engine);

    let paying = &before[..1];
    assert_eq!(
        tap_mana_where(&mut engine, P0, |id| paying.contains(&id)),
        1
    );
    engine
        .apply(P0, PlayerAction::PassPriority)
        .expect("a window may be closed short; the cast is what fails");

    let card = in_hand(&engine, P0, prismatic_ending()).expect("the card is back in hand");
    assert!(on_stack(&engine, prismatic_ending()).is_none());
    assert_eq!(
        untapped_plains(&engine),
        before,
        "the Plains tapped in the window are untapped again"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        (pool.total(), pool.available(ManaColor::White)),
        (1, 1),
        "the {{W}} floated before the cast is still floating"
    );
    assert_eq!(
        cancel_reasons(&engine, card),
        vec![CastCancelReason::Unpaid],
        "the journal says the window closed short"
    );
    assert!(engine.payment_window().is_none(), "no window is left open");
    let Pending::Priority { player, legal } = engine.pending() else {
        panic!("the caster has priority again: {:?}", engine.pending())
    };
    assert_eq!(*player, P0);
    assert!(legal.castable.contains(&card), "the card can be cast again");

    let (_, max) = x_question(&mut engine, prismatic_ending());
    assert_eq!(max, u32::MAX, "and the question is open again");
}

/// The bound opens with the first source left to tap and not before. With
/// one {W} floating, a seat whose Plains are all tapped is offered X = 0 only;
/// the same seat with one more Plains untapped, or with an untapped Elves that
/// makes mana, is offered an open X. The no-source half passes both before and
/// after the change; the other two did not before it.
#[test]
fn the_x_bound_is_open_exactly_while_a_mana_source_is_left() {
    let mut engine = ending_with_white_floating(1, 0);
    let (_, max) = x_question(&mut engine, prismatic_ending());
    assert_eq!(max, 0, "nothing is left to tap");

    let mut engine = ending_with_white_floating(1, 1);
    let (_, max) = x_question(&mut engine, prismatic_ending());
    assert_eq!(max, u32::MAX, "an untapped Plains is a source");

    let mut engine = main_phase(
        &[plains(), llanowar_elves()],
        &[prismatic_ending()],
        &[llanowar_elves()],
    );
    float(&mut engine, plains(), 1);
    assert_offered(&engine, prismatic_ending(), true);
    let (_, max) = x_question(&mut engine, prismatic_ending());
    assert_eq!(max, u32::MAX, "an untapped mana creature is a source");
}
