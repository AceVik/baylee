//! Convoke (CR 702.51), the two things that were wrong with it, and the two
//! other cost reductions that were wrong in the same place.
//!
//! Convoke asks a question during the cast, and answering it went to the
//! wrong place: `ChooseTargets` was routed to "the wizard" without asking
//! which stage the wizard was standing in, so the tapped permanents were
//! filed as the spell's targets and the wizard was put back at its kicker
//! stage — which asked the kicker question again, over a board nothing had
//! changed. A self-play game stalled on turn 34 casting one spell forever,
//! and `CastWizard::convoke_taps` was written by no reachable code at all.
//!
//! The second was underneath it: the taps and the delve exiles were spent
//! *before* the mana was, so a cast that then could not pay the rest
//! returned the spell to hand with the creatures left tapped. CR 601.2h
//! reverses the whole casting, and there is no half of it to keep.
//!
//! Delve (CR 702.66a) is here for the third: it is the same reduction of the
//! same generic half, paid in the same wizard a stage earlier, and the
//! `can_cast` fix that taught the offer to count convoke sources left it out
//! — so the one delve card in the pool was offered exactly when its printed
//! cost was already payable, which is the one case delve is not for.
//!
//! And the printed cost reduction for the fourth, which is the same fault
//! pointing the other way: the wizard applied Surgical Metamorph's discount
//! when it built the cast options and the offer did not, so the seat the
//! discount belongs to was never shown the card at the price it would have
//! paid. Convoke, delve and that discount are now one pair of readers in
//! `casting`, asked by both probes — the module is really about that: what
//! moves a mana cost before anybody asks whether the pool covers it.

use super::testkit::{
    Duel, RegistryLookup, card_index, keep_mulligans, pass_until, reach_main_phase,
};
use super::*;
use baylee_core::ids::CardIndex;

fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}
/// `{2}{W}{W}` instant, convoke, "any number of target nonland permanents
/// you control phase out".
fn clever_concealment() -> CardIndex {
    card_index("42bb7ea9-f6e4-4551-8d93-3b1eae84b865")
}
/// `{1}{W}` 1/1 — a body to tap, and a legal phase-out target.
fn ondu_cleric() -> CardIndex {
    card_index("f4232466-dd6a-49bf-be6c-95905c3ded17")
}
fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}
/// `{6}{U}{U}` instant, delve — the whole of the pool's use of the keyword.
fn dig_through_time() -> CardIndex {
    card_index("f8b17b89-26ce-4208-874a-9e1d66514640")
}
/// `{3}{U}` creature, "costs {1} less if you weren't the starting player" —
/// the whole of the pool's use of `FaceDef::cost_reduction`.
fn surgical_metamorph() -> CardIndex {
    card_index("4f328996-f9dd-4c7a-9548-bc4b9d0d943f")
}

/// Moves `n` cards off the top of `seat`'s library into their graveyard, and
/// answers with them.
///
/// A test harness rewriting the board, which is what the dev capability is
/// for: how a card reaches a graveyard is not what the delve test is about,
/// and milling six with a spell would put that spell's own rules text between
/// the premise and the assertion.
#[track_caller]
fn bury(engine: &mut Engine<RegistryLookup>, seat: PlayerId, n: usize) -> Vec<ObjectId> {
    let state = engine
        .dev_state_mut(seat)
        .expect("the test harness grants dev commands");
    let doomed: Vec<ObjectId> = state
        .zones
        .list(crate::zone::ZoneLocation::Library(seat))
        .iter()
        .copied()
        .take(n)
        .collect();
    assert_eq!(doomed.len(), n, "the library holds that many cards");
    doomed
        .iter()
        .map(|id| {
            state
                .move_object(
                    *id,
                    crate::zone::ZoneLocation::Graveyard(seat),
                    crate::zone::ZonePosition::Top,
                    crate::event::Cause::DevCommand,
                )
                .expect("a card moves to the graveyard")
        })
        .collect()
}

/// Every untapped permanent `seat` controls, in the engine's own order.
fn permanents(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
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
                .is_some_and(|o| o.controller == seat)
        })
        .collect()
}

