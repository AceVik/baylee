//! One object named twice in one answer, at every door that takes a list.
//!
//! Each door checked the answer's *length* against its question and each
//! member against the options, and none asked whether two members were the
//! same one. So `[c, c]` passed wherever two was a legal count, and what it
//! bought depended on who read the list next: a spell kept both copies as its
//! targets (CR 115.3), convoke and delve counted two payments for one tap and
//! one exile (CR 702.51a, CR 702.66a), and the discard and the mulligan
//! moved one card where the count said two (CR 514.1, CR 103.5).
//!
//! Every test here fails against the doors as they were, and every one
//! answers the same question a second time with distinct cards afterwards —
//! a refusal that left the question unanswerable would pass the first half.

use super::testkit::{
    Duel, RegistryLookup, card_index, keep_mulligans, pass_until, reach_main_phase, seed_graveyard,
    tap_all_mana,
};
use super::*;
use crate::choice::TargetPrompt;
use crate::object::Status;
use baylee_core::ids::CardIndex;

fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}
fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}
/// `{2}{W}{W}` instant, convoke, "any number of target nonland permanents
/// you control phase out" — one instance of "target" with room for two, and
/// a convoke question, on one card.
fn clever_concealment() -> CardIndex {
    card_index("42bb7ea9-f6e4-4551-8d93-3b1eae84b865")
}
/// `{1}{W}` 1/1 — a body to tap, and a legal phase-out target.
fn ondu_cleric() -> CardIndex {
    card_index("f4232466-dd6a-49bf-be6c-95905c3ded17")
}
/// `{6}{U}{U}` instant, delve.
fn dig_through_time() -> CardIndex {
    card_index("f8b17b89-26ce-4208-874a-9e1d66514640")
}

fn zone(engine: &Engine<RegistryLookup>, at: ZoneLocation) -> Vec<ObjectId> {
    engine.state().zones.list(at).clone()
}

fn is_tapped(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    engine
        .state()
        .object(id)
        .is_some_and(|o| o.status.contains(Status::TAPPED))
}

/// Seat 0's two Clerics, with Clever Concealment cast off two Plains and the
/// cast standing at its first question.
fn concealment_cast() -> (Engine<RegistryLookup>, PlayerId, [ObjectId; 2]) {
    let mut engine = Duel::new(4, plains())
        .hand(0, &[clever_concealment()])
        .battlefield(0, &[plains(), plains(), ondu_cleric(), ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    let seat = PlayerId::new(0);
    reach_main_phase(&mut engine, seat);
    let clerics: Vec<ObjectId> = zone(&engine, ZoneLocation::Battlefield)
        .into_iter()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == ondu_cleric())
        })
        .collect();
    let clerics: [ObjectId; 2] = clerics.try_into().expect("two Clerics on the table");
    tap_all_mana(&mut engine, seat);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let card = *legal
        .castable
        .first()
        .expect("two Plains and two Clerics to convoke with pay {2}{W}{W}");
    engine
        .apply(seat, PlayerAction::CastSpell { card })
        .expect("the spell is castable");
    (engine, seat, clerics)
}

fn choose_targets(objects: Vec<ObjectId>) -> PlayerAction {
    PlayerAction::ChooseTargets {
        objects,
        players: vec![],
    }
}

/// "Any number of target nonland permanents" is one instance of the word,
/// so a Cleric may be one of its targets once (CR 115.3) — and was stored
/// twice, because the door counted the answer and never compared it.
#[test]
fn one_instance_of_target_cannot_name_an_object_twice() {
    let (mut engine, seat, [a, b]) = concealment_cast();
    let Pending::ChooseTargets { reason, max, .. } = engine.pending().clone() else {
        panic!("expected the target question, got {:?}", engine.pending())
    };
    assert_eq!(reason, TargetPrompt::Targets, "targets are asked first");
    assert!(max >= 2, "room for two targets, or the count refuses first");

    assert!(
        engine.apply(seat, choose_targets(vec![a, a])).is_err(),
        "one Cleric named twice for one instance of \"target\""
    );
    assert!(
        matches!(
            engine.pending(),
            Pending::ChooseTargets {
                reason: TargetPrompt::Targets,
                ..
            }
        ),
        "a refused answer leaves the question standing"
    );
    engine
        .apply(seat, choose_targets(vec![a, b]))
        .expect("two different Clerics are two legal targets");
}

/// Two untapped Clerics make the convoke question's `max` two, so
/// `[a, a]` passed the count — and the wizard reduced the cost by
/// `convoke_taps.len()`, two mana, for tapping one creature once.
#[test]
fn convoke_cannot_tap_one_creature_twice() {
    let (mut engine, seat, [a, b]) = concealment_cast();
    engine
        .apply(seat, choose_targets(vec![]))
        .expect("phasing out nothing is a legal answer");
    let Pending::ChooseTargets { reason, max, .. } = engine.pending().clone() else {
        panic!("expected the convoke question, got {:?}", engine.pending())
    };
    assert_eq!(reason, TargetPrompt::Convoke);
    assert_eq!(max, 2, "two untapped Clerics");

    assert!(
        engine.apply(seat, choose_targets(vec![a, a])).is_err(),
        "one Cleric tapped twice paid for two mana"
    );
    assert!(
        !is_tapped(&engine, a) && !is_tapped(&engine, b),
        "a refused answer taps nothing"
    );
    assert!(
        matches!(
            engine.pending(),
            Pending::ChooseTargets {
                reason: TargetPrompt::Convoke,
                ..
            }
        ),
        "and leaves the convoke question standing"
    );

    engine
        .apply(seat, choose_targets(vec![a, b]))
        .expect("two Clerics pay the generic half");
    assert!(
        !engine.state().zones.stack_is_empty(),
        "the spell reached the stack"
    );
    assert!(is_tapped(&engine, a) && is_tapped(&engine, b));
}

