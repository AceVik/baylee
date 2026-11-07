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
//! - Disturb (CR 702.146) casts the card transformed, for the back's own
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

use super::testkit::{
    Duel, RegistryLookup, card_index, cast_from_hand, keep_mulligans, pass_until, reach_main_phase,
    stack_is_empty, tap_all_mana, tap_mana_where,
};
use super::*;
use crate::choice::CastModeKind;
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
/// `{W}` "Exile target creature."
fn swords_to_plowshares() -> CardIndex {
    card_index("b1544f21-7e98-461b-aed5-e748b0168c52")
}
/// The quietest creature there is, here only as something to destroy.
fn llanowar_elves() -> CardIndex {
    card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3")
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
        // The Elves are what Ghastly Mimicry, an Aura, has to enchant: an
        // Aura spell with nothing to target is not offered at all.
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                plains(),
                llanowar_elves(),
            ],
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
    let elves = find(
        &engine,
        crate::zone::ZoneLocation::Battlefield,
        llanowar_elves(),
    )
    .expect("the Elves are out");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elves],
                players: vec![],
            },
        )
        .expect("the Aura enchants the Elves");
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
    // And the other side of the same gate. CR 715.3d refuses the adventure
    // only "this way", out of the exile the adventure itself made, so a
    // Twining Twins that was cast off its adventure and is later bounced may
    // be cast as Swift Spiral from the hand once more. The `Adventure` rider
    // ends as the card leaves exile (`Rider::ends_as_it_leaves_exile`); it
    // was once kept, and a reader that asked the rider and not the exile
    // refused the adventure here. Two white mana is the price that says which
    // of the two happened: the creature is `{2}{U}{U}` and is not payable
    // here, so the card is offered at all only as its adventure.
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
        engine.state().object(twins).is_some_and(|o| !o
            .riders
            .iter()
            .any(|r| matches!(r, crate::object::Rider::Adventure(_)))),
        "the card left the exile it was on its adventure in, and the rider with it"
    );
    next_own_main(&mut engine, p0);
    tap_some_mana(&mut engine, p0, 2);
    assert!(
        is_offered(&engine, twins),
        "a stale adventure rider kept Swift Spiral from being cast out of the hand"
    );
}

/// The permission CR 715.3d gives lasts "for as long as that card remains
/// exiled", and no longer. Twining Twins goes on its adventure, is cast out
/// of that exile, and later Swords to Plowshares exiles the creature. The
/// card in exile now was exiled by Swords, and nothing lets anybody cast it:
/// not as the creature, and not on a board that pays for either face.
///
/// The adventure's mark rode along with the card through the stack and the
/// battlefield, and the offer asks only whether a marked card is in exile, so
/// the Swords' Twins was castable from exile every turn after (CR 400.7).
#[test]
#[allow(clippy::too_many_lines)] // an adventure, a cast from exile, Swords and a turn, told in order
fn a_card_cast_off_its_adventure_and_exiled_again_is_not_castable_from_exile() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(43, island())
        .hand(0, &[twining_twins(), swords_to_plowshares()])
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                island(),
                island(),
                island(),
                island(),
                ondu_cleric(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain) && e.state().turn.active == p0
    });
    let lands = |engine: &Engine<RegistryLookup>, land: CardIndex| -> Vec<ObjectId> {
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Battlefield)
            .iter()
            .copied()
            .filter(|id| {
                engine
                    .state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == land))
            })
            .collect()
    };
    let (plains, islands) = (lands(&engine, plains()), lands(&engine, island()));

    // Swift Spiral off two Plains, at the Cleric: the card goes on its
    // adventure.
    tap_mana_where(&mut engine, p0, |id| plains[..2].contains(&id));
    let card = find(
        &engine,
        crate::zone::ZoneLocation::Hand(p0),
        twining_twins(),
    )
    .expect("in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("the adventure is payable");
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
    pass_until(&mut engine, |e| {
        find(e, crate::zone::ZoneLocation::Exile(p0), twining_twins()).is_some()
    });

    // The creature, cast out of that exile off the four Islands.
    tap_mana_where(&mut engine, p0, |id| islands.contains(&id));
    assert!(is_offered(&engine, card), "CR 715.3d allows the creature");
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("the offer is honoured");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        find(
            &engine,
            crate::zone::ZoneLocation::Battlefield,
            twining_twins()
        ),
        Some(card),
        "the creature resolved"
    );

    // Swords to Plowshares off the last Plains exiles it again.
    tap_mana_where(&mut engine, p0, |id| id == plains[2]);
    let swords = find(
        &engine,
        crate::zone::ZoneLocation::Hand(p0),
        swords_to_plowshares(),
    )
    .expect("in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: swords })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![card],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(card).map(|o| o.zone),
        Some(crate::zone::Zone::Exile),
        "exiled by Swords"
    );

    // Seven lands, both faces paid for, and the card is offered as neither.
    next_own_main(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    assert!(
        !is_offered(&engine, card),
        "a card Swords exiled was castable from exile"
    );
}

