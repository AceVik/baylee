//! The order of triggers at a table (CR 603.3b): who is told what, what
//! answers for a seat nobody is at, and what a record of it holds.
//!
//! The board asks at once: Copper Tablet and Phyrexian Arena both trigger on
//! seat 0's first upkeep and do different things.

use super::*;
use crate::record::{Checked, replay};
use baylee_core::generated::index::{COPPER_TABLET, PHYREXIAN_ARENA};
use baylee_core::ids::{AbilityRef, ObjectId};
use baylee_engine::choice::{ArrangePlace, ArrangePrompt};
use baylee_engine::zone::ZoneLocation;
use baylee_view::StackItem;

const ME: PlayerId = PlayerId::new(0);
const THEM: PlayerId = PlayerId::new(1);

fn entry(card: CardIndex) -> DeckEntry {
    DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card,
        print: PrintRef::new(0),
    }
}

/// Seat 0 at the two artifacts, both chairs a player's, under the rule.
fn table() -> GamePreset {
    let mut preset = test_preset();
    preset.house_rules.starting_player = Some(0);
    preset.house_rules.order_triggers = true;
    preset.seats[1].controller = SeatController::Open;
    preset.seats[0].starting_battlefield = vec![entry(COPPER_TABLET), entry(PHYREXIAN_ARENA)];
    preset
}

fn keep_hands(session: &mut Session) {
    session.pump();
    for seat in [ME, THEM] {
        session
            .act(seat, PlayerAction::MulliganKeep)
            .expect("a keep");
    }
    session.pump();
}

fn at_the_question(preset: &GamePreset) -> Session {
    let mut session = Session::new(preset).expect("the table builds");
    keep_hands(&mut session);
    for _ in 0..8 {
        if is_the_question(session.pending()) {
            return session;
        }
        let seat = session.awaiting_seat().expect("somebody is asked");
        session
            .act(seat, PlayerAction::PassPriority)
            .expect("a pass");
        session.pump();
    }
    panic!("never asked: {:?}", session.pending());
}

const fn is_the_question(pending: &Pending) -> bool {
    matches!(
        pending,
        Pending::Arrange {
            prompt: ArrangePrompt::TriggerOrder,
            ..
        }
    )
}

fn stack_sources(session: &Session) -> Vec<ObjectId> {
    let state = session.state();
    state
        .zones
        .list(ZoneLocation::Stack)
        .iter()
        .rev()
        .filter_map(|id| state.object(*id)?.ability.map(|loc| loc.source))
        .collect()
}

fn source(row: &baylee_view::OrderedTrigger) -> ObjectId {
    let StackItem::Ability { source, .. } = row.item else {
        panic!("a trigger is an ability: {:?}", row.item)
    };
    source
}

/// The seat asked is told the abilities, each with the key it is remembered
/// by, and nobody else is told any of it: not the other chair, not a
/// spectator's feed, not an agent's plain view.
#[test]
fn only_the_seat_asked_is_told_what_its_triggers_are() {
    let session = at_the_question(&table());
    let Pending::Arrange { cards, piles, .. } = session.pending().clone() else {
        panic!("the question")
    };
    assert_eq!(piles[0].place, ArrangePlace::Stack);

    let view = seat_view(&session, ME);
    let handles: Vec<ObjectId> = view.trigger_order.iter().map(|t| t.handle).collect();
    assert_eq!(handles, cards, "the rows are the question's, in its order");
    let keys: Vec<AbilityRef> = view
        .trigger_order
        .iter()
        .filter_map(|t| match t.item {
            StackItem::Ability { ability, .. } => ability,
            StackItem::Spell => None,
        })
        .collect();
    assert!(
        keys.contains(&AbilityRef::new(COPPER_TABLET, 0)),
        "{keys:?}"
    );
    assert!(
        keys.contains(&AbilityRef::new(PHYREXIAN_ARENA, 0)),
        "{keys:?}"
    );
    assert!(view.trigger_order.iter().all(|t| matches!(
        t.item,
        StackItem::Ability {
            text: Some(_),
            rules: Some(_),
            ..
        }
    )));
    assert_ne!(view.trigger_order[0].twin, view.trigger_order[1].twin);
    let names: Vec<&str> = view.trigger_order.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"Copper Tablet"), "{names:?}");

    assert!(seat_view(&session, THEM).trigger_order.is_empty());
    assert!(session.observe_view(THEM).trigger_order.is_empty());
    let (_, plain) = session.view_for(ME).expect("asked");
    assert!(
        plain.trigger_order.is_empty(),
        "an agent's view stays as it was"
    );
    let (_, context) = session.view_for_with_context(ME).expect("asked");
    assert_eq!(context.trigger_order.len(), 2);
}

