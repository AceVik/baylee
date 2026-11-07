use super::digest::fingerprint;
use super::*;
use crate::tests::{obj, view};
use crate::turn::{Phase, Step};
use crate::{CardIdentity, CombatView, DamageSourceView, PlayerView};
use baylee_core::color::ColorSet;
use baylee_core::ids::{CardIndex, PrintRef};
use baylee_core::types::TypeSet;
use proptest::prelude::*;

fn hand_card(id: u32) -> HandObject {
    HandObject {
        id: ObjectId::new(id, 0),
        card: CardIdentity {
            index: CardIndex::new(id),
            print: PrintRef::new(u16::try_from(id).expect("a small id")),
            face: 0,
        },
        name: format!("Card {id}"),
        mana_value: id % 7,
        colors: ColorSet::default(),
        types: TypeSet::CREATURE,
        commander: false,
    }
}

/// The promise of the module, through JSON as the wire would carry it.
fn holds(old: &PlayerView, new: &PlayerView) -> ViewPatch {
    let patch = diff(old, new);
    let wire = serde_json::to_vec(&patch).expect("a patch serializes");
    let read: ViewPatch = serde_json::from_slice(&wire).expect("a patch reads back");
    assert_eq!(read, patch, "a patch survives its encoding");
    let rebuilt = apply(old, &read).expect("a patch applies to its own base");
    assert_eq!(&rebuilt, new);
    assert_eq!(
        serde_json::to_vec(&rebuilt).expect("a view serializes"),
        serde_json::to_vec(new).expect("a view serializes"),
    );
    patch
}

#[test]
fn a_view_against_itself_is_an_empty_patch() {
    let table = view(2);
    let patch = holds(&table, &table);
    assert!(patch.is_empty());
}

#[test]
fn only_what_changed_is_carried() {
    let mut old = view(2);
    old.battlefield = (1..=20).map(|id| obj(id, 0)).collect();
    let mut new = old.clone();
    new.seq += 1;
    new.battlefield[7].damage = 2;
    let patch = holds(&old, &new);
    let [(Zone::Battlefield, edit)] = &patch.zones[..] else {
        panic!("one zone changed: {patch:?}");
    };
    assert_eq!(edit.upserts, vec![new.battlefield[7].clone()]);
    assert!(edit.removed.is_empty());
    assert_eq!(edit.order, None, "nothing moved");
    assert!(patch.fields.is_empty() && patch.seats.is_empty());
}

#[test]
fn a_card_going_to_the_graveyard_is_an_id_and_an_object() {
    let mut old = view(2);
    old.battlefield = (1..=5).map(|id| obj(id, 0)).collect();
    let mut new = old.clone();
    let dead = new.battlefield.remove(2);
    new.graveyards[0].push(dead.clone());
    let patch = holds(&old, &new);
    let [(Zone::Battlefield, left), (Zone::Graveyard(0), arrived)] = &patch.zones[..] else {
        panic!("two zones changed: {patch:?}");
    };
    assert_eq!(left.removed, vec![dead.id]);
    assert_eq!(left.order, None, "the rest closes up by itself");
    assert_eq!(arrived.upserts, vec![dead]);
}

#[test]
fn a_changed_order_is_said_and_an_appended_one_is_not() {
    let mut old = view(2);
    old.stack = (1..=3).map(|id| obj(id, 0)).collect();
    let mut new = old.clone();
    new.stack.reverse();
    let reversed = holds(&old, &new);
    assert!(reversed.zones[0].1.order.is_some());
    assert!(reversed.zones[0].1.upserts.is_empty(), "no object changed");

    let mut new = old.clone();
    new.stack.push(obj(9, 1));
    assert_eq!(holds(&old, &new).zones[0].1.order, None);

    let mut new = old.clone();
    new.stack.insert(0, obj(9, 1));
    assert!(
        holds(&old, &new).zones[0].1.order.is_some(),
        "a new object anywhere but the end needs its place said"
    );
}

