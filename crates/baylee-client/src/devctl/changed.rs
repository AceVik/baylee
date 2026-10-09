//! `/changed`: what this client marks changed while it should be at rest.
//!
//! A write marks a component or resource changed even when the value is the
//! one it held, and a changed `Node` relays out the whole interface, a
//! changed material is uploaded again. At rest nothing should be written; the
//! second and third client rounds (`docs/perf-client.md`) found writers that
//! had crept back with a system in `Last` written by hand each time. This is
//! that system, kept:
//!
//! - `/changed {"frames":N}` arms it for the next `N` frames (at most 600);
//! - `/changed {}` answers what it saw, as plain text: per component (and
//!   resource, which bevy 0.19 keeps as a component) the number of
//!   entity-frames it was changed in, and — in a build with
//!   `--features bevy/track_location` — the last writer's source line for
//!   each, counted the same way. Without that feature the writers read `?`.
//!
//! It walks every archetype's components by id, so a type nobody thought to
//! list is seen too. That costs a few milliseconds a frame on a busy table,
//! which is why it runs only while armed.

use bevy::ecs::change_detection::Tick;
use bevy::ecs::component::ComponentId;
use bevy::prelude::*;
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// Frames a single arming may cover.
const MAX_FRAMES: u32 = 600;

/// What `/changed` has seen since it was armed.
#[derive(Resource, Default)]
pub(super) struct Tally {
    /// Frames still to watch.
    left: u32,
    /// Frames watched.
    watched: u32,
    /// Per component name: entity-frames changed, and per writer.
    seen: BTreeMap<String, (u32, BTreeMap<String, u32>)>,
}

/// `/changed`: arms the tally for `frames` frames, or (no `frames`) answers
/// with what it saw.
pub(super) fn answer(tally: &mut Tally, frames: Option<u32>) -> String {
    if let Some(frames) = frames {
        *tally = Tally {
            left: frames.clamp(1, MAX_FRAMES),
            ..Tally::default()
        };
        return format!("{{\"ok\":true,\"armed\":{}}}", tally.left);
    }
    let mut out = format!(
        "frames watched {} (still to watch {})\n",
        tally.watched, tally.left
    );
    let mut rows: Vec<_> = tally.seen.iter().collect();
    rows.sort_by(|a, b| b.1.0.cmp(&a.1.0).then_with(|| a.0.cmp(b.0)));
    for (name, (count, writers)) in rows {
        let _ = writeln!(out, "{count:7}  {name}");
        let mut writers: Vec<_> = writers.iter().collect();
        writers.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
        for (writer, n) in writers.into_iter().take(8) {
            let _ = writeln!(out, "           {n:7}  {writer}");
        }
    }
    out
}

/// A component whose writer can be read by type: the ones a rest check
/// asks about first.
type Writer = fn(&EntityRef) -> Option<String>;

fn writer_of<T: Component>(entity: &EntityRef) -> Option<String> {
    entity
        .get_changed_by::<T>()
        .and_then(bevy::ecs::change_detection::MaybeLocation::into_option)
        .map(|at| format!("{}:{}", at.file(), at.line()))
}

/// The types the writer is read for, each with its name (a component's own
/// name needs bevy's `debug` feature; these are named without it): the
/// interface's layout and paint, the table's transforms and visibility, and
/// what bevy derives from them.
fn writers(world: &World) -> Vec<(ComponentId, &'static str, Writer)> {
    macro_rules! of {
        ($($t:ty),* $(,)?) => {
            [$((
                world.component_id::<$t>(),
                std::any::type_name::<$t>(),
                writer_of::<$t> as Writer,
            )),*]
        };
    }
    of![
        GlobalTransform,
        bevy::ui::ComputedNode,
        bevy::ui::UiGlobalTransform,
        InheritedVisibility,
        ViewVisibility,
        Node,
        Text,
        TextColor,
        TextFont,
        TextSpan,
        UiTransform,
        Transform,
        BackgroundColor,
        BorderColor,
        Visibility,
        ImageNode,
        crate::Duel,
        crate::prefs::Prefs,
        crate::hud::HudRevision,
        crate::hud::seatbar::BarRevision,
    ]
    .into_iter()
    .filter_map(|(id, name, read)| Some((id?, name, read)))
    .collect()
}

/// `on A, B, …`: the entity's other components (at most eight), each by its
/// last path segment; `?` for a resource standing alone.
fn shape(world: &World, components: &[ComponentId], changed: ComponentId) -> String {
    let names: Vec<String> = components
        .iter()
        .filter(|&&id| id != changed)
        .take(8)
        .filter_map(|&id| world.components().get_name(id))
        .map(|name| {
            let name = name.to_string();
            let head = name.split('<').next().unwrap_or(&name);
            let cut = head.rfind("::").map_or(0, |at| at + 2);
            name[cut..].to_string()
        })
        .collect();
    if names.is_empty() {
        "?".to_string()
    } else {
        format!("on {}", names.join(", "))
    }
}

/// Runs in `Last` while armed: every component changed since the last run.
pub(super) fn watch(world: &mut World, mut since: Local<Option<Tick>>) {
    let now = world.change_tick();
    let last = since.replace(now);
    if world.resource::<Tally>().left == 0 {
        return;
    }
    let Some(last) = last else {
        return;
    };
    let readers = writers(world);
    let mut frame: BTreeMap<String, (u32, BTreeMap<String, u32>)> = BTreeMap::new();
    for archetype in world.archetypes().iter() {
        for entity in archetype.entities() {
            let Ok(entity) = world.get_entity(entity.id()) else {
                continue;
            };
            for &id in archetype.components() {
                let Some(ticks) = entity.get_change_ticks_by_id(id) else {
                    continue;
                };
                if !ticks.is_changed(last, now) {
                    continue;
                }
                let known = readers.iter().find(|(of, ..)| *of == id);
                let name = known.map_or_else(
                    || {
                        world
                            .components()
                            .get_name(id)
                            .map_or_else(|| format!("{id:?}"), |n| n.to_string())
                    },
                    |(_, name, _)| (*name).to_string(),
                );
                // With no writer to read, what else the entity carries says
                // which one it is.
                let writer = known
                    .and_then(|(_, _, read)| read(&entity))
                    .unwrap_or_else(|| shape(world, archetype.components(), id));
                let row = frame.entry(name).or_default();
                row.0 += 1;
                *row.1.entry(writer).or_default() += 1;
            }
        }
    }
    let mut tally = world.resource_mut::<Tally>();
    tally.left -= 1;
    tally.watched += 1;
    for (name, (count, writers)) in frame {
        let row = tally.seen.entry(name).or_default();
        row.0 += count;
        for (writer, n) in writers {
            *row.1.entry(writer).or_default() += n;
        }
    }
}