fn is_tapped(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    engine
        .state()
        .object(id)
        .is_some_and(|o| o.status.contains(crate::object::Status::TAPPED))
}

/// Taps every land `seat` controls for mana.
#[track_caller]
fn tap_all_lands(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    loop {
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        let Some(&source) = legal.mana_abilities.first() else {
            return;
        };
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source })
            .expect("a land taps for mana");
    }
}

/// Walks a cast from `CastSpell` to the stack, answering whatever the
/// wizard asks: the phase-out targets, then the convoke taps.
///
/// It answers by *stage*, not by a script, because the point of the bug was
/// that two stages arrived as the same `Pending` — a test that answered a
/// fixed sequence would have agreed with the broken engine.
#[track_caller]
fn cast_answering(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    card: ObjectId,
    targets: &[ObjectId],
    convoke: &[ObjectId],
) {
    engine
        .apply(seat, PlayerAction::CastSpell { card })
        .expect("the spell is castable");
    for _ in 0..8 {
        match engine.pending().clone() {
            Pending::ChooseTargets { options, .. } => {
                // The convoke question offers the permanents that may be
                // tapped; the target question offers what may be phased
                // out. Here they are told apart by what the test asked
                // for, which is all a caller has: the engine sends the
                // same variant for both.
                let answer: Vec<ObjectId> = if options.iter().any(|o| convoke.contains(o))
                    && !options.iter().any(|o| targets.contains(o))
                {
                    convoke.to_vec()
                } else if targets.iter().all(|t| options.contains(t)) {
                    targets.to_vec()
                } else {
                    convoke.to_vec()
                };
                engine
                    .apply(
                        seat,
                        PlayerAction::ChooseTargets {
                            objects: answer,
                            players: vec![],
                        },
                    )
                    .expect("the choice is legal");
            }
            Pending::Priority { .. } => return,
            other => panic!("unexpected question during the cast: {other:?}"),
        }
    }
    panic!("the cast never finished");
}

