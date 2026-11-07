use super::*;
use crate::CardIdentity;
use crate::tests::{obj, view};
use baylee_core::ids::{CardIndex, PrintRef};

fn plain(view: &PlayerView) -> Vec<u8> {
    serde_json::to_vec(view).expect("a view serializes")
}

fn written(objects: &mut WrittenObjects, view: &PlayerView) -> Vec<u8> {
    let mut json = Vec::new();
    objects.write(&mut json, view).expect("a view writes");
    json
}

/// A view with an object in every zone that writes through the table.
fn a_full_view() -> PlayerView {
    let mut view = view(2);
    view.battlefield = vec![obj(1, 0), obj(2, 1)];
    view.stack = vec![obj(3, 0)];
    view.graveyards = vec![vec![obj(4, 0)], vec![obj(5, 1)]];
    view.exile = vec![vec![], vec![obj(6, 1)]];
    view.command = vec![vec![obj(7, 0)], vec![]];
    view.looking_at = vec![obj(8, 0)];
    view.library_tops = vec![obj(9, 1)];
    view
}

fn a_card(index: u32) -> CardIdentity {
    CardIdentity {
        index: CardIndex::new(index),
        print: PrintRef::new(0),
        face: 0,
    }
}

#[test]
fn a_written_view_reads_as_a_plain_one_each_time_it_is_written() {
    let view = a_full_view();
    let mut objects = WrittenObjects::default();
    assert_eq!(written(&mut objects, &view), plain(&view));
    assert_eq!(objects.objects.len(), 9);
    assert_eq!(written(&mut objects, &view), plain(&view), "from the table");
}

#[test]
fn the_second_writing_of_an_object_comes_from_the_table() {
    let view = a_full_view();
    let mut objects = WrittenObjects::default();
    written(&mut objects, &view);
    // Bytes no serializer would make of this object, to see them come back.
    let marked = RawValue::from_string("\"marked\"".to_owned()).expect("a JSON string");
    objects
        .objects
        .get_mut(&view.battlefield[0].id)
        .expect("it was written")
        .json = marked;
    let json = String::from_utf8(written(&mut objects, &view)).expect("JSON");
    assert!(json.contains("\"battlefield\":[\"marked\","), "{json}");
}

#[test]
fn an_object_that_changed_is_written_again() {
    let mut view = a_full_view();
    let mut objects = WrittenObjects::default();
    written(&mut objects, &view);
    view.battlefield[0].damage = 2;
    view.graveyards[1][0].name = "Elf".into();
    view.stack[0].power = None;
    assert_eq!(written(&mut objects, &view), plain(&view));
    assert_eq!(written(&mut objects, &view), plain(&view));
}

#[test]
fn an_object_that_moved_zone_is_read_where_it_went() {
    let mut view = a_full_view();
    let mut objects = WrittenObjects::default();
    written(&mut objects, &view);
    let moved = view.battlefield.remove(0);
    view.graveyards[0].push(moved);
    assert_eq!(written(&mut objects, &view), plain(&view));
}

/// What an object shows depends on who looks (CR 400.2): the same handle is a
/// card to its controller and a blank to the table, and neither reads the other.
#[test]
fn a_seat_never_reads_what_another_seats_view_showed_of_an_object() {
    let mut known = a_full_view();
    known.battlefield[0].card = Some(a_card(41));
    known.battlefield[0].name = "Hidden Name".into();
    let mut blank = a_full_view();
    blank.battlefield[0].name = "Face-down".into();

    let (mut theirs, mut mine) = (WrittenObjects::default(), WrittenObjects::default());
    for _ in 0..2 {
        assert_eq!(written(&mut mine, &known), plain(&known));
        let json = written(&mut theirs, &blank);
        assert_eq!(json, plain(&blank));
        let json = String::from_utf8(json).expect("JSON");
        assert!(!json.contains("Hidden Name"), "{json}");
    }
    // A plain view written right after takes nothing from either.
    assert_eq!(plain(&blank), plain(&blank.clone()));
    assert!(WRITING.with_borrow(Option::is_none));
}

/// Even handed the wrong seat's table, equality is what lets a byte through: the
/// blank object is not the object those bytes were written from.
#[test]
fn a_table_gives_up_only_what_is_equal() {
    let mut known = a_full_view();
    known.battlefield[0].card = Some(a_card(41));
    known.battlefield[0].name = "Hidden Name".into();
    let mut blank = a_full_view();
    blank.battlefield[0].name = "Face-down".into();

    let mut objects = WrittenObjects::default();
    written(&mut objects, &known);
    assert_eq!(written(&mut objects, &blank), plain(&blank));
}

#[test]
fn what_left_the_view_is_not_kept() {
    let mut view = a_full_view();
    let mut objects = WrittenObjects::default();
    written(&mut objects, &view);
    view.battlefield.clear();
    view.graveyards[0].clear();
    written(&mut objects, &view);
    assert_eq!(objects.objects.len(), 6);
    let gone = obj(1, 0).id;
    assert!(!objects.objects.contains_key(&gone));
}

/// A writer that takes nothing.
struct Refusing;

impl std::io::Write for Refusing {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("refused"))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn a_refused_write_leaves_nothing_in_the_thread_and_loses_no_seat() {
    let view = a_full_view();
    let mut objects = WrittenObjects::default();
    written(&mut objects, &view);
    assert!(objects.write(Refusing, &view).is_err());
    assert!(WRITING.with_borrow(Option::is_none));
    assert_eq!(written(&mut objects, &view), plain(&view));
}
