//! Convoke (CR 702.51), and the two things that were wrong with it.
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

use super::testkit::{Duel, RegistryLookup, card_index, keep_mulligans, reach_main_phase};
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
