use super::*;
use crate::patch::diff;
use crate::patch::tests::{edit, play};
use crate::tests::{obj, view};
use baylee_core::ids::PlayerId;
use proptest::prelude::*;

fn told(sender: &mut ViewSender, reader: &mut ViewReader, view: &PlayerView) {
    let frame = sender.next(view);
    let got = reader.read(&frame).expect("a frame reads").cloned();
    assert_eq!(got.as_ref(), Some(view));
}

#[test]
fn a_stream_opens_whole_and_goes_on_in_patches() {
    let mut sender = ViewSender::default();
    let mut table = view(2);
    let first = sender.next(&table);
    assert!(matches!(
        ViewFrame::decode(&first),
        Ok(ViewFrame::Snapshot(_))
    ));
    table.seq += 1;
    table.battlefield.push(obj(1, 0));
    let second = sender.next(&table);
    assert!(matches!(
        ViewFrame::decode(&second),
        Ok(ViewFrame::Patch(_))
    ));
    assert!(second.len() < first.len());
}

/// What is written and what is read are two types, so the two are held to
/// one encoding here.
#[test]
fn what_a_sender_writes_is_what_a_frame_reads() {
    let mut table = view(2);
    table.hand.clear();
    let frame = ViewFrame::Snapshot(Box::new(table.clone()));
    assert_eq!(
        ViewFrame::decode(&ViewFrame::snapshot_of(&table)),
        Ok(frame)
    );
    let mut next = table.clone();
    next.turn += 1;
    let patch = diff(&table, &next);
    let wire = Written::Patch(&patch).encode();
    assert_eq!(
        ViewFrame::decode(&wire),
        Ok(ViewFrame::Patch(Box::new(patch)))
    );
    assert_eq!(ViewFrame::whole(&ViewFrame::snapshot_of(&next)), Some(next));
    assert_eq!(ViewFrame::whole(&wire), None, "a patch is no whole view");
    assert_eq!(ViewFrame::whole(b"not a frame"), None);
}

#[test]
fn a_sender_that_forgot_goes_whole() {
    let mut sender = ViewSender::default();
    let table = view(2);
    sender.next(&table);
    sender.forget();
    assert!(matches!(
        ViewFrame::decode(&sender.next(&table)),
        Ok(ViewFrame::Snapshot(_))
    ));
}

/// A patch between two seats' views would be one seat's cards in the other's
/// frame, so a sender handed another seat's view tells it whole.
#[test]
fn a_sender_never_patches_across_seats() {
    let mut sender = ViewSender::default();
    let mine = view(2);
    let mut theirs = view(2);
    theirs.seat = PlayerId::new(1);
    sender.next(&mine);
    assert!(matches!(
        ViewFrame::decode(&sender.next(&theirs)),
        Ok(ViewFrame::Snapshot(_))
    ));
}

#[test]
fn a_snapshot_is_where_the_next_patch_starts_from() {
    let mut sender = ViewSender::default();
    let mut reader = ViewReader::default();
    let mut table = view(2);
    told(&mut sender, &mut reader, &table);
    table.seq += 5;
    table.life_of_seat_zero(12);
    let whole = sender.snapshot(&table);
    assert_eq!(reader.read(&whole).expect("a snapshot reads"), Some(&table));
    table.seq += 1;
    table.turn += 1;
    told(&mut sender, &mut reader, &table);
}

impl PlayerView {
    fn life_of_seat_zero(&mut self, life: i32) {
        self.seats[0].life = life;
    }
}

