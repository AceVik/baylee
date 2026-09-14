//! The faces a card may be cast as, asked of both probes at once.
//!
//! `casting::can_cast` decides whether a card goes into `LegalActions`;
//! `cast_options` builds the ways of paying for it once the card has been
//! pressed. They are two readings of one question, and every disagreement
//! between them is one of two defects: a card offered and then refused with
//! "no way to cast this spell", or a card never offered that the player
//! would have paid for. The first is the one a player cannot recover from —
//! the button they are told to press is the button they are punished for.
//!
//! `convoke_tests` holds the four that were about *price*. These two are
//! about *which face*, and the offer was reading the front one in both:
//!
//! - Disturb (CR 702.112) casts the card transformed, for the back's own
//!   disturb cost, and the wizard has always known it — its disturb branch
//!   returns the backs and nothing else. The offer probed the front's mana
//!   cost, so Mirrorhall Mimic (`{3}{U}` in front of a `{3}{U}{U}` disturb)
//!   was castable from the graveyard at four mana and then refused.
//! - An adventure (CR 715) is cast from the hand at its own printed cost,
//!   and the offer knew about no face but the first. Twining Twins is a
//!   `{2}{U}{U}` creature in front of a `{1}{W}` instant, so Swift Spiral
//!   was unreachable on exactly the boards it is for: the ones where the
//!   creature cannot be paid for.
//!
//! The third test is the bound on the second, and it is why the shared
//! reader asks whether the card is *on its adventure* rather than reading
//! the face alone. CR 715.3d lets the owner of a card exiled that way cast
//! the creature and not the adventure again, so a reader that asked
//! `castable_from_hand` and nothing else would offer the adventure out of
//! the exile its own resolution had just put the card in.

use super::testkit::{Duel, RegistryLookup, card_index, keep_mulligans, pass_until};
use super::*;
use baylee_core::ids::{CardIndex, ObjectId};

fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}
fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}
/// `{1}{W}` 1/1 — a body for Swift Spiral to point at.
fn ondu_cleric() -> CardIndex {
    card_index("f4232466-dd6a-49bf-be6c-95905c3ded17")
}
/// `{2}{U}{U}` 4/4 in front of Swift Spiral, a `{1}{W}` instant adventure.
fn twining_twins() -> CardIndex {
    card_index("105aea98-8eb9-4fb2-a0cb-7c7513317c5b")
}
/// `{3}{U}` clone in front of Ghastly Mimicry, disturb `{3}{U}{U}`.
fn mirrorhall_mimic() -> CardIndex {
    card_index("5768fe50-a134-492c-a725-5ed02610c39f")
}
fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}
/// `{B}{B}` "destroy target creature", overload `{2}{W}{W}` — the one card
/// in the pool whose two modes differ in *both* price and target line.
fn damn() -> CardIndex {
    card_index("b01d61cc-9844-4191-86a0-f2db6d42d6e5")
}
/// The quietest creature there is, here only as something to destroy.
fn llanowar_elves() -> CardIndex {
    card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3")
}

/// Taps everything `seat` can tap for mana right now.
#[track_caller]
fn tap_all_mana(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
}

/// Taps `n` of the mana sources `seat` still has, in the engine's own order.
#[track_caller]
fn tap_some_mana(engine: &mut Engine<RegistryLookup>, seat: PlayerId, n: usize) {
    for _ in 0..n {
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        let source = *legal
            .mana_abilities
            .first()
            .expect("an untapped mana source");
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
}

/// Whether the engine is offering `card` as castable right now.
#[track_caller]
/// Walks to `seat`'s *next* first main phase.
///
/// The turn a cast happened in is already in its first main phase and is
/// tapped out, so a predicate naming only the phase answers "we are there"
/// and the test then measures this turn's board with last turn's mana.
fn next_own_main(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    let spent = engine.state().turn.number;
    pass_until(engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain)
            && e.state().turn.active == seat
            && e.state().turn.number > spent
    });
}

fn is_offered(engine: &Engine<RegistryLookup>, card: ObjectId) -> bool {
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    legal.castable.contains(&card)
}

