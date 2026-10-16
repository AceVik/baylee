//! Zone animation must follow public identity, not allocation slots or dialog rows.
use super::*;
use crate::test_support::{ViewBuilder, printed};

#[test]
fn recycled_slots_leave_and_arrive_as_distinct_objects() {
    let old = printed(7, 0, "Old permanent", 1);
    let mut new = old.clone();
    new.id = ObjectId::new(7, 1);
    let mut before = ViewBuilder::new(2)
        .with_battlefield(0, vec![old.clone()])
        .build();
    let mut after = ViewBuilder::new(2)
        .with_battlefield(0, vec![new.clone()])
        .build();
    before.seq = 10;
    after.seq = 11;
    let mut tracker = Tracker::default();
    tracker.observe(&before);
    let moves = tracker.observe(&after);
    assert_eq!(moves.len(), 2);
    assert!(moves.contains(&Move {
        object: old.id,
        from: Some(Place::Battlefield),
        to: None
    }));
    assert!(moves.contains(&Move {
        object: new.id,
        from: None,
        to: Some(Place::Battlefield)
    }));
    assert!(moves.iter().all(|movement| movement.passage().is_none()));
}

#[test]
fn dialog_reveals_neither_hide_departures_nor_create_arrivals() {
    let object = printed(7, 0, "Shown card", 1);
    let mut shown = ViewBuilder::new(2).build();
    shown.seq = 2;
    shown.looking_at = vec![object.clone()];
    let mut tracker = Tracker::default();
    assert!(tracker.observe(&shown).is_empty());
    let mut standing = ViewBuilder::new(2)
        .with_battlefield(0, vec![object.clone()])
        .build();
    standing.seq = 3;
    assert_eq!(
        tracker.observe(&standing),
        vec![Move {
            object: object.id,
            from: None,
            to: Some(Place::Battlefield),
        }]
    );
    shown.seq = 4;
    assert_eq!(
        tracker.observe(&shown),
        vec![Move {
            object: object.id,
            from: Some(Place::Battlefield),
            to: None,
        }]
    );
}

#[test]
fn pile_index_wins_over_owner_and_controller_after_a_control_change() {
    for destination in 0..4 {
        let object = printed(7, 1, "Borrowed permanent", 1);
        let before = ViewBuilder::new(4)
            .with_battlefield(1, vec![object.clone()])
            .build();
        let mut after = ViewBuilder::new(4).build();
        after.seq = before.seq + 1;
        after.graveyards[usize::from(destination)] = vec![object.clone()];
        let mut tracker = Tracker::default();
        tracker.observe(&before);
        assert_eq!(
            tracker.observe(&after),
            vec![Move {
                object: object.id,
                from: Some(Place::Battlefield),
                to: Some(Place::Graveyard(PlayerId::new(destination))),
            }]
        );
    }
}

#[test]
fn duplicate_zone_rows_have_stable_battlefield_precedence() {
    let object = printed(7, 0, "Duplicate row", 1);
    let mut view = ViewBuilder::new(2)
        .with_battlefield(0, vec![object.clone()])
        .build();
    view.stack = vec![object.clone()];
    view.graveyards[0] = vec![object.clone()];
    view.exile[1] = vec![object.clone()];
    view.command[0] = vec![object.clone()];
    let mut tracker = Tracker::default();
    assert_eq!(
        tracker.observe(&view),
        vec![Move {
            object: object.id,
            from: None,
            to: Some(Place::Battlefield),
        }]
    );
    view.seq += 1;
    assert!(tracker.observe(&view).is_empty());
    view.battlefield.clear();
    view.seq += 1;
    assert_eq!(
        tracker.observe(&view),
        vec![Move {
            object: object.id,
            from: Some(Place::Battlefield),
            to: Some(Place::Stack),
        }]
    );
}

#[test]
fn simultaneous_departures_are_sorted_independently_of_view_order() {
    let cards: Vec<_> = [9, 2, 7, 1]
        .into_iter()
        .map(|id| printed(id, 0, "Fixture", 1))
        .collect();
    let mut expected = cards
        .iter()
        .map(|card| Move {
            object: card.id,
            from: Some(Place::Battlefield),
            to: Some(Place::Graveyard(PlayerId::new(0))),
        })
        .collect::<Vec<_>>();
    expected.sort_by_key(|movement| movement.object);
    for offset in 0..cards.len() {
        let mut order = cards.clone();
        order.rotate_left(offset);
        let before = ViewBuilder::new(2)
            .with_battlefield(0, order.clone())
            .build();
        order.reverse();
        let mut after = ViewBuilder::new(2).with_graveyard(0, order).build();
        after.seq = before.seq + 1;
        let mut tracker = Tracker::default();
        tracker.observe(&before);
        assert_eq!(tracker.observe(&after), expected);
        assert!(tracker.observe(&after).is_empty());
    }
}

#[test]
fn clear_accepts_the_same_sequence_in_a_new_game() {
    let view = ViewBuilder::new(2)
        .with_battlefield(0, vec![printed(1, 0, "Fixture", 1)])
        .build();
    let mut tracker = Tracker::default();
    let initial = tracker.observe(&view);
    assert_eq!(initial.len(), 1);
    assert!(tracker.observe(&view).is_empty());
    tracker.clear();
    assert_eq!(tracker.observe(&view), initial);
}