/// Six cards in the graveyard make the delve question's `max` six, and an
/// answer of six naming one card twice passed the count: the wizard reduced
/// the cost by `delve_exiles.len()` and exiled what the list named, so five
/// cards paid `{6}` and the sixth stayed in the graveyard (CR 702.66a).
#[test]
fn delve_cannot_exile_one_card_twice() {
    let mut engine = Duel::new(5, island())
        .hand(0, &[dig_through_time()])
        .battlefield(0, &[island(), island()])
        .start();
    keep_mulligans(&mut engine);
    let seat = PlayerId::new(0);
    seed_graveyard(&mut engine, seat, 6);
    reach_main_phase(&mut engine, seat);
    tap_all_mana(&mut engine, seat);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let card = *legal
        .castable
        .first()
        .expect("two Islands and six cards to delve pay {6}{U}{U}");
    engine
        .apply(seat, PlayerAction::CastSpell { card })
        .expect("the spell is castable");
    let Pending::ChooseCards { options, max, .. } = engine.pending().clone() else {
        panic!("expected the delve question, got {:?}", engine.pending())
    };
    assert_eq!(max, 6);
    let grave = zone(&engine, ZoneLocation::Graveyard(seat));

    let mut twice = vec![options[0]; 2];
    twice.extend_from_slice(&options[1..5]);
    assert!(
        engine
            .apply(seat, PlayerAction::ChooseObjects { objects: twice })
            .is_err(),
        "five cards, one of them named twice, paid for six mana"
    );
    assert_eq!(
        zone(&engine, ZoneLocation::Graveyard(seat)),
        grave,
        "a refused answer exiles nothing"
    );
    assert!(
        matches!(engine.pending(), Pending::ChooseCards { .. }),
        "and leaves the delve question standing"
    );

    engine
        .apply(seat, PlayerAction::ChooseObjects { objects: options })
        .expect("six different cards pay the generic half");
    assert!(
        !engine.state().zones.stack_is_empty(),
        "the spell reached the stack"
    );
}

/// Nine cards at the end of a turn discard two, and `[c, c]` discarded one
/// and left the hand at eight — over its maximum, with the step over.
#[test]
fn the_cleanup_discard_cannot_name_one_card_twice() {
    let nine = [island(); 9];
    let mut engine = Duel::new(6, island()).hand(0, &nine).hand(1, &nine).start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::DiscardChoice { .. })
    });
    let Pending::DiscardChoice { player, count } = engine.pending().clone() else {
        unreachable!("pass_until stopped on it")
    };
    assert_eq!(count, 2, "nine cards against a maximum of seven");
    let hand = zone(&engine, ZoneLocation::Hand(player));
    assert_eq!(hand.len(), 9);

    assert!(
        engine
            .apply(
                player,
                PlayerAction::ChooseObjects {
                    objects: vec![hand[0], hand[0]],
                },
            )
            .is_err(),
        "one card named twice discarded one of the two"
    );
    assert_eq!(
        zone(&engine, ZoneLocation::Hand(player)),
        hand,
        "a refused answer discards nothing"
    );

    engine
        .apply(
            player,
            PlayerAction::ChooseObjects {
                objects: hand[..2].to_vec(),
            },
        )
        .expect("two different cards are the discard");
    assert_eq!(zone(&engine, ZoneLocation::Hand(player)).len(), 7);
}

/// After enough mulligans to owe two cards, `[c, c]` bottomed one and kept
/// an eighth card in a seven-card hand — the exploit a player could reach
/// on purpose, and the reason this door is in the list at all.
#[test]
fn a_mulligan_cannot_bottom_one_card_twice() {
    let mut engine = Duel::new(7, island()).start();
    let Pending::Mulligan { player, .. } = engine.pending().clone() else {
        panic!("expected a mulligan, got {:?}", engine.pending())
    };
    // Three, so that two are owed whether or not the first one is free.
    for _ in 0..3 {
        engine
            .apply(player, PlayerAction::MulliganTake)
            .expect("a mulligan may be taken");
    }
    engine
        .apply(player, PlayerAction::MulliganKeep)
        .expect("the hand is kept");
    let Pending::MulliganBottom { player: p, count } = engine.pending().clone() else {
        panic!("expected the bottom question, got {:?}", engine.pending())
    };
    assert_eq!(p, player);
    assert!(
        count >= 2,
        "two cards owed, or `[c, c]` is refused by its length"
    );
    let hand = zone(&engine, ZoneLocation::Hand(player));
    let library = zone(&engine, ZoneLocation::Library(player));

    let mut twice = vec![hand[0]; 2];
    twice.extend_from_slice(&hand[1..count as usize - 1]);
    assert_eq!(twice.len(), count as usize);
    assert!(
        engine
            .apply(player, PlayerAction::ChooseObjects { objects: twice })
            .is_err(),
        "one card named twice bottomed one card for two"
    );
    assert_eq!(
        zone(&engine, ZoneLocation::Hand(player)),
        hand,
        "a refused answer moves nothing out of the hand"
    );
    assert_eq!(
        zone(&engine, ZoneLocation::Library(player)),
        library,
        "and nothing into the library"
    );

    engine
        .apply(
            player,
            PlayerAction::ChooseObjects {
                objects: hand[..count as usize].to_vec(),
            },
        )
        .expect("that many different cards is the answer");
    assert_eq!(
        zone(&engine, ZoneLocation::Hand(player)).len(),
        7 - count as usize
    );
}