/// The object in `zone` that was printed as `card`.
#[track_caller]
fn find(
    engine: &Engine<RegistryLookup>,
    zone: crate::zone::ZoneLocation,
    card: CardIndex,
) -> Option<ObjectId> {
    engine.state().zones.list(zone).iter().copied().find(|id| {
        engine
            .state()
            .object(*id)
            .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
    })
}

/// A disturb cast is priced by the back face, so the offer must be too.
///
/// Four mana covers Mirrorhall Mimic's printed `{3}{U}` and not Ghastly
/// Mimicry's `{3}{U}{U}` disturb, and the front cost is not a way of casting
/// this card from a graveyard at all: `cast_options`' disturb branch returns
/// the disturb backs and nothing else. So the card sat in `legal.castable`
/// with no option behind it, and pressing it answered "no way to cast this
/// spell" — the offer contradicting itself in the one direction a player
/// cannot do anything about.
#[test]
fn a_disturb_cast_is_offered_at_the_back_face_s_cost_and_not_the_front_s() {
    let mut engine = Duel::new(31, island())
        .hand(0, &[mirrorhall_mimic()])
        .battlefield(
            0,
            &[island(), island(), island(), island(), island(), plains()],
        )
        .start();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain) && e.state().turn.active == p0
    });
    // Into the graveyard by hand: how the card got there is not what this
    // test is about, and a spell that put it there would put its own rules
    // text between the premise and the assertion.
    let card = find(
        &engine,
        crate::zone::ZoneLocation::Hand(p0),
        mirrorhall_mimic(),
    )
    .expect("the Mimic is in hand");
    engine
        .dev_state_mut(p0)
        .expect("the test harness grants dev commands")
        .move_object(
            card,
            crate::zone::ZoneLocation::Graveyard(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::DevCommand,
        )
        .expect("a card moves to the graveyard");
    // Four mana: the front's price, one short of the disturb's.
    tap_some_mana(&mut engine, p0, 4);
    assert!(
        !is_offered(&engine, card),
        "a disturb cast was offered for the front face's mana cost"
    );
    // The fifth makes it a real offer, and the wizard has to agree.
    tap_some_mana(&mut engine, p0, 1);
    assert!(
        is_offered(&engine, card),
        "the disturb cost is payable and the card was not offered"
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("the offer is honoured");
    let stack = engine.state().zones.list(crate::zone::ZoneLocation::Stack);
    let spell = stack.last().copied().expect("a spell on the stack");
    let obj = engine.state().object(spell).unwrap();
    assert_eq!(obj.face_index, 1, "a disturb cast is the transformed back");
    assert_eq!(
        engine.state().names.get(obj.characteristics().name),
        "Ghastly Mimicry"
    );
}

/// The adventure is a way of casting the card, on boards the creature is not.
///
/// Two Plains pay Swift Spiral's `{1}{W}` and come nowhere near Twining
/// Twins' `{2}{U}{U}`, which is the whole point of printing an adventure on
/// a creature — and the offer, reading the front face alone, refused the
/// card on precisely those boards.
#[test]
fn an_adventure_is_offered_when_only_the_adventure_can_be_paid_for() {
    let mut engine = Duel::new(37, island())
        .hand(0, &[twining_twins()])
        .battlefield(0, &[plains(), plains(), ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain) && e.state().turn.active == p0
    });
    let card = find(
        &engine,
        crate::zone::ZoneLocation::Hand(p0),
        twining_twins(),
    )
    .expect("in hand");
    tap_all_mana(&mut engine, p0);
    assert!(
        is_offered(&engine, card),
        "the adventure is payable and the card was not offered"
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("the offer is honoured");
    // One way to cast it, so the wizard asked no mode question and went
    // straight to Swift Spiral's target.
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected the adventure's target choice, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, p0);
    let cleric = find(
        &engine,
        crate::zone::ZoneLocation::Battlefield,
        ondu_cleric(),
    )
    .expect("a creature to point at");
    assert!(options.contains(&cleric));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![cleric],
            },
        )
        .unwrap();
    let stack = engine.state().zones.list(crate::zone::ZoneLocation::Stack);
    let spell = stack.last().copied().expect("a spell on the stack");
    let obj = engine.state().object(spell).unwrap();
    assert_eq!(obj.face_index, 1, "the adventure is the back face");
    assert_eq!(
        engine.state().names.get(obj.characteristics().name),
        "Swift Spiral"
    );
}