/// The same id in two zones is an object shown twice, with what each zone says
/// of it (`PlayerView::object`): an edit is per zone, so neither disturbs the
/// other.
#[test]
fn one_id_in_two_zones_stays_two_objects() {
    let mut old = view(2);
    old.battlefield = vec![obj(4, 0)];
    old.looking_at = vec![obj(4, 0)];
    let mut new = old.clone();
    new.looking_at[0].name = "Shown".into();
    let patch = holds(&old, &new);
    assert_eq!(patch.zones.len(), 1);
    assert_eq!(patch.zones[0].0, Zone::LookingAt);
}

#[test]
fn a_zone_holding_an_id_twice_goes_whole() {
    let mut old = view(2);
    old.stack = vec![obj(1, 0), obj(1, 1)];
    let mut new = old.clone();
    new.stack[1].damage = 1;
    let patch = holds(&old, &new);
    assert!(patch.zones.is_empty());
    assert!(matches!(
        patch.fields[..],
        [ViewField::Zone(Zone::Stack, _)]
    ));

    let mut twice = view(2);
    twice.hand = vec![hand_card(1), hand_card(1)];
    let mut once = twice.clone();
    once.hand.pop();
    for (old, new) in [(&twice, &once), (&once, &twice)] {
        let patch = holds(old, new);
        assert!(patch.hand.is_none());
        assert!(matches!(patch.fields[..], [ViewField::Hand(_)]));
    }
}

#[test]
fn seat_lines_go_one_by_one_and_whole_when_the_seats_differ() {
    let old = view(4);
    let mut new = old.clone();
    new.seats[2].life = 3;
    let patch = holds(&old, &new);
    assert_eq!(patch.seats, vec![new.seats[2].clone()]);

    let fewer = view(3);
    let patch = holds(&old, &fewer);
    assert!(patch.seats.is_empty());
    assert!(
        patch
            .fields
            .iter()
            .any(|field| matches!(field, ViewField::Seats(_)))
    );
    assert!(
        patch
            .fields
            .iter()
            .any(|field| matches!(field, ViewField::Graveyards(_)))
    );
}

#[test]
fn a_field_goes_alone() {
    let old = view(2);
    let mut new = old.clone();
    new.turn = 9;
    new.phase = Phase::Combat;
    new.step = Step::DeclareAttackers;
    new.combat = CombatView::default();
    let patch = holds(&old, &new);
    assert_eq!(
        patch.fields,
        vec![
            ViewField::Turn(9),
            ViewField::Phase(Phase::Combat),
            ViewField::Step(Step::DeclareAttackers),
        ]
    );
}

/// Hostile patches: each refused by name, and the base untouched.
mod refused {
    use super::*;

    fn patched(
        old: &PlayerView,
        edit: impl FnOnce(&mut ViewPatch),
    ) -> Result<PlayerView, PatchError> {
        let mut new = old.clone();
        new.seq += 1;
        new.turn += 1;
        let mut patch = diff(old, &new);
        edit(&mut patch);
        apply(old, &patch)
    }

    fn table() -> PlayerView {
        let mut table = view(2);
        table.battlefield = (1..=4).map(|id| obj(id, 0)).collect();
        table.hand = vec![hand_card(10), hand_card(11)];
        table
    }

    fn edit(removed: Vec<ObjectId>, order: Option<Vec<ObjectId>>) -> ZoneEdit<PublicObject> {
        ZoneEdit {
            removed,
            upserts: Vec::new(),
            order,
        }
    }

    /// One seat's patch is that seat's cards: it lands on no other's view.
    #[test]
    fn a_patch_of_another_seat_is_refused() {
        let other = PlayerId::new(1);
        assert_eq!(
            patched(&table(), |patch| patch.seat = other),
            Err(PatchError::OtherSeat(other))
        );
    }

    #[test]
    fn a_patch_for_another_view_is_a_gap() {
        let old = table();
        assert_eq!(
            patched(&old, |patch| patch.base += 1),
            Err(PatchError::Gap {
                have: old.seq,
                base: old.seq + 1
            })
        );
    }

    #[test]
    fn a_removal_of_an_unknown_id_is_an_error() {
        let unknown = ObjectId::new(99, 0);
        assert_eq!(
            patched(&table(), |patch| patch
                .zones
                .push((Zone::Battlefield, edit(vec![unknown], None)))),
            Err(PatchError::UnknownObject(unknown))
        );
    }