/// An Adventure is cast at its own timing (CR 715.3a: only the alternative
/// characteristics are evaluated to see if it can be cast; CR 601.3e).
///
/// Swift Spiral is an instant on the back of Twining Twins, a creature
/// without flash. On the other seat's turn, with mana for both faces
/// floating, the card is offered and the only way to cast it is the
/// Adventure: the creature is not a spell that can be cast then, so the
/// wizard asks no face question and goes straight to Swift Spiral's target.
/// The offer used to read the front face's timing alone and refused the
/// whole card, so an instant Adventure was a sorcery.
#[test]
fn an_instant_adventure_is_cast_at_instant_speed_and_its_creature_is_not() {
    let mut engine = Duel::new(37, island())
        .hand(0, &[twining_twins()])
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                plains(),
                plains(),
                ondu_cleric(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain)
            && e.state().turn.active == p1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
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
        "Swift Spiral is an instant and was not offered on the other turn"
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("the offer is honoured");
    let Pending::ChooseTargets { player, .. } = engine.pending().clone() else {
        panic!(
            "the creature is no way to cast the card now, so no face is asked \
             about: got {:?}",
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
    assert_eq!(
        engine.state().object(spell).map(|o| o.face_index),
        Some(1),
        "the Adventure, on the other seat's turn"
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

/// `{W}{U}`: "As this creature enters, choose a nonland card name. Spells
/// with the chosen name can't be cast."
fn meddling_mage() -> CardIndex {
    card_index("a5d02339-9f6d-486e-87d8-5ec2274cf680")
}
/// `{1}{R}` Fire and `{1}{U}` Ice: two halves, each a name of its own.
fn fire_ice() -> CardIndex {
    card_index("ae92942b-919c-4ea9-b693-85fcef765d5a")
}
/// `{1}{U}{U}` flash creature in front of Petty Theft, a `{1}{U}` instant.
fn brazen_borrower() -> CardIndex {
    card_index("c7b044c3-3cfa-407e-bf20-2875e8e04b7b")
}
fn mountain() -> CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

/// Seat 0's next main phase with `spell` in hand and every land open as
/// floating mana, seat 1 holding a creature to point at. With `named`, a
/// Meddling Mage of seat 0's stands there having named that face.
fn table_with(spell: CardIndex, named: Option<(CardIndex, u8)>) -> Engine<RegistryLookup> {
    let me = PlayerId::new(0);
    let mut engine = Duel::new(1140, plains())
        .battlefield(0, &[plains(), island(), island(), mountain()])
        .hand(0, &[meddling_mage(), spell])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, me);
    if let Some((card, face)) = named {
        cast_from_hand(&mut engine, me, meddling_mage());
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseCardName { .. })
        });
        engine
            .apply(me, PlayerAction::ChooseCardName { card, face })
            .expect("a nonland face may be named");
    }
    next_own_main(&mut engine, me);
    tap_all_mana(&mut engine, me);
    engine
}