/// From exile, the creature is the only thing CR 715 allows.
///
/// Swift Spiral resolving exiles the card on an adventure, and what may be
/// cast out of that exile is Twining Twins. Offering the adventure again
/// would be an infinite one: cast Swift Spiral, exile, cast Swift Spiral.
/// Nothing on the *face* says so — `castable_from_hand` is true on it, which
/// is what makes it castable in the test above — so the shared reader asks
/// for the `Adventure` rider the resolution left on the card, and this is the
/// assertion that keeps it doing so.
#[test]
fn the_adventure_is_not_offered_again_from_the_exile_it_was_cast_into() {
    let mut engine = Duel::new(41, island())
        .hand(0, &[twining_twins()])
        .battlefield(0, &[plains(), plains(), island(), island(), ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain) && e.state().turn.active == p0
    });
    let card = find(
        &engine,
        crate::zone::ZoneLocation::Hand(p0),
        twining_twins(),
    )
    .expect("in hand");
    // Exactly Swift Spiral's `{1}{W}`, off the two Plains, and every point of
    // it spent on the cast, so nothing floats into the question the next turn
    // asks — this test is about which faces are offered at WW and at WWUU, and
    // leftover mana would answer it with the wrong board.
    tap_some_mana(&mut engine, p0, 2);
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("the adventure is payable, so the card is castable");
    let cleric = find(
        &engine,
        crate::zone::ZoneLocation::Battlefield,
        ondu_cleric(),
    )
    .expect("a creature to point at");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![cleric],
            },
        )
        .unwrap();
    // Let it resolve: the card goes on its adventure.
    pass_until(&mut engine, |e| {
        find(e, crate::zone::ZoneLocation::Exile(p0), twining_twins()).is_some()
    });
    let exiled = find(
        &engine,
        crate::zone::ZoneLocation::Exile(p0),
        twining_twins(),
    )
    .expect("on an adventure in exile");
    // p0's *next* turn, for the lands: the exile happened in this one.
    next_own_main(&mut engine, p0);
    // Two mana, white: Swift Spiral's price exactly, and it must buy nothing.
    tap_some_mana(&mut engine, p0, 2);
    assert!(
        !is_offered(&engine, exiled),
        "the adventure was offered again out of the exile it was cast into"
    );
    // Four with two blue is the creature's price, which is what CR 715 does
    // allow — so the test cannot pass by the card being unreachable.
    tap_some_mana(&mut engine, p0, 2);
    assert!(
        is_offered(&engine, exiled),
        "the creature is payable from exile and was not offered"
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card: exiled })
        .expect("the offer is honoured");
    let stack = engine.state().zones.list(crate::zone::ZoneLocation::Stack);
    let spell = stack.last().copied().expect("a spell on the stack");
    let obj = engine.state().object(spell).unwrap();
    assert_eq!(obj.face_index, 0, "from exile it is the creature");
    assert_eq!(
        engine.state().names.get(obj.characteristics().name),
        "Twining Twins"
    );
    // And the other side of the same gate. Nothing ever takes the `Adventure`
    // rider off again — `move_object` clears a copy's characteristics
    // (CR 400.7) and leaves the rider list alone — so a Twining Twins that
    // was cast off its adventure and is later bounced arrives in the hand
    // still wearing it. CR 715.3d refuses the adventure only "this way", out
    // of the exile the adventure itself made, so from the hand Swift Spiral
    // is a way of casting the card once more. A reader that asked the rider
    // and not the exile would refuse it, and two white mana is the price
    // that says which of the two happened: the creature is `{2}{U}{U}` and
    // is not payable here, so the card is offered at all only as its
    // adventure.
    pass_until(&mut engine, |e| {
        find(e, crate::zone::ZoneLocation::Battlefield, twining_twins()).is_some()
    });
    let twins = find(
        &engine,
        crate::zone::ZoneLocation::Battlefield,
        twining_twins(),
    )
    .expect("the creature resolved");
    engine
        .dev_state_mut(p0)
        .expect("the test harness grants dev commands")
        .move_object(
            twins,
            crate::zone::ZoneLocation::Hand(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::DevCommand,
        )
        .expect("a permanent bounces to its owner's hand");
    assert!(
        engine
            .state()
            .object(twins)
            .is_some_and(|o| o.riders.contains(&crate::object::Rider::Adventure)),
        "the premise of the assertion below is that the rider survives the exile"
    );
    next_own_main(&mut engine, p0);
    tap_some_mana(&mut engine, p0, 2);
    assert!(
        is_offered(&engine, twins),
        "a stale adventure rider kept Swift Spiral from being cast out of the hand"
    );
}

