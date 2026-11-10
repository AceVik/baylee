//! `apply(old, diff(old, new)) == new` over recorded games, for every seat, on
//! the frames a socket at that seat is sent (`docs/proposals/view-patches.md`).
//!
//! `baylee-view` proves the same over views it generates; this is the proof on
//! views a game made, and on what a patch may carry: nothing that is not in the
//! seat's own view.

use std::collections::BTreeSet;
use std::ops::ControlFlow;

use baylee_core::ids::{ObjectId, PlayerId};
use baylee_gamehost::PlayerView;
use baylee_gamehost::replay::{Perspective, Stop, Verify, project};
use baylee_protocol::v1::envelope::Msg;
use baylee_view::{ViewFrame, ViewPatch, apply, diff, digest};

const RECORD: &[u8] = include_bytes!("../src/record/seen/acceptance-seed3.jsonl");

/// Every view a socket at `seat` is sent over the record.
fn views_of(seat: PlayerId) -> Vec<PlayerView> {
    let mut views = Vec::new();
    let projection = project(
        RECORD,
        vec!["a".to_owned(), "b".to_owned()],
        Perspective::Seat(seat),
        Verify::Stop,
        |moment| {
            for frame in moment.frames {
                if let Some(Msg::StateDelta(delta)) = frame.msg {
                    views
                        .push(ViewFrame::whole(&delta.view_frame).expect("a tape's view is whole"));
                }
            }
            ControlFlow::Continue(())
        },
    )
    .expect("the record plays");
    assert_eq!(projection.stop, Stop::End, "the record plays to its end");
    views
}

/// Every object id a view shows, in any zone and in the hand.
fn shown(view: &PlayerView) -> BTreeSet<ObjectId> {
    view.battlefield
        .iter()
        .chain(&view.stack)
        .chain(view.graveyards.iter().flatten())
        .chain(view.exile.iter().flatten())
        .chain(view.command.iter().flatten())
        .chain(&view.looking_at)
        .chain(&view.library_tops)
        .map(|object| object.id)
        .chain(view.hand.iter().map(|card| card.id))
        .collect()
}

#[test]
fn every_seat_of_a_recorded_game_rebuilds_from_patches() {
    for seat in [PlayerId::new(0), PlayerId::new(1)] {
        let views = views_of(seat);
        assert!(views.len() > 100, "a whole game: {}", views.len());
        let mut rebuilt = views[0].clone();
        for pair in views.windows(2) {
            let [old, new] = pair else { unreachable!() };
            let wire = serde_json::to_vec(&diff(old, new)).expect("a patch serializes");
            let patch: ViewPatch = serde_json::from_slice(&wire).expect("a patch reads back");
            rebuilt = apply(&rebuilt, &patch).expect("a patch applies to what the last one left");
            assert_eq!(
                serde_json::to_vec(&rebuilt).expect("a view serializes"),
                serde_json::to_vec(new).expect("a view serializes"),
                "seat {seat:?} at seq {}",
                new.seq,
            );
            assert_eq!(digest(&rebuilt), patch.digest);
        }
    }
}

/// What a patch holds is what the seat's new view holds: no object that view
/// does not show, so it can say no more than the view does (CR 400.2).
#[test]
fn a_patch_names_only_what_the_seat_can_see() {
    for seat in [PlayerId::new(0), PlayerId::new(1)] {
        for pair in views_of(seat).windows(2) {
            let [old, new] = pair else { unreachable!() };
            let patch = diff(old, new);
            let visible = shown(new);
            for (_, edit) in &patch.zones {
                for object in &edit.upserts {
                    assert!(visible.contains(&object.id), "{:?}", object.id);
                }
            }
            for card in patch.hand.iter().flat_map(|edit| &edit.upserts) {
                assert!(new.hand.iter().any(|held| held == card));
            }
        }
    }
}

/// A card in one seat's hand that the other seat's view does not show at that
/// moment is in no byte of the patch the other seat is sent then.
#[test]
fn one_seats_hidden_hand_is_in_no_patch_of_the_other() {
    let mine = views_of(PlayerId::new(0));
    let theirs = views_of(PlayerId::new(1));
    let mut held_back = 0;
    for pair in theirs.windows(2) {
        let [old, new] = pair else { unreachable!() };
        let Some(at_that_moment) = mine.iter().find(|view| view.seq == new.seq) else {
            continue;
        };
        let patch = serde_json::to_string(&diff(old, new)).expect("a patch serializes");
        let shown = serde_json::to_string(new).expect("a view serializes");
        for card in &at_that_moment.hand {
            if !shown.contains(&card.name) {
                held_back += 1;
                assert!(
                    !patch.contains(&card.name),
                    "{} reached seat 1 at seq {}",
                    card.name,
                    new.seq,
                );
            }
        }
    }
    assert!(held_back > 100, "the game hid cards: {held_back}");
}