#[test]
fn a_lost_frame_is_a_gap_and_a_snapshot_mends_it() {
    let mut sender = ViewSender::default();
    let mut reader = ViewReader::default();
    let mut table = view(2);
    told(&mut sender, &mut reader, &table);
    table.seq += 1;
    table.turn += 1;
    let _lost = sender.next(&table);
    table.seq += 1;
    table.turn += 1;
    let after = sender.next(&table);
    assert!(matches!(
        reader.read(&after),
        Err(ViewLost::Patch(PatchError::Gap { .. }))
    ));
    assert_eq!(
        reader.view().map(|held| held.turn),
        Some(1),
        "the view held stays"
    );

    table.seq += 1;
    assert_eq!(
        reader.read(&sender.next(&table)),
        Ok(None),
        "asked once: what comes meanwhile is passed over"
    );
    let whole = sender.snapshot(&table);
    assert_eq!(reader.read(&whole).expect("a snapshot reads"), Some(&table));
    table.seq += 1;
    table.turn += 1;
    told(&mut sender, &mut reader, &table);
}

#[test]
fn a_patch_with_no_view_asks_once() {
    let mut sender = ViewSender::default();
    let table = view(2);
    sender.next(&table);
    let mut next = table.clone();
    next.seq += 1;
    let patch = sender.next(&next);
    let mut reader = ViewReader::default();
    assert_eq!(reader.read(&patch), Err(ViewLost::NoBase));
    assert_eq!(reader.read(&patch), Ok(None));
}

#[test]
fn bytes_that_do_not_read_ask_once_and_change_nothing() {
    let mut sender = ViewSender::default();
    let mut reader = ViewReader::default();
    let table = view(2);
    told(&mut sender, &mut reader, &table);
    assert!(matches!(
        reader.read(b"\xff not a frame"),
        Err(ViewLost::Unreadable(_))
    ));
    assert_eq!(reader.read(b"\xff not a frame"), Ok(None));
    assert_eq!(reader.view(), Some(&table));
}

#[test]
fn empty_bytes_are_a_frame_with_no_view_in_it() {
    let mut sender = ViewSender::default();
    let mut reader = ViewReader::default();
    assert_eq!(reader.read(&[]), Ok(None), "nothing held yet");
    let table = view(2);
    told(&mut sender, &mut reader, &table);
    assert_eq!(reader.read(&[]), Ok(Some(&table)));
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Any frames lost, in any stream: a reader is either on the sender's view
    /// or knows it is not, and a snapshot always brings it back.
    #[test]
    fn a_reader_is_never_silently_wrong(
        steps in proptest::collection::vec(
            (proptest::collection::vec(edit(), 0..4), any::<bool>()),
            1..32,
        ),
    ) {
        let mut sender = ViewSender::default();
        let mut reader = ViewReader::default();
        let mut table = view(3);
        told(&mut sender, &mut reader, &table);
        let mut asked = false;
        for (edits, lost) in steps {
            for edit in &edits {
                play(&mut table, edit);
            }
            table.seq += 1;
            let frame = sender.next(&table);
            if lost {
                continue;
            }
            match reader.read(&frame) {
                Ok(Some(view)) => prop_assert_eq!(view, &table),
                Ok(None) => prop_assert!(asked, "passed over without having asked"),
                Err(_) => {
                    prop_assert!(!asked, "asked twice");
                    asked = true;
                }
            }
        }
        let whole = sender.snapshot(&table);
        prop_assert_eq!(reader.read(&whole).expect("a snapshot reads"), Some(&table));
    }

    /// Whatever comes down a socket: bytes, damaged frames, frames for another
    /// stream, in any order. A reader answers, and never panics.
    #[test]
    fn no_bytes_take_a_reader_down(
        chunks in proptest::collection::vec(
            proptest::collection::vec(any::<u8>(), 0..64),
            0..16,
        ),
        damage in proptest::collection::vec((any::<usize>(), any::<u8>()), 0..8),
    ) {
        let mut sender = ViewSender::default();
        let mut reader = ViewReader::default();
        let mut table = view(2);
        table.battlefield = (1..=6).map(|id| obj(id, 0)).collect();
        let first = sender.next(&table);
        let _ = reader.read(&first);
        for chunk in &chunks {
            let _ = reader.read(chunk);
        }
        table.seq += 1;
        table.battlefield.pop();
        let mut frame = sender.next(&table);
        for (at, byte) in damage {
            let at = at % frame.len();
            frame[at] = byte;
        }
        let _ = reader.read(&frame);
    }
}