    #[test]
    fn a_removal_named_twice_is_an_error() {
        let id = ObjectId::new(1, 0);
        assert_eq!(
            patched(&table(), |patch| patch
                .zones
                .push((Zone::Battlefield, edit(vec![id, id], None)))),
            Err(PatchError::UnknownObject(id))
        );
    }

    #[test]
    fn a_zone_the_view_lacks_is_an_error() {
        assert_eq!(
            patched(&table(), |patch| patch
                .zones
                .push((Zone::Graveyard(200), edit(Vec::new(), None)))),
            Err(PatchError::NoZone(Zone::Graveyard(200)))
        );
        assert_eq!(
            patched(&table(), |patch| patch
                .fields
                .push(ViewField::Zone(Zone::Exile(9), Vec::new()))),
            Err(PatchError::NoZone(Zone::Exile(9)))
        );
    }

    #[test]
    fn an_order_that_is_no_order_of_the_zone_is_an_error() {
        let ids = |list: &[u32]| list.iter().map(|&id| ObjectId::new(id, 0)).collect();
        let short = patched(&table(), |patch| {
            patch
                .zones
                .push((Zone::Battlefield, edit(Vec::new(), Some(ids(&[1, 2])))));
        });
        assert_eq!(short, Err(PatchError::BadOrder));
        let twice = patched(&table(), |patch| {
            patch.zones.push((
                Zone::Battlefield,
                edit(Vec::new(), Some(ids(&[1, 1, 2, 3]))),
            ));
        });
        assert_eq!(twice, Err(PatchError::UnknownObject(ObjectId::new(1, 0))));
        let stranger = patched(&table(), |patch| {
            patch.zones.push((
                Zone::Battlefield,
                edit(Vec::new(), Some(ids(&[1, 2, 3, 99]))),
            ));
        });
        assert_eq!(
            stranger,
            Err(PatchError::UnknownObject(ObjectId::new(99, 0)))
        );
    }

    #[test]
    fn a_seat_the_view_lacks_is_an_error() {
        let old = table();
        let mut stranger = old.seats[0].clone();
        stranger.player = PlayerId::new(7);
        assert_eq!(
            patched(&old, |patch| patch.seats.push(stranger)),
            Err(PatchError::NoSeat(PlayerId::new(7)))
        );
    }

    #[test]
    fn a_wrong_digest_is_an_error() {
        let old = table();
        let result = patched(&old, |patch| patch.digest = ViewDigest(patch.digest.0 ^ 1));
        assert!(matches!(result, Err(PatchError::Digest { .. })));
    }

    /// A patch that applies cleanly and leaves another view than it says.
    #[test]
    fn a_lie_about_the_contents_is_caught_by_the_digest() {
        let old = table();
        let result = patched(&old, |patch| patch.fields.push(ViewField::Turn(77)));
        assert!(matches!(result, Err(PatchError::Digest { .. })));
    }

    #[test]
    fn a_base_with_an_id_twice_refuses_an_edit() {
        let mut old = table();
        old.battlefield.push(obj(1, 1));
        let result = patched(&old, |patch| {
            patch
                .zones
                .push((Zone::Battlefield, edit(Vec::new(), None)));
        });
        assert_eq!(result, Err(PatchError::Unkeyed(ObjectId::new(1, 0))));
    }
}