/// A mode's price and a mode's target line belong to the **same** mode.
///
/// Damn is `{B}{B}` "destroy target creature" and an overload at
/// `{2}{W}{W}` that needs no target at all. On three Swamps against an empty
/// board both questions were asked, both answered yes, and each about the
/// other's mode: `can_cast` found the printed `{B}{B}` payable and returned
/// early, and `has_a_legal_target` found the overload pointable because a
/// requirement of `None` is reachable everywhere. So the card lit up in the
/// hand and every press came back "illegal action for your seat", which is
/// the offer contradicting itself in the direction a player cannot recover
/// from — the button they are told to press is the button they are punished
/// for.
///
/// `cast_options` was already right, and had been since Cyclonic Rift: it
/// intersects the two per mode. The offer now asks the same intersection,
/// which is the whole of the fix.
#[test]
fn a_modal_spell_is_offered_only_where_one_mode_is_paid_for_and_pointed() {
    let p0 = PlayerId::new(0);

    // Three Swamps, nothing to destroy. `{B}{B}` buys a mode with no target
    // and `{2}{W}{W}` is out of reach, so there is no way to cast the card.
    let mut engine = Duel::new(41, swamp())
        .hand(0, &[damn()])
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain) && e.state().turn.active == p0
    });
    let card = find(&engine, crate::zone::ZoneLocation::Hand(p0), damn()).expect("Damn is in hand");
    tap_all_mana(&mut engine, p0);
    assert!(
        !is_offered(&engine, card),
        "Damn was offered with neither of its modes available"
    );
    // And the two halves agree about it, which is the point: an offer this
    // test could not see is worth nothing if pressing the card works anyway.
    assert!(
        engine.apply(p0, PlayerAction::CastSpell { card }).is_err(),
        "the card was refused by the offer and accepted by the wizard"
    );

    // The same board with something to destroy: the `{B}{B}` mode is now
    // both payable and pointable, so the card is castable and stays so.
    let mut engine = Duel::new(41, swamp())
        .hand(0, &[damn()])
        .battlefield(0, &[swamp(), swamp(), swamp(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain) && e.state().turn.active == p0
    });
    let card = find(&engine, crate::zone::ZoneLocation::Hand(p0), damn()).expect("Damn is in hand");
    tap_all_mana(&mut engine, p0);
    assert!(
        is_offered(&engine, card),
        "a creature on the board makes Damn's printed mode a real cast"
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("the offer is honoured");

    // Four Plains and an empty board: the overload needs no target and is
    // paid for, so the card is castable for the mode whose price is *not*
    // the printed one.
    let mut engine = Duel::new(41, plains())
        .hand(0, &[damn()])
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain) && e.state().turn.active == p0
    });
    let card = find(&engine, crate::zone::ZoneLocation::Hand(p0), damn()).expect("Damn is in hand");
    tap_all_mana(&mut engine, p0);
    assert!(
        is_offered(&engine, card),
        "the overload is payable and needs no target, and the card was not offered"
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("the offer is honoured");
}