/// Two lands and two creatures pay a `{2}{W}{W}` spell: the creatures
/// convoke away the generic half, the lands pay the coloured half, and the
/// spell reaches the stack.
///
/// Before the routing fix this never got past the question — the answer
/// was filed as the spell's targets and the wizard asked again.
#[test]
fn convoke_taps_the_creatures_and_the_spell_is_cast() {
    let mut engine = Duel::new(4, plains())
        .hand(0, &[clever_concealment()])
        .battlefield(0, &[plains(), plains(), ondu_cleric(), ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    let seat = PlayerId::new(0);
    reach_main_phase(&mut engine, seat);

    let board = permanents(&engine, seat);
    let creatures: Vec<ObjectId> = board
        .iter()
        .copied()
        .filter(|id| {
            engine.state().object(*id).is_some_and(|o| {
                o.characteristics()
                    .types
                    .contains(baylee_core::types::TypeSet::CREATURE)
            })
        })
        .collect();
    assert_eq!(creatures.len(), 2, "two bodies to convoke with");

    tap_all_lands(&mut engine, seat);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority")
    };
    let card = *legal
        .castable
        .first()
        .expect("the spell is offered once the lands are tapped");

    cast_answering(&mut engine, seat, card, &creatures, &creatures);

    assert!(
        !engine.state().zones.stack_is_empty(),
        "the spell never reached the stack"
    );
    for id in &creatures {
        assert!(
            is_tapped(&engine, *id),
            "a convoked creature was left untapped"
        );
    }
}

/// The question the engine asks for convoke is answerable, and answering it
/// *moves* — which is the whole of the stall: the wizard used to come back
/// to the same question with nothing changed.
#[test]
fn answering_the_convoke_question_does_not_ask_it_again() {
    let mut engine = Duel::new(4, plains())
        .hand(0, &[clever_concealment()])
        .battlefield(0, &[plains(), plains(), ondu_cleric(), ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    let seat = PlayerId::new(0);
    reach_main_phase(&mut engine, seat);
    tap_all_lands(&mut engine, seat);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority")
    };
    let card = *legal.castable.first().expect("the spell is castable");
    let before = engine.state().snapshot_hash();
    engine
        .apply(seat, PlayerAction::CastSpell { card })
        .expect("cast starts");

    // Answer every question the wizard asks with everything it offers, and
    // count them. A wizard that loops asks the same one forever; a wizard
    // that works asks each of its stages once.
    let mut asked = 0;
    while let Pending::ChooseTargets { options, .. } = engine.pending().clone() {
        asked += 1;
        assert!(asked <= 4, "the cast wizard asked the same question again");
        engine
            .apply(
                seat,
                PlayerAction::ChooseTargets {
                    objects: options,
                    players: vec![],
                },
            )
            .expect("answering with everything offered is legal");
    }
    assert_ne!(
        engine.state().snapshot_hash(),
        before,
        "the cast changed nothing at all"
    );
}

/// A refused cast must not leave the question it was asking on the table.
///
/// Reported from a live game against a waterbend card: "sie wollte von mir 99
/// Targets, hat 5 Mana gekostet und als ich keine Targets bestimmen konnte,
/// kam nur die Meldung für invalid targets oder costs und ab hier war das
/// Spiel gar nicht mehr spielbar."
///
/// The last clause is what this pins. The convoke question offers `min: 0`,
/// so declining to tap anything is a legal answer — and castability is
/// computed with the *maximum* convoke already subtracted, so declining it
/// can leave a cost the pool cannot pay. `finish_cast` then refuses and tears
/// the wizard down, CR 601.2h reversing the whole casting.
///
/// This was written to prove that the refusal orphaned the wizard's
/// question: `Engine::apply` propagates `apply_inner`'s error with `?`, which
/// is before it reaches `run_until_choice`. It does not — the wizard's own
/// error branch resumes the game before returning. The first version of this
/// test therefore passed, and was wrong to: it asked only for
/// `Pending::Priority` and never for *whose*, and the answer was the
/// opponent's. `waterbend_tests` has the reproduction that named the seat.
#[test]
fn a_cast_that_cannot_pay_hands_priority_back() {
    let mut engine = Duel::new(4, plains())
        .hand(0, &[clever_concealment()])
        .battlefield(0, &[plains(), plains(), ondu_cleric(), ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    let seat = PlayerId::new(0);
    reach_main_phase(&mut engine, seat);

    tap_all_lands(&mut engine, seat);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority")
    };
    // The premise: two Plains float {W}{W}, and the engine offers a {2}{W}{W}
    // spell anyway because two untapped creatures *could* convoke the {2}.
    let card = *legal
        .castable
        .first()
        .expect("convoke makes the spell castable on two lands");

    engine
        .apply(seat, PlayerAction::CastSpell { card })
        .expect("the spell is castable");

    // Answer every question the wizard asks with "nothing", which is legal
    // for both of them and is what leaves the cost unpayable.
    let mut refused = None;
    for _ in 0..8 {
        match engine.pending().clone() {
            Pending::ChooseTargets { .. } => {
                if let Err(err) = engine.apply(
                    seat,
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![],
                    },
                ) {
                    refused = Some(err);
                    break;
                }
            }
            Pending::Priority { .. } => break,
            other => panic!("unexpected question during the cast: {other:?}"),
        }
    }
    assert!(
        refused.is_some(),
        "the engine paid a cost it had no mana for"
    );

    // The claim: after the refusal *this* seat is asked something it can
    // answer. Naming the seat is the whole assertion — the first version of
    // this test only asked for `Pending::Priority` and passed while the
    // question went to the opponent, which is the bug it was written to find.
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat),
        "the refused cast left its own question standing: {:?}",
        engine.pending()
    );
    // And the spell is back in hand with nothing spent — the other half of
    // CR 601.2h, already true before this fix and worth pinning beside it.
    assert!(
        engine.state().zones.stack_is_empty(),
        "the unpayable spell reached the stack"
    );
    for id in permanents(&engine, seat) {
        if engine.state().object(id).is_some_and(|o| {
            o.characteristics()
                .types
                .contains(baylee_core::types::TypeSet::CREATURE)
        }) {
            assert!(!is_tapped(&engine, id), "a creature was convoked anyway");
        }
    }
}