/// Fixed so that both ends of a socket read the same digest wherever they were
/// built: the numbers below are this encoding. A change here is a protocol
/// change, and neither of them may move with a compiler or a target.
mod golden {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize)]
    enum Kind {
        Unit,
        New(u8),
        Pair(u8, u8),
        Named { a: u8 },
    }

    #[derive(Serialize)]
    struct Shape {
        flag: bool,
        small: i16,
        wide: u128,
        text: &'static str,
        none: Option<u32>,
        some: Option<u32>,
        list: Vec<u16>,
        kinds: [Kind; 4],
        #[serde(skip_serializing_if = "Option::is_none")]
        left_out: Option<u8>,
    }

    fn shape() -> Shape {
        Shape {
            flag: true,
            small: -2,
            wide: u128::MAX - 1,
            text: "Llanowar Elves, longer than one word",
            none: None,
            some: Some(7),
            list: vec![1, 2, 3],
            kinds: [
                Kind::Unit,
                Kind::New(1),
                Kind::Pair(2, 3),
                Kind::Named { a: 4 },
            ],
            left_out: None,
        }
    }

    #[test]
    fn the_digest_of_a_fixed_value_does_not_move() {
        assert_eq!(fingerprint(&shape()), 7_277_142_247_575_331_905);
    }

    #[test]
    fn empty_values_differ_by_shape() {
        assert_ne!(fingerprint(&Vec::<u8>::new()), fingerprint(&None::<u8>));
        assert_ne!(fingerprint(&vec![0_u8]), fingerprint(&vec![0_u8, 0]));
        assert_ne!(fingerprint(&""), fingerprint(&"\0"));
    }

    /// Which field was left out is part of the value, or a field written at
    /// one place would read like its neighbour.
    #[test]
    fn a_skipped_field_is_not_its_neighbour() {
        #[derive(Serialize)]
        struct Two {
            #[serde(skip_serializing_if = "Option::is_none")]
            a: Option<u8>,
            #[serde(skip_serializing_if = "Option::is_none")]
            b: Option<u8>,
        }
        assert_ne!(
            fingerprint(&Two {
                a: Some(1),
                b: None
            }),
            fingerprint(&Two {
                a: None,
                b: Some(1)
            })
        );
    }
}

#[test]
fn the_digest_notices_every_kind_of_change() {
    let mut table = view(2);
    table.battlefield = vec![obj(1, 0), obj(2, 1)];
    let base = digest(&table);
    assert_eq!(base, digest(&table.clone()), "it reads the value alone");

    let mut seq = table.clone();
    seq.seq += 1;
    assert_ne!(digest(&seq), base);

    let mut swapped = table.clone();
    swapped.battlefield.swap(0, 1);
    assert_ne!(digest(&swapped), base);

    let mut moved = table.clone();
    let object = moved.battlefield.pop().expect("two objects");
    moved.graveyards[1].push(object);
    assert_ne!(digest(&moved), base);

    let mut hurt = table.clone();
    hurt.battlefield[1].damage = 1;
    assert_ne!(digest(&hurt), base);
}

/// One edit to a view; [`arbitrary_view`] plays a list of them.
#[derive(Clone, Debug)]
pub(super) enum Edit {
    Enter { zone: u8, id: u32, at: usize },
    Leave { zone: u8, at: usize },
    Change { zone: u8, at: usize, by: u16 },
    Move { from: u8, to: u8, at: usize },
    Swap { zone: u8, a: usize, b: usize },
    Duplicate { zone: u8, at: usize },
    Draw { id: u32 },
    Discard { at: usize },
    Life { seat: usize, by: i32 },
    Turn(u32),
    Seq(u64),
    Reseat(u8),
}

fn zone_of(view: &mut PlayerView, zone: u8) -> &mut Vec<PublicObject> {
    let seats = view.graveyards.len().max(1);
    let seat = usize::from(zone / 7) % seats;
    match zone % 7 {
        0 => &mut view.battlefield,
        1 => &mut view.stack,
        2 => &mut view.graveyards[seat],
        3 => &mut view.exile[seat],
        4 => &mut view.command[seat],
        5 => &mut view.looking_at,
        _ => &mut view.library_tops,
    }
}