/// What the offer and the wizard both say of `spell` in seat 0's hand: the
/// card is offered exactly when there is a way to cast it, and these are the
/// ways.
#[track_caller]
fn ways_to_cast(engine: &Engine<RegistryLookup>, spell: CardIndex) -> Vec<CastModeKind> {
    let me = PlayerId::new(0);
    let card = find(engine, crate::zone::ZoneLocation::Hand(me), spell).expect("in hand");
    let ways: Vec<CastModeKind> = engine.cast_options(me, card, false).map_or_else(
        |_| Vec::new(),
        |options| options.iter().map(|o| o.kind).collect(),
    );
    assert_eq!(
        engine.compute_legal(me).castable.contains(&card),
        !ways.is_empty(),
        "the offer and the wizard disagree: {ways:?}"
    );
    ways
}

/// A lock of seat 1's on what seat 0 casts, as Llawan's line lands.
fn lock(engine: &mut Engine<RegistryLookup>, modifier: baylee_cards_dsl::Modifier) {
    let state = engine
        .dev_state_mut(PlayerId::new(0))
        .expect("the harness may set boards up");
    let timestamp = state.next_timestamp();
    state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: None,
        controller: PlayerId::new(1),
        origin: crate::effects::EffectOrigin::Resolution,
        layer: modifier.layer(),
        timestamp,
        duration: baylee_cards_dsl::Duration::Indefinitely,
        filter: crate::effects::EffectFilter::Dsl(&baylee_cards_dsl::Filter::This),
        modifier,
    });
}

/// A split card's halves are two names, and a spell cast as one half has
/// that half's alone (CR 709.3a, 709.3b). So naming Fire stops Fire and
/// leaves Ice, though the card in hand is Fire by its front: the lock is
/// asked of the face being cast, at the offer and at the cast. No object of
/// this game has been called Ice when the question is asked, which is the
/// case that a projection falling back on the card's own name gets wrong.
#[test]
fn naming_a_split_card_s_front_leaves_its_other_half_castable() {
    let me = PlayerId::new(0);
    let mut engine = table_with(fire_ice(), Some((fire_ice(), 0)));
    assert_eq!(
        engine.state().names.find("Ice"),
        None,
        "nothing has carried the name yet"
    );
    assert_eq!(ways_to_cast(&engine, fire_ice()), [CastModeKind::Face(1)]);

    let card = find(&engine, crate::zone::ZoneLocation::Hand(me), fire_ice()).unwrap();
    engine
        .apply(me, PlayerAction::CastSpell { card })
        .expect("Ice is another name");
    let elves = find(
        &engine,
        crate::zone::ZoneLocation::Battlefield,
        llanowar_elves(),
    )
    .unwrap();
    engine
        .apply(
            me,
            PlayerAction::ChooseTargets {
                objects: vec![elves],
                players: vec![],
            },
        )
        .expect("Ice taps a permanent");
    let spell = engine.state().object(card).expect("the card");
    assert_eq!(spell.zone, crate::zone::Zone::Stack);
    assert_eq!(
        engine.state().names.get(spell.characteristics().name),
        "Ice"
    );
}

/// And the other way about: naming Ice stops the half and not the card, so
/// the back face is not offered and Fire is.
#[test]
fn naming_a_split_card_s_back_half_stops_only_that_half() {
    let engine = table_with(fire_ice(), Some((fire_ice(), 1)));
    assert_eq!(ways_to_cast(&engine, fire_ice()), [CastModeKind::Normal]);
}

/// A name that is neither half stops neither, and with no lock at all the
/// card keeps both ways.
#[test]
fn a_name_on_another_card_leaves_both_halves() {
    let both = [CastModeKind::Normal, CastModeKind::Face(1)];
    let engine = table_with(fire_ice(), None);
    assert_eq!(ways_to_cast(&engine, fire_ice()), both);
    let engine = table_with(fire_ice(), Some((brazen_borrower(), 0)));
    assert_eq!(ways_to_cast(&engine, fire_ice()), both);
}

/// An adventurer has its alternative characteristics only as an Adventure
/// (CR 715.3a, 715.3b): Brazen Borrower named leaves Petty Theft, and Petty
/// Theft named leaves Brazen Borrower.
#[test]
fn a_name_reaches_one_side_of_an_adventurer() {
    let engine = table_with(brazen_borrower(), Some((brazen_borrower(), 0)));
    assert_eq!(
        ways_to_cast(&engine, brazen_borrower()),
        [CastModeKind::Face(1)]
    );
    let engine = table_with(brazen_borrower(), Some((brazen_borrower(), 1)));
    assert_eq!(
        ways_to_cast(&engine, brazen_borrower()),
        [CastModeKind::Normal]
    );
}

