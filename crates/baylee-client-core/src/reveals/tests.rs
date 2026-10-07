//! Which reveals stand up for this seat, and when they go: a reveal another
//! seat makes on a live socket, never one a telling from line 0 repeats,
//! never this seat's own, never a card the line did not name `Known`, and
//! never a card this seat's own question is already showing it.

use super::*;
use crate::test_support::{ViewBuilder, printed};
use baylee_core::ids::{CardIndex, PrintRef};

fn oid(slot: u32) -> ObjectId {
    ObjectId::new(slot, 0)
}

fn me() -> PlayerId {
    PlayerId::new(0)
}

fn them() -> PlayerId {
    PlayerId::new(1)
}

/// A card the line names to this seat, Mystical Tutor's find.
fn known(slot: u32, print: u16) -> LogObject {
    LogObject::Known {
        id: oid(slot),
        card: Some(CardIdentity {
            index: CardIndex::new(u32::from(print)),
            print: PrintRef::new(print),
            face: 0,
        }),
        token: None,
        name: "Lightning Bolt".to_string(),
    }
}

fn entry(event: LogEvent) -> LogEntry {
    LogEntry {
        turn: 3,
        repeat: 1,
        at: 0,
        event,
    }
}

fn revealed(player: PlayerId, cards: Vec<LogObject>) -> LogEntry {
    entry(LogEvent::Revealed { player, cards })
}

fn quiet() -> LogEntry {
    entry(LogEvent::Shuffled { player: them() })
}

fn view(seq: u64) -> PlayerView {
    let mut view = ViewBuilder::new(3).build();
    view.seq = seq;
    view
}

fn tail(from: u32, entries: Vec<LogEntry>) -> LogTail {
    LogTail { from, entries }
}

/// A seat that has been told the first two lines, as every live socket is
/// before anything can be revealed.
fn attached() -> (LogBook, Reveals) {
    let mut book = LogBook::new();
    let mut reveals = Reveals::new();
    reveals.take(&mut book, &tail(0, vec![quiet(), quiet()]), &view(1));
    assert!(reveals.current().is_none());
    (book, reveals)
}

#[test]
fn another_seats_reveal_on_a_live_socket_stands_up_with_the_card_the_line_named() {
    let (mut book, mut reveals) = attached();
    reveals.take(
        &mut book,
        &tail(2, vec![revealed(them(), vec![known(40, 7)])]),
        &view(2),
    );
    let shown = reveals.current().expect("the reveal stands up");
    assert_eq!(shown.player, them());
    assert_eq!(shown.cards.len(), 1);
    assert_eq!(shown.cards[0].id, oid(40));
    assert_eq!(
        shown.cards[0].card.map(|c| c.print),
        Some(PrintRef::new(7)),
        "the printing the line named, not one looked up again"
    );
}

#[test]
fn a_reveal_told_from_line_zero_is_history_on_attach_and_on_reconnect() {
    // A fresh client attaching mid-game: the whole log, from 0.
    let mut book = LogBook::new();
    let mut reveals = Reveals::new();
    reveals.take(
        &mut book,
        &tail(0, vec![quiet(), revealed(them(), vec![known(40, 7)])]),
        &view(9),
    );
    assert!(
        reveals.current().is_none(),
        "a reveal from before this socket attached is in the log, not in front of the player"
    );

    // A long telling comes in chunks that repeat the view's seq; the chunks
    // after the first start past 0 and are history all the same.
    reveals.take(
        &mut book,
        &tail(2, vec![revealed(them(), vec![known(41, 8)])]),
        &view(9),
    );
    assert!(
        reveals.current().is_none(),
        "the second chunk of a telling from 0 is the same telling"
    );

    // Live again: the next frame is news.
    reveals.take(
        &mut book,
        &tail(3, vec![revealed(them(), vec![known(42, 9)])]),
        &view(10),
    );
    assert_eq!(reveals.current().map(|r| r.cards[0].id), Some(oid(42)));
    assert!(reveals.dismiss());

    // A reconnect: the host tells the same log again from 0, with a line the
    // socket missed while it was down. Neither stands up.
    reveals.take(
        &mut book,
        &tail(
            0,
            vec![
                quiet(),
                revealed(them(), vec![known(40, 7)]),
                revealed(them(), vec![known(41, 8)]),
                revealed(them(), vec![known(42, 9)]),
                revealed(them(), vec![known(43, 10)]),
            ],
        ),
        &view(14),
    );
    assert!(
        reveals.current().is_none(),
        "a reconnect's telling stands nothing up again"
    );
}