pub(super) fn play(view: &mut PlayerView, edit: &Edit) {
    match *edit {
        Edit::Enter { zone, id, at } => {
            let list = zone_of(view, zone);
            list.insert(at % (list.len() + 1), obj(id, 0));
        }
        Edit::Leave { zone, at } => {
            let list = zone_of(view, zone);
            if !list.is_empty() {
                list.remove(at % list.len());
            }
        }
        Edit::Change { zone, at, by } => {
            let list = zone_of(view, zone);
            if !list.is_empty() {
                let at = at % list.len();
                list[at].damage = by;
                list[at].summoning_sick = by % 2 == 0;
            }
        }
        Edit::Move { from, to, at } => {
            let list = zone_of(view, from);
            if !list.is_empty() {
                let object = list.remove(at % list.len());
                zone_of(view, to).push(object);
            }
        }
        Edit::Swap { zone, a, b } => {
            let list = zone_of(view, zone);
            if !list.is_empty() {
                let len = list.len();
                list.swap(a % len, b % len);
            }
        }
        Edit::Duplicate { zone, at } => {
            let list = zone_of(view, zone);
            if !list.is_empty() {
                let object = list[at % list.len()].clone();
                list.push(object);
            }
        }
        Edit::Draw { id } => view.hand.push(hand_card(id)),
        Edit::Discard { at } => {
            if !view.hand.is_empty() {
                let at = at % view.hand.len();
                view.hand.remove(at);
            }
        }
        Edit::Life { seat, by } => {
            let at = seat % view.seats.len();
            view.seats[at].life = by;
        }
        Edit::Turn(turn) => view.turn = turn,
        Edit::Seq(seq) => view.seq = seq,
        Edit::Reseat(seats) => {
            let keep = view.battlefield.clone();
            *view = crate::tests::view(seats);
            view.battlefield = keep;
        }
    }
}

pub(super) fn edit() -> impl Strategy<Value = Edit> {
    prop_oneof![
        8 => (0..28_u8, 1..64_u32, any::<usize>())
            .prop_map(|(zone, id, at)| Edit::Enter { zone, id, at }),
        4 => (0..28_u8, any::<usize>()).prop_map(|(zone, at)| Edit::Leave { zone, at }),
        8 => (0..28_u8, any::<usize>(), any::<u16>())
            .prop_map(|(zone, at, by)| Edit::Change { zone, at, by }),
        4 => (0..28_u8, 0..28_u8, any::<usize>())
            .prop_map(|(from, to, at)| Edit::Move { from, to, at }),
        3 => (0..28_u8, any::<usize>(), any::<usize>())
            .prop_map(|(zone, a, b)| Edit::Swap { zone, a, b }),
        1 => (0..28_u8, any::<usize>()).prop_map(|(zone, at)| Edit::Duplicate { zone, at }),
        3 => (1..64_u32).prop_map(|id| Edit::Draw { id }),
        2 => any::<usize>().prop_map(|at| Edit::Discard { at }),
        3 => (any::<usize>(), any::<i32>()).prop_map(|(seat, by)| Edit::Life { seat, by }),
        2 => any::<u32>().prop_map(Edit::Turn),
        2 => any::<u64>().prop_map(Edit::Seq),
        1 => (1..5_u8).prop_map(Edit::Reseat),
    ]
}