/// A lock on a type reads the face the same way a name does: "creature
/// spells" stops the creature and not its instant Adventure, "noncreature
/// spells" the reverse, and the two together leave nothing to offer.
#[test]
fn a_type_lock_is_asked_of_the_face_being_cast() {
    let creature =
        baylee_cards_dsl::Modifier::OpponentsCantCast(&baylee_cards_dsl::Filter::CREATURE);
    let noncreature =
        baylee_cards_dsl::Modifier::OpponentsCantCast(&baylee_cards_dsl::Filter::NONCREATURE);

    let mut engine = table_with(brazen_borrower(), None);
    lock(&mut engine, creature);
    assert_eq!(
        ways_to_cast(&engine, brazen_borrower()),
        [CastModeKind::Face(1)]
    );

    let mut engine = table_with(brazen_borrower(), None);
    lock(&mut engine, noncreature);
    assert_eq!(
        ways_to_cast(&engine, brazen_borrower()),
        [CastModeKind::Normal]
    );

    lock(&mut engine, creature);
    assert_eq!(ways_to_cast(&engine, brazen_borrower()), Vec::new());
}

/// A permission that waives the mana cost casts through its own list of ways,
/// and that list is held to the same lock: naming Fire leaves Ice alone there.
#[test]
fn a_free_permission_casts_only_the_face_a_lock_leaves() {
    let me = PlayerId::new(0);
    let mut engine = table_with(fire_ice(), Some((fire_ice(), 0)));
    let card = find(&engine, crate::zone::ZoneLocation::Hand(me), fire_ice()).unwrap();
    let state = engine
        .dev_state_mut(me)
        .expect("the harness may set boards up");
    let card = state
        .move_object(
            card,
            crate::zone::ZoneLocation::Exile(me),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    let version = state.object(card).expect("the card").version;
    state.play_permissions.push(crate::state::PlayPermission {
        player: me,
        card,
        version,
        free: true,
        mana_any_type: false,
        cast_only: true,
        spell_types: None,
        window: crate::state::PlayLifetime::WhileExiled,
    });

    engine
        .start_permitted_free_cast(me, card)
        .expect("Ice is another name");
    let wizard = engine.cast_wizard.as_ref().expect("the cast is under way");
    assert_eq!(
        wizard.options.iter().map(|o| o.kind).collect::<Vec<_>>(),
        [CastModeKind::Face(1)]
    );
}

/// So is a granted escape, which is offered per face from the graveyard.
#[test]
fn a_granted_escape_offers_only_the_face_a_lock_leaves() {
    let me = PlayerId::new(0);
    let mut engine = table_with(fire_ice(), Some((fire_ice(), 0)));
    let card = find(&engine, crate::zone::ZoneLocation::Hand(me), fire_ice()).unwrap();
    let state = engine
        .dev_state_mut(me)
        .expect("the harness may set boards up");
    let card = state
        .move_object(
            card,
            crate::zone::ZoneLocation::Graveyard(me),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    let timestamp = state.next_timestamp();
    let filter = crate::effects::EffectFilter::object(state, card);
    state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: None,
        controller: me,
        origin: crate::effects::EffectOrigin::Resolution,
        layer: baylee_cards_dsl::Layer::Text,
        timestamp,
        duration: baylee_cards_dsl::Duration::Indefinitely,
        filter,
        modifier: baylee_cards_dsl::Modifier::GrantsEscape(0),
    });

    let options = casting::granted_escape_options(
        engine.state(),
        &engine.lookup,
        me,
        card,
        casting::Paying::DuringTheCast,
    );
    assert!(!options.is_empty(), "Ice escapes");
    assert!(
        options
            .iter()
            .all(|o| matches!(o.kind, CastModeKind::EscapeGranted { face: 1, .. })),
        "and Fire does not: {options:?}"
    );
}
