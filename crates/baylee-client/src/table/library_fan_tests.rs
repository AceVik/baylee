use super::*;
use baylee_client_core::{BoardModel, Lane, LaneKind, SeatPod, ZonePile};

fn pod(library: u32) -> SeatPod {
    SeatPod {
        player: PlayerId::new(0),
        life: 20,
        poison: 0,
        energy: 0,
        hand_count: 0,
        library_count: library,
        graveyard_count: 0,
        has_lost: false,
        is_local: true,
        is_active: true,
        has_priority: true,
        role: baylee_client_core::board::SeatRole::Present,
        lanes: vec![Lane {
            kind: LaneKind::Creatures,
            groups: Vec::new(),
            overflowing: false,
        }],
        piles: baylee_client_core::PileKind::ALL
            .into_iter()
            .map(|kind| {
                let mut pile = ZonePile::empty(kind);
                if kind == baylee_client_core::PileKind::Library {
                    pile.count = library;
                }
                pile
            })
            .collect(),
        tokens: Vec::new(),
        threat: baylee_client_core::ThreatSummary::default(),
    }
}

/// An app holding one seat with `library` cards in its deck, and the two
/// handles the system refuses to draw without.
fn running(library: u32) -> App {
    let mut app = App::new();
    app.insert_resource(Duel {
        board: Some(BoardModel {
            seq: 1,
            local: PlayerId::new(0),
            turn: 1,
            step: baylee_view::Step::Main,
            pods: vec![pod(library)],
            stack: Vec::new(),
            hand: Vec::new(),
        }),
        layout: Some(TableLayout::new(&[PlayerId::new(0)], 1.78, None)),
        ..Duel::default()
    });
    // Two dangling handles: the system asks whether it has a mesh and a
    // material and never asks what is in them, so nothing here needs an
    // asset server or a render device.
    app.insert_resource(SceneIndex {
        quad: Some(Handle::default()),
        blank: Some(Handle::default()),
        ..SceneIndex::default()
    });
    app.add_systems(Update, sync_library_fan);
    app.update();
    app
}

fn slabs(app: &mut App) -> usize {
    app.world_mut()
        .query::<&PileVisual>()
        .iter(app.world())
        .count()
}

fn leaving(app: &mut App) -> usize {
    app.world_mut()
        .query::<&Departing>()
        .iter(app.world())
        .count()
}

/// The pointer on a library spreads seven backs out of it, and taking the
/// pointer away sends them home.
///
/// The counter-arm is the first assertion: a library nobody is pointing
/// at draws no fan, so a system that fanned unconditionally would fail
/// here rather than pass everything below it.
#[test]
fn a_library_fans_under_the_pointer_and_folds_when_it_leaves() {
    let mut app = running(60);
    assert_eq!(slabs(&mut app), 0, "a library nobody is pointing at fanned");

    app.world_mut().resource_mut::<Duel>().hovered_pile =
        Some((PlayerId::new(0), baylee_client_core::PileKind::Library));
    app.update();
    assert_eq!(slabs(&mut app), ZonePile::FAN_MAX);
    assert_eq!(leaving(&mut app), 0);

    // And again with the pointer still there: a fan that rebuilt itself
    // every frame would spawn seven more slabs a frame for as long as
    // anybody looked at a deck.
    app.update();
    assert_eq!(slabs(&mut app), ZonePile::FAN_MAX, "the fan was rebuilt");

    app.world_mut().resource_mut::<Duel>().hovered_pile = None;
    app.update();
    assert_eq!(slabs(&mut app), 0, "the fan stayed out");
    assert_eq!(
        leaving(&mut app),
        ZonePile::FAN_MAX,
        "the backs blinked out instead of gliding home"
    );
}

/// A deck fans what it has, and an empty one fans nothing.
///
/// The count is the whole of what a library's fan says — seven identical
/// backs carry no other information, and may not: nobody may look through
/// a library, its owner included (CR 401.2). It is also the one thing the
/// pile's own thickness stops being able to say, because that is capped.
#[test]
fn a_short_deck_fans_as_many_backs_as_it_has() {
    for (cards, backs) in [(0, 0), (1, 1), (4, 4), (7, 4), (60, 4)] {
        let mut app = running(cards);
        app.world_mut().resource_mut::<Duel>().hovered_pile =
            Some((PlayerId::new(0), baylee_client_core::PileKind::Library));
        app.update();
        assert_eq!(
            slabs(&mut app),
            backs,
            "a library of {cards} fanned the wrong number of backs"
        );
    }
}

/// The backs stand in the same fan a graveyard's cards do.
///
/// One shape for every pile, which is the reason a library fans at all: a
/// pile that ignored the pointer would teach a player that some piles are
/// worth pointing at and leave them to guess which.
#[test]
fn the_backs_stand_where_the_fan_puts_them() {
    let mut app = running(60);
    app.world_mut().resource_mut::<Duel>().hovered_pile =
        Some((PlayerId::new(0), baylee_client_core::PileKind::Library));
    app.update();

    let layout = TableLayout::new(&[PlayerId::new(0)], 1.78, None);
    let slot = layout.local().expect("a local seat");
    let wanted: Vec<Vec3> = (0..ZonePile::FAN_MAX)
        .map(|i| {
            let pose = slot.fan_pose(
                baylee_client_core::PileKind::Library,
                i,
                ZonePile::FAN_MAX,
                false,
            );
            card_transform(slot, pose.at, false, pose.lift).translation
        })
        .collect();

    let mut targets: Vec<Vec3> = app
        .world_mut()
        .query_filtered::<&Motion, With<PileVisual>>()
        .iter(app.world())
        .map(|motion| motion.target.translation)
        .collect();
    targets.sort_by(|a, b| a.z.total_cmp(&b.z));
    let mut wanted = wanted;
    wanted.sort_by(|a, b| a.z.total_cmp(&b.z));

    for (got, want) in targets.iter().zip(&wanted) {
        assert!(
            got.distance(*want) < 1e-4,
            "a back is gliding to {got:?} where the fan puts it at {want:?}"
        );
    }
    assert_eq!(targets.len(), ZonePile::FAN_MAX);
}