/// A view reached by a list of edits from an empty table of `seats`.
fn played(seats: u8, edits: &[Edit]) -> PlayerView {
    let mut table = view(seats);
    for edit in edits {
        play(&mut table, edit);
    }
    table
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    /// Any two views: the second one edited on from the first, or two with
    /// nothing in common.
    #[test]
    fn apply_undoes_diff(
        seats in 1..5_u8,
        before in proptest::collection::vec(edit(), 0..40),
        after in proptest::collection::vec(edit(), 0..40),
        related in any::<bool>(),
    ) {
        let old = played(seats, &before);
        let mut new = if related { old.clone() } else { view(seats) };
        for edit in &after {
            play(&mut new, edit);
        }
        holds(&old, &new);
    }

    /// A whole stream of views, each patch applied to what the last one left
    /// rather than to what the sender holds.
    #[test]
    fn a_stream_of_patches_stays_the_stream_of_views(
        seats in 1..5_u8,
        steps in proptest::collection::vec(proptest::collection::vec(edit(), 0..6), 1..24),
    ) {
        let mut sent = view(seats);
        let mut rebuilt = sent.clone();
        for step in &steps {
            let last = sent.clone();
            for edit in step {
                play(&mut sent, edit);
            }
            rebuilt = apply(&rebuilt, &diff(&last, &sent)).expect("a stream applies");
            prop_assert_eq!(&rebuilt, &sent);
        }
    }

    /// A patch with any of its bytes changed either fails to read, is refused,
    /// or still leaves the view its digest says. Never a panic.
    #[test]
    fn a_damaged_patch_never_panics(
        before in proptest::collection::vec(edit(), 0..24),
        after in proptest::collection::vec(edit(), 1..24),
        damage in proptest::collection::vec((any::<usize>(), any::<u8>()), 1..8),
    ) {
        let old = played(3, &before);
        let mut new = old.clone();
        for edit in &after {
            play(&mut new, edit);
        }
        let mut wire = serde_json::to_vec(&diff(&old, &new)).expect("a patch serializes");
        for (at, byte) in damage {
            let at = at % wire.len();
            wire[at] = byte;
        }
        if let Ok(patch) = serde_json::from_slice::<ViewPatch>(&wire)
            && let Ok(view) = apply(&old, &patch)
        {
            prop_assert_eq!(digest(&view), patch.digest);
        }
    }

    /// A patch built by hand from nothing: no relation to the view at all.
    #[test]
    fn a_made_up_patch_never_panics(
        before in proptest::collection::vec(edit(), 0..24),
        removed in proptest::collection::vec(1..64_u32, 0..12),
        order in proptest::option::of(proptest::collection::vec(1..64_u32, 0..12)),
        upserts in proptest::collection::vec(1..64_u32, 0..12),
        zone in 0..=u8::MAX,
    ) {
        let old = played(3, &before);
        let edit = ZoneEdit {
            removed: removed.iter().map(|&id| ObjectId::new(id, 0)).collect(),
            upserts: upserts.iter().map(|&id| obj(id, 0)).collect(),
            order: order.map(|ids| ids.iter().map(|&id| ObjectId::new(id, 0)).collect()),
        };
        let mut patch = diff(&old, &old);
        patch.zones.push((Zone::Graveyard(zone), edit.clone()));
        patch.zones.push((Zone::Battlefield, edit));
        let _ = apply(&old, &patch);
    }
}

/// What a list edit carries, for the lists a game makes.
mod lists {
    use super::*;

    fn added(old: &[u8], new: &[u8]) -> Vec<u8> {
        let edit = ListEdit::of(old, new);
        assert_eq!(edit.applied(old).expect("an edit applies"), new);
        edit.added
    }

    #[test]
    fn a_list_grown_at_the_end_carries_the_new_entry() {
        assert_eq!(added(&[1, 2, 3], &[1, 2, 3, 4]), [4]);
    }

    #[test]
    fn a_list_changed_in_the_middle_carries_the_middle() {
        assert_eq!(added(&[1, 2, 3, 4], &[1, 9, 3, 4]), [9]);
        assert_eq!(added(&[1, 2, 3, 4], &[1, 4]), [] as [u8; 0]);
    }

    #[test]
    fn a_window_that_slid_carries_what_came_in() {
        assert_eq!(added(&[1, 2, 3, 4], &[2, 3, 4, 5]), [5]);
        assert_eq!(added(&[1, 2, 3, 4], &[3, 4, 5, 6]), [5, 6]);
    }

    #[test]
    fn unrelated_lists_go_whole() {
        assert_eq!(added(&[1, 2], &[3, 4, 5]), [3, 4, 5]);
        assert_eq!(added(&[], &[3]), [3]);
        assert_eq!(added(&[3], &[]), [] as [u8; 0]);
    }

    #[test]
    fn repeated_entries_are_not_confused() {
        assert_eq!(added(&[1, 1, 1], &[1, 1]), [] as [u8; 0]);
        assert_eq!(added(&[1, 2, 1, 2], &[1, 2, 1, 2, 1]), [1]);
        added(&[2, 1, 2, 1, 3], &[2, 1, 3, 2, 1]);
    }

    #[test]
    fn an_edit_that_keeps_more_than_the_list_is_refused() {
        let edit = |dropped, kept, tail| ListEdit::<u8> {
            dropped,
            kept,
            added: Vec::new(),
            tail,
        };
        for bad in [
            edit(0, 4, 0),
            edit(4, 0, 0),
            edit(0, 0, 4),
            edit(u32::MAX, u32::MAX, 0),
            edit(1, u32::MAX, u32::MAX),
        ] {
            assert_eq!(bad.applied(&[1, 2, 3]), Err(PatchError::BadList), "{bad:?}");
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(2048))]

