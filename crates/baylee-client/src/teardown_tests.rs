//! Leaving a table takes down what it drew, each entity once (#321).
//!
//! The owner's release client logged 46 "Entity despawned … is invalid"
//! warnings in one millisecond on leaving a table: `table::despawn_stage`
//! walked everything with `DuelStage` and then everything with `CardVisual`,
//! and every card wears both, so each card was despawned twice.
//!
//! A test cannot see that through bevy's `FallbackErrorHandler`: a failed
//! `EntityCommands::despawn` goes to the fixed `warn` handler, which logs
//! through the `log` facade and tells nobody else. So this module installs a
//! logger of its own and counts what reaches it. The warnings are recognised
//! by the entities they name, which are spawned past a block of placeholder
//! indices no other test's world reaches, so a test running beside this one
//! cannot put a line into its count.

use std::sync::Mutex;

use bevy::prelude::*;
use bevy::state::app::StatesPlugin;

use crate::DuelPhase;
use crate::table::{CardVisual, DuelStage};

/// Every `bevy_ecs` warning logged while this process runs.
static CAUGHT: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Held by each test here for its whole run: the two build worlds the same
/// way, so their entities share indices, and one's deliberate double despawn
/// would otherwise land in the other's count.
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

struct Catch;

impl log::Log for Catch {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= log::Level::Warn && metadata.target().starts_with("bevy_ecs")
    }

    fn log(&self, record: &log::Record<'_>) {
        if self.enabled(record.metadata()) {
            CAUGHT
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(record.args().to_string());
        }
    }

    fn flush(&self) {}
}

static CATCH: Catch = Catch;

/// Installs [`Catch`] as the process's logger, and says whether it is the
/// one installed — a count taken through somebody else's logger would be a
/// count of nothing.
fn catching() -> bool {
    if log::set_logger(&CATCH).is_ok() {
        log::set_max_level(log::LevelFilter::Warn);
    }
    std::ptr::addr_eq(log::logger(), &raw const CATCH)
}

/// The warnings that name one of `ours`.
fn warnings_about(ours: &[Entity]) -> Vec<String> {
    let caught = CAUGHT
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    caught
        .iter()
        .filter(|line| ours.iter().any(|e| line.contains(&format!("ID {e} "))))
        .cloned()
        .collect()
}

/// An app with exactly the client's teardown on `OnEnter(Closed)`, a duel
/// open on it.
fn a_table() -> App {
    // What an earlier test here caught names entities at this world's
    // indices too.
    CAUGHT
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clear();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin))
        .init_state::<DuelPhase>()
        .init_resource::<crate::table::SceneIndex>()
        .init_resource::<crate::table::ZoneWatch>()
        .init_resource::<crate::hud::HudRevision>()
        .init_resource::<crate::hud::LedgeRevision>();
    crate::tear_the_table_down(&mut app);
    // Indices no other test's world reaches, so a line naming one of this
    // world's entities is this world's. Left alive: a despawned index is
    // handed out again, and the next spawn would land back among them.
    app.world_mut().spawn_batch((0..50_000).map(|_| ()));
    app.update();
    app.world_mut()
        .resource_mut::<NextState<DuelPhase>>()
        .set(DuelPhase::Opening);
    app.update();
    app
}

/// What `sync_scene` spawns for one card, as it spawns it: both markers,
/// and a child under it the way its shadow and its badge hang.
fn a_card(app: &mut App, n: u32) -> [Entity; 2] {
    let card = app
        .world_mut()
        .spawn((
            DuelStage,
            CardVisual {
                object: baylee_core::ids::ObjectId::new(n, 0),
                count: 1,
            },
        ))
        .id();
    let shadow = app.world_mut().spawn(ChildOf(card)).id();
    [card, shadow]
}

fn close(app: &mut App) {
    app.world_mut()
        .resource_mut::<NextState<DuelPhase>>()
        .set(DuelPhase::Closed);
    app.update();
}

#[test]
fn leaving_a_table_despawns_every_card_once() {
    let _alone = ONE_AT_A_TIME
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert!(catching(), "the test's logger is the process's logger");
    let mut app = a_table();
    let mut ours = Vec::new();
    for n in 0..5 {
        ours.extend(a_card(&mut app, n));
    }
    // A zone is `DuelStage` too, and nothing stops one of its parts from
    // being marked as well: the ancestor takes it, and no second command
    // may ask for it again.
    // Both orders of spawning, because the order the commands are queued in
    // follows the entities' own and only one of the two fails without the
    // ancestor check: a pile made before its zone, and one made after.
    for pile_first in [true, false] {
        let (zone, pile) = if pile_first {
            let pile = app.world_mut().spawn(DuelStage).id();
            let zone = app.world_mut().spawn(DuelStage).id();
            app.world_mut().entity_mut(pile).insert(ChildOf(zone));
            (zone, pile)
        } else {
            let zone = app.world_mut().spawn(DuelStage).id();
            let pile = app.world_mut().spawn((DuelStage, ChildOf(zone))).id();
            (zone, pile)
        };
        ours.extend([zone, pile]);
    }

    close(&mut app);

    let gone = ours
        .iter()
        .filter(|e| app.world().get_entity(**e).is_err())
        .count();
    assert_eq!(gone, ours.len(), "the whole stage came down");
    let warned = warnings_about(&ours);
    assert!(
        warned.is_empty(),
        "leaving the table asked for {} entities that were already gone:\n{}",
        warned.len(),
        warned.join("\n")
    );
}

/// The instrument, run against the fault it exists to see: a double despawn
/// of one of this world's entities must reach the count, or the test above
/// passes by seeing nothing.
#[test]
fn the_logger_sees_a_double_despawn() {
    let _alone = ONE_AT_A_TIME
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert!(catching(), "the test's logger is the process's logger");
    let mut app = a_table();
    let [card, _] = a_card(&mut app, 99);
    app.world_mut().commands().entity(card).despawn();
    app.world_mut().commands().entity(card).despawn();
    app.update();
    assert_eq!(
        warnings_about(&[card]).len(),
        1,
        "the second despawn is warned about exactly once"
    );
}

#[test]
fn leaving_a_table_removes_detached_hud_roots_and_their_contents() {
    let _alone = ONE_AT_A_TIME
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut app = a_table();
    let root = app.world_mut().spawn(crate::hud::DetachedHud).id();
    let child = app.world_mut().spawn(ChildOf(root)).id();
    close(&mut app);
    assert!(app.world().get_entity(root).is_err());
    assert!(app.world().get_entity(child).is_err());
}
