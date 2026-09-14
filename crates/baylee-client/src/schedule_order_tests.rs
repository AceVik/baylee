use super::*;
use bevy::ecs::schedule::{NodeId, Schedules};
use bevy::ecs::system::ScheduleSystem;
use core::any::TypeId;

/// The type a system is stored under once a schedule has wrapped it.
///
/// Read by putting the system into a schedule of its own rather than by
/// naming its type: the schedule wraps both the same way, and
/// `System::name` is a placeholder unless `bevy_ecs`'s `debug` feature is
/// on, which this workspace does not carry.
fn stored_as<M>(system: impl IntoScheduleConfigs<ScheduleSystem, M>) -> TypeId {
    let mut app = App::new();
    app.init_schedule(Update);
    app.add_systems(Update, system);
    let schedules = app.world().resource::<Schedules>();
    let graph = schedules.get(Update).expect("just initialized").graph();
    let (_, only, _) = graph
        .systems
        .iter()
        .next()
        .expect("one system was just added");
    only.system_type()
}

/// Whether the assembled `Update` schedule says `first` runs before `second`.
///
/// An ordering edge is not always between two systems: `.before(a_system)`
/// names that system as a set of one, so an edge's far end is usually a
/// `SystemTypeSet` — and a set's own `TypeId` is the *unwrapped* system's,
/// which nothing else here can be compared against. So each end of an edge
/// is resolved to the systems it stands for, through the hierarchy that
/// says which systems a set holds.
fn runs_before(first: TypeId, second: TypeId) -> bool {
    let mut app = App::new();
    app.init_schedule(Update);
    add_present_systems(&mut app);
    let schedules = app.world().resource::<Schedules>();
    let graph = schedules.get(Update).expect("just initialized").graph();
    let stands_for = |node: NodeId| -> Vec<TypeId> {
        match node {
            NodeId::System(key) => graph
                .systems
                .get(key)
                .map(|s| vec![s.system().system_type()])
                .unwrap_or_default(),
            NodeId::Set(_) => graph
                .hierarchy()
                .graph()
                .all_edges()
                .filter(|(parent, _)| *parent == node)
                .filter_map(|(_, child)| match child {
                    NodeId::System(key) => graph.systems.get(key).map(|s| s.system().system_type()),
                    NodeId::Set(_) => None,
                })
                .collect(),
        }
    };
    graph
        .dependency()
        .graph()
        .all_edges()
        .any(|(from, to)| stands_for(from).contains(&first) && stands_for(to).contains(&second))
}

/// A view's batch of moves exists once, and two systems want it:
/// `watch_for_arrivals` stamps the door on to the sweep of a card that has
/// *arrived*, `sync_scene` drains the list to dress the ones that have
/// *left*. Drained first, the tracker answers the second reader with
/// nothing — that view has already been read — and every arrival door
/// silently stops being drawn, with no error and no other failing test.
/// The ordering edge is what stops that, so the edge is what is asserted.
#[test]
fn the_doors_are_stamped_before_the_list_they_are_stamped_from_is_drained() {
    assert!(runs_before(
        stored_as(sheen::watch_for_arrivals),
        stored_as(table::sync_scene),
    ));
}

/// The counter-test: a reader that answered "yes" to any pair at all would
/// pass the assertion above however the two systems were ordered.
#[test]
fn the_edge_the_doors_depend_on_runs_one_way_only() {
    assert!(!runs_before(
        stored_as(table::sync_scene),
        stored_as(sheen::watch_for_arrivals),
    ));
}