        #[test]
        fn any_list_edit_leaves_the_new_list(
            old in proptest::collection::vec(0..4_u8, 0..24),
            new in proptest::collection::vec(0..4_u8, 0..24),
        ) {
            let edit = ListEdit::of(&old, &new);
            prop_assert_eq!(edit.applied(&old), Ok(new.clone()));
            prop_assert!(edit.added.len() <= new.len());
        }

        #[test]
        fn any_made_up_edit_is_refused_or_applied(
            old in proptest::collection::vec(any::<u8>(), 0..24),
            dropped in any::<u32>(),
            kept in any::<u32>(),
            tail in any::<u32>(),
        ) {
            let edit = ListEdit::<u8> { dropped: dropped % 32, kept: kept % 32, added: vec![], tail: tail % 32 };
            let _ = edit.applied(&old);
            let wild = ListEdit::<u8> { dropped, kept, added: vec![], tail };
            let _ = wild.applied(&old);
        }
    }
}

/// `target_objects` and `policy_acts` go as list edits through a whole patch.
#[test]
fn the_edited_lists_survive_a_patch() {
    use crate::log::{LogAbility, PolicyAct, PolicyAnswer};
    use baylee_core::ids::DamageSourceRef;

    let act = |number: u32| PolicyAct {
        number,
        ability: LogAbility {
            ability: None,
            text: None,
            rules: None,
        },
        answer: PolicyAnswer::Passed,
    };
    let mut old = view(2);
    old.policy_acts = (0..16).map(act).collect();
    let mut new = old.clone();
    new.policy_acts.remove(0);
    new.policy_acts.push(act(99));
    let patch = holds(&old, &new);
    let [ViewField::PolicyActs(edit)] = &patch.fields[..] else {
        panic!("one field changed: {patch:?}");
    };
    assert_eq!(edit.added.len(), 1, "a window that slid is one new entry");

    let target = |at: u32| DamageSourceView {
        source: DamageSourceRef {
            object: ObjectId::new(at, 0),
            version: 0,
        },
        name: format!("Target {at}"),
        card: None,
        rules: None,
        token: None,
        controller: PlayerId::new(0),
        zone: crate::LogZone::Stack,
        is_current: true,
        referenced_by: Vec::new(),
        colors: ColorSet::default(),
        types: TypeSet::CREATURE,
        power: None,
        toughness: None,
        keywords: 0,
    };
    let mut old = view(2);
    old.target_objects = (1..=50).map(target).collect();
    let mut new = old.clone();
    new.target_objects.push(target(51));
    let patch = holds(&old, &new);
    let [ViewField::TargetObjects(edit)] = &patch.fields[..] else {
        panic!("one field changed: {patch:?}");
    };
    assert_eq!(edit.added, vec![target(51)]);
}

#[test]
fn a_list_edit_that_keeps_too_much_is_refused_by_the_patch() {
    let old = view(2);
    let mut new = old.clone();
    new.seq += 1;
    let mut patch = diff(&old, &new);
    patch.fields.push(ViewField::PolicyActs(ListEdit {
        dropped: 0,
        kept: 5,
        added: Vec::new(),
        tail: 0,
    }));
    assert_eq!(apply(&old, &patch), Err(PatchError::BadList));
}

/// Twenty thousand objects, every one of them removed and named in an order:
/// quadratic work here would be a frame that holds a client for minutes.
#[test]
fn a_huge_patch_costs_no_more_than_its_size() {
    let mut old = view(2);
    old.battlefield = (1..=20_000).map(|id| obj(id, 0)).collect();
    let mut new = old.clone();
    new.battlefield.reverse();
    new.battlefield
        .iter_mut()
        .for_each(|object| object.damage = 1);
    holds(&old, &new);
    let mut gone = old.clone();
    gone.battlefield.clear();
    holds(&old, &gone);
}