/// `{6}{U}{U}` on two islands, with six cards in the graveyard.
///
/// Delve, CR 702.66a: "for each generic mana in this spell's total cost, you
/// may exile a card from your graveyard rather than pay that mana". The
/// generic half of a cost, one card each, after the total cost is worked out
/// — which is convoke's arithmetic exactly, and the reason both are one
/// subtraction in `can_cast`.
///
/// They were not. The convoke fix wrote its own reason down —
/// "`can_cast` did not count them at all, so a convoke spell was offered as
/// castable exactly when its printed cost was already payable, which is the
/// one case convoke is not for" — and delve was left sitting one field over
/// with the whole sentence still true of it. Dig Through Time is the only
/// delve card in the pool, is `Coverage::Implemented`, and the deckbuilder
/// offers it as playable: eight mana or nothing, with a full graveyard doing
/// nothing at all.
///
/// The graveyard is filled through `dev_state_mut` rather than played into,
/// because what is under test is the offer and not how cards get to a
/// graveyard.
#[test]
fn delve_makes_a_spell_castable_that_the_pool_alone_could_not_pay() {
    let mut engine = Duel::new(5, island())
        .hand(0, &[dig_through_time()])
        .battlefield(0, &[island(), island()])
        .start();
    keep_mulligans(&mut engine);
    let seat = PlayerId::new(0);
    let buried = bury(&mut engine, seat, 6);
    reach_main_phase(&mut engine, seat);
    tap_all_lands(&mut engine, seat);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let card = *legal
        .castable
        .first()
        .expect("two islands and six cards in the graveyard pay {6}{U}{U}");

    engine
        .apply(seat, PlayerAction::CastSpell { card })
        .expect("the spell is castable");
    let Pending::ChooseCards { options, max, .. } = engine.pending().clone() else {
        panic!("expected the delve question, got {:?}", engine.pending())
    };
    assert_eq!(
        options.len(),
        6,
        "every card in the graveyard may be exiled"
    );
    assert_eq!(max, 6, "and all six of them at once");
    engine
        .apply(
            seat,
            PlayerAction::ChooseObjects {
                objects: buried.clone(),
            },
        )
        .expect("exiling all six is a legal answer");

    assert!(
        !engine.state().zones.stack_is_empty(),
        "the spell never reached the stack"
    );
    for id in &buried {
        assert_eq!(
            engine.state().object(*id).map(|o| o.zone),
            Some(crate::zone::Zone::Exile),
            "a delved card was not exiled"
        );
    }
    assert!(
        engine.state().players[seat.get() as usize]
            .mana_pool
            .is_empty(),
        "the two islands paid the coloured half"
    );
}

/// `{3}{U}` for the seat that did not start is `{2}{U}`, and three islands
/// pay it.
///
/// Surgical Metamorph prints "this spell costs {1} less to cast if you
/// weren't the starting player", which `FaceDef::cost_reduction` carries and
/// `cast_options` read when it built the normal-cost option — while
/// `can_cast`, the probe that decides whether the card is offered at all,
/// read the printed cost and nothing else. So the discount existed only after
/// the point it could no longer be reached: the seat entitled to it saw the
/// card greyed out at three mana and had to hold a fourth land to be shown a
/// spell that costs three.
///
/// The mirror of the delve test above, and deliberately kept beside it. Both
/// are the offer and the option list probing different costs; they simply
/// leaned opposite ways, and a fix for either alone would have left the other
/// looking like a different kind of problem.
#[test]
fn a_printed_cost_reduction_is_counted_by_the_offer_as_well() {
    let mut engine = Duel::new(6, island())
        .hand(1, &[surgical_metamorph()])
        .battlefield(1, &[island(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    let seat = PlayerId::new(1);
    assert_ne!(
        engine.state().starting_player,
        seat,
        "the discount is printed for the seat that did not start"
    );

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain) && e.state().turn.active == seat
    });
    let metamorph = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(seat))
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == surgical_metamorph()))
        })
        .expect("Surgical Metamorph in hand");
    tap_all_lands(&mut engine, seat);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&metamorph),
        "three islands pay {{2}}{{U}}, which is what this seat is charged"
    );
    engine
        .apply(seat, PlayerAction::CastSpell { card: metamorph })
        .expect("and the wizard charges the same price the offer quoted");
}