/// Nothing about a hidden card comes through a row: a card in another hand
/// is no object this seat can be told of, one in its own hand is.
#[test]
fn what_a_trigger_is_about_is_told_only_as_far_as_the_seat_can_see() {
    let session = at_the_question(&table());
    let state = session.state();
    let theirs = state.zones.list(ZoneLocation::Hand(THEM))[0];
    let mine = state.zones.list(ZoneLocation::Hand(ME))[0];
    assert!(crate::view::visible_event_object(state, theirs, ME).is_none());
    assert!(crate::view::visible_event_object(state, theirs, THEM).is_some());
    assert!(crate::view::visible_event_object(state, mine, ME).is_some());
}

/// The clock answers with the order offered, and that is an answer a client
/// can write on its button: the order a game without the question plays.
#[test]
fn the_clock_keeps_the_order_offered() {
    let mut plain = table();
    plain.house_rules.order_triggers = false;
    let mut played = Session::new(&plain).expect("the table builds");
    keep_hands(&mut played);
    for _ in 0..8 {
        if stack_sources(&played).len() == 2 {
            break;
        }
        let seat = played.awaiting_seat().expect("somebody is asked");
        played
            .act(seat, PlayerAction::PassPriority)
            .expect("a pass");
        played.pump();
    }

    let mut session = at_the_question(&table());
    let at = session.asked_at(ME).expect("the seat is asked");
    let answer = session.timeout_action(ME).expect("an answer to write down");
    assert!(session.pending().answer_fault(&answer).is_none());
    session
        .answer_by_clock(ME, at)
        .expect("the seat is asked")
        .expect("the clock's answer stands");
    assert_eq!(stack_sources(&session), stack_sources(&played));
    assert_eq!(stack_sources(&session).len(), 2);
}

/// The house at a table: asked, it answers, and the game does not wait.
#[test]
fn a_house_seat_answers_and_the_game_goes_on() {
    let mut preset = table();
    preset.seats[0].controller = SeatController::Ai(AIProfile::default());
    let mut session = Session::new(&preset).expect("the table builds");
    session.pump();
    session
        .act(THEM, PlayerAction::MulliganKeep)
        .expect("a keep");
    for _ in 0..16 {
        session.pump();
        assert!(!is_the_question(session.pending()), "the house answered");
        if session.state().turn.number > 1 {
            return;
        }
        if let Some(seat) = session.awaiting_seat()
            && seat == THEM
            && matches!(session.pending(), Pending::Priority { .. })
        {
            session
                .act(seat, PlayerAction::PassPriority)
                .expect("a pass");
        }
    }
    assert!(
        session.state().players[0].life < 20
            || !session.state().zones.list(ZoneLocation::Stack).is_empty(),
        "the triggers went on: {:?}",
        session.pending()
    );
}

/// A player who steps away mid-question is stood in for like any other.
#[test]
fn a_stand_in_answers_for_a_player_who_left_the_chair() {
    let mut session = at_the_question(&table());
    assert!(session.stand_in(ME));
    session.pump();
    assert!(
        !is_the_question(session.pending()),
        "{:?}",
        session.pending()
    );
    assert_eq!(stack_sources(&session).len(), 2);
}

/// A game with the question is a record like any other: the rule in its
/// header, the answer among its inputs, and the same game when replayed.
#[test]
fn a_game_with_the_question_replays_its_record() {
    let preset = table();
    let mut session =
        Session::new_recorded(&preset, crate::record::this_build()).expect("the table builds");
    keep_hands(&mut session);
    for _ in 0..8 {
        if is_the_question(session.pending()) {
            break;
        }
        let seat = session.awaiting_seat().expect("somebody is asked");
        session
            .act(seat, PlayerAction::PassPriority)
            .expect("a pass");
        session.pump();
    }
    let Pending::Arrange { cards, .. } = session.pending().clone() else {
        panic!("the question: {:?}", session.pending())
    };
    let reversed: Vec<ObjectId> = cards.iter().rev().copied().collect();
    let rows = seat_view(&session, ME).trigger_order;
    let on_top = rows
        .iter()
        .find(|t| t.handle == reversed[0])
        .map(source)
        .expect("a row for it");
    session
        .act(
            ME,
            PlayerAction::Arrange {
                piles: vec![reversed],
            },
        )
        .expect("an order");
    session.pump();
    assert_eq!(
        stack_sources(&session)[0],
        on_top,
        "the first listed is on top"
    );

    let record = session.take_record();
    let text = String::from_utf8_lossy(&record);
    assert!(
        text.contains("\"order_triggers\":true"),
        "the header says so"
    );
    let replayed = replay(&record).expect("the record replays");
    assert_eq!(replayed.engine.snapshot_hash(), session.snapshot_hash());
    assert_eq!(replayed.checked, Checked::Exact);
}

/// A record from before the rule has no such field, reads as off, and a game
/// with it off writes none: nothing in a stored record moves.
#[test]
fn the_rule_off_is_not_written_and_absent_reads_as_off() {
    let mut preset = table();
    preset.house_rules.order_triggers = false;
    let written = serde_json::to_value(&preset.house_rules).expect("house rules");
    assert!(written.get("order_triggers").is_none(), "{written}");
    let read: HouseRules = serde_json::from_value(written).expect("house rules");
    assert!(!read.order_triggers);
}