#[test]
fn an_empty_tail_from_zero_marks_nothing_as_history() {
    let (mut book, mut reveals) = attached();
    // A question asked again: `{from: 0, entries: []}` beside the same view.
    reveals.take(&mut book, &tail(0, vec![]), &view(2));
    reveals.take(
        &mut book,
        &tail(2, vec![revealed(them(), vec![known(40, 7)])]),
        &view(2),
    );
    assert!(
        reveals.current().is_some(),
        "an empty tail tells nothing and must not make the next frame history"
    );
}

#[test]
fn my_own_reveal_is_not_held_up_to_me() {
    let (mut book, mut reveals) = attached();
    reveals.take(
        &mut book,
        &tail(2, vec![revealed(me(), vec![known(40, 7)])]),
        &view(2),
    );
    assert!(
        reveals.current().is_none(),
        "the seat chose the card and saw it; an echo would stand over its next question"
    );
}

#[test]
fn only_a_card_the_line_named_known_is_shown() {
    let (mut book, mut reveals) = attached();
    reveals.take(
        &mut book,
        &tail(
            2,
            vec![revealed(
                them(),
                vec![LogObject::Hidden, LogObject::FaceDown { id: oid(50) }],
            )],
        ),
        &view(2),
    );
    assert!(
        reveals.current().is_none(),
        "a line that names nothing this seat may see shows nothing, not an empty sheet"
    );

    reveals.take(
        &mut book,
        &tail(
            3,
            vec![revealed(
                them(),
                vec![
                    LogObject::Hidden,
                    known(40, 7),
                    LogObject::FaceDown { id: oid(50) },
                ],
            )],
        ),
        &view(3),
    );
    let shown = reveals.current().expect("the one known card stands up");
    assert_eq!(
        shown.cards.iter().map(|c| c.id).collect::<Vec<_>>(),
        vec![oid(40)],
        "the hidden and the face-down card are not among them"
    );
}

#[test]
fn a_card_my_own_question_shows_me_is_not_shown_twice() {
    let (mut book, mut reveals) = attached();
    // Another seat reveals its hand and this seat is choosing from it: the
    // sheet the question opened already draws the card.
    let mut asked = view(2);
    asked.looking_at = vec![printed(40, 1, "Lightning Bolt", 7)];
    reveals.take(
        &mut book,
        &tail(2, vec![revealed(them(), vec![known(40, 7), known(41, 8)])]),
        &asked,
    );
    assert_eq!(
        reveals
            .current()
            .map(|r| r.cards.iter().map(|c| c.id).collect::<Vec<_>>()),
        Some(vec![oid(41)])
    );
}

#[test]
fn one_frame_s_reveals_merge_per_seat_and_queue_across_seats() {
    let (mut book, mut reveals) = attached();
    reveals.take(
        &mut book,
        &tail(
            2,
            vec![
                revealed(them(), vec![known(40, 7)]),
                revealed(PlayerId::new(2), vec![known(60, 9)]),
                revealed(them(), vec![known(41, 8), known(40, 7)]),
            ],
        ),
        &view(2),
    );
    let first = reveals.current().expect("a reveal stands");
    assert_eq!(first.player, them());
    assert_eq!(
        first.cards.iter().map(|c| c.id).collect::<Vec<_>>(),
        vec![oid(40), oid(41)],
        "one seat's reveals in one frame are one picture, each card once"
    );
    assert_eq!(reveals.waiting(), 1);
    let number = first.number;
    assert!(reveals.dismiss());
    let second = reveals.current().expect("the other seat's waits behind");
    assert_eq!(second.player, PlayerId::new(2));
    assert!(second.number > number);
}

#[test]
fn a_reveal_goes_when_its_time_is_up_and_the_next_one_gets_its_own() {
    let (mut book, mut reveals) = attached();
    reveals.take(
        &mut book,
        &tail(2, vec![revealed(them(), vec![known(40, 7)])]),
        &view(2),
    );
    reveals.take(
        &mut book,
        &tail(
            3,
            vec![revealed(
                them(),
                vec![known(41, 8), known(42, 9), known(43, 10)],
            )],
        ),
        &view(3),
    );
    // The clock starts the first time it is asked, not when the line came.
    assert!(reveals.due(100.0));
    assert!(!reveals.tick(100.0));
    assert!(!reveals.due(100.0 + SHOW_SECS - 0.1));
    assert!(!reveals.tick(100.0 + SHOW_SECS - 0.1));
    assert_eq!(reveals.current().map(|r| r.cards.len()), Some(1));
    assert!(reveals.tick(100.0 + SHOW_SECS));
    let next = reveals
        .current()
        .expect("the second stands up as the first goes");
    assert_eq!(next.cards.len(), 3);
    assert!(
        (next.lasts() - (SHOW_SECS + 2.0 * PER_CARD_SECS)).abs() < 1e-9,
        "three cards stand longer than one"
    );
    // Its own time, counted from when it stood up.
    let up = 100.0 + SHOW_SECS;
    assert!(!reveals.tick(up + SHOW_SECS));
    assert!(reveals.tick(up + next_lasts(&reveals)));
    assert!(reveals.current().is_none());
    assert!(!reveals.due(1e9), "nothing standing is never due");
}

fn next_lasts(reveals: &Reveals) -> f64 {
    reveals.current().map_or(0.0, Reveal::lasts)
}

#[test]
fn no_reveal_stands_longer_than_the_longest() {
    let reveal = Reveal {
        player: them(),
        cards: (0..40)
            .map(|i| ShownCard {
                id: oid(i),
                card: None,
                token: Some(1),
            })
            .collect(),
        number: 1,
    };
    assert!((reveal.lasts() - LONGEST_SECS).abs() < 1e-9);
}

#[test]
fn a_long_queue_lets_the_oldest_waiting_go_and_never_the_one_standing() {
    let (mut book, mut reveals) = attached();
    for i in 0..(WAITING_CAP as u32 + 4) {
        reveals.take(
            &mut book,
            &tail(2 + i, vec![revealed(them(), vec![known(100 + i, 7)])]),
            &view(u64::from(i) + 2),
        );
    }
    assert_eq!(reveals.waiting(), WAITING_CAP);
    assert_eq!(
        reveals.current().map(|r| r.cards[0].id),
        Some(oid(100)),
        "the reveal standing is the first one, whatever queued behind it"
    );
    assert!(reveals.dismiss());
    assert_eq!(
        reveals.current().map(|r| r.cards[0].id),
        Some(oid(100 + 4)),
        "the three oldest waiting were let go, the newest kept"
    );
}

#[test]
fn every_card_fits_the_room_at_one_size_in_as_few_rows_as_cost_nothing() {
    let room = (900.0, 480.0);
    let gap = 10.0;
    for n in 1..=24 {
        let (width, across) = fit(n, room, gap, 240.0);
        let rows = n.div_ceil(across);
        #[allow(clippy::cast_precision_loss)]
        let (used_w, used_h) = (
            across as f32 * width + (across as f32 - 1.0) * gap,
            rows as f32 * width * 88.0 / 63.0 + (rows as f32 - 1.0) * gap,
        );
        assert!(width > 0.0, "{n} cards draw at some size");
        assert!(width <= 240.0, "never wider than asked");
        assert!(used_w <= room.0 + 0.01, "{n} cards stay inside the width");
        assert!(used_h <= room.1 + 0.01, "{n} cards stay inside the height");
    }
    assert_eq!(
        fit(1, room, gap, 240.0),
        (240.0, 1),
        "one card at the widest it may be"
    );
    assert_eq!(
        fit(3, room, gap, 240.0).1,
        3,
        "three cards that fit side by side stand in one row"
    );
}
