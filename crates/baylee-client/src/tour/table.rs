//! The table's side of the guided tour (TOURS.md §2.3): the practice game
//! starts it, a house game's menu row restarts it, the try-it checks read
//! the duel, the just-in-time tips fire on their anchors — in a game hosted
//! in this client only. A networked table gets no scripted tour and no
//! bubble at all (§1.4, decided: house games only for beta.6), which is
//! what keeps "no narrated step over a pending question there" true.
//!
//! The anchors are marked here, onto the markers the table's spawners
//! already put on their nodes, rather than in each spawner: every one of
//! them is re-spawned on its own revision, and a marker that rides on the
//! existing one is there from the frame the node is. The two 3D things —
//! my mat and the dial — get a proxy node standing where they are drawn.

use baylee_client_core::tour::{
    Anchor, Check, Kind, Mode, Moved, Run, TableGate, Tour, offered_here, table_gate,
};
use bevy::picking::events::{Click, Pointer};
use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;

use super::{Setting, TourAnchor, TourDesk};
use crate::Duel;
use crate::hud::{MenuAction, MenuButton};

/// What the try-it checks have watched since their step opened.
#[derive(Default)]
pub(super) struct Watch {
    /// The step the watch is for (chapter, step).
    step: Option<(usize, usize)>,
    cast_seen: bool,
    visit_seen: bool,
    arrangement: Option<baylee_client_core::tableview::Arrangement>,
    /// The step the last frame stood at, to close the report form when the
    /// tour leaves T33, whose Next closes it (TOURS.md §2.3).
    last: Option<&'static str>,
    /// The menu's Tour row was pressed while the opening hands were being
    /// decided: the tour starts once the game has begun.
    deferred: bool,
}

/// A node standing where a thing drawn in 3D is, for the hole to find.
#[derive(Component)]
pub(super) struct Proxy(Anchor);

/// Whether the table's host runs in this client (a house game).
fn local(host: Option<&crate::InstalledHost>) -> bool {
    host.is_some_and(|h| h.0.link() == crate::host::LinkState::Local)
}

/// Whether this seat is being asked something.
fn pending(duel: &Duel) -> bool {
    duel.view
        .as_ref()
        .is_some_and(|v| v.awaiting == Some(v.seat) || v.deciding.contains(v.seat))
}

/// Starts, restarts and answers the table tour, and fires its tips.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // a Bevy system: the duel and what it watches
pub(super) fn tours(
    duel: Option<Res<Duel>>,
    host: Option<Res<crate::InstalledHost>>,
    mut desk: ResMut<TourDesk>,
    mut settings: ResMut<crate::settings::ClientSettings>,
    input: Option<Res<crate::shellkit::InputClass>>,
    windows: Query<&Window>,
    mut report: Option<ResMut<crate::report::ReportDesk>>,
    anchors: Query<(&TourAnchor, &InheritedVisibility, &ComputedNode)>,
    (mut clicks, buttons, parents): (
        MessageReader<Pointer<Click>>,
        Query<&MenuButton>,
        Query<&ChildOf>,
    ),
    mut watch: Local<Watch>,
) {
    let Some(duel) = duel else {
        return;
    };
    let house = local(host.as_deref());
    let phone = windows.iter().next().is_some_and(|w| w.height() < 500.0);
    let touch = input
        .as_deref()
        .is_some_and(|i| *i == crate::shellkit::InputClass::Touch);
    let asked = pending(&duel);
    let single = desk.run.as_ref().is_some_and(|r| r.single);
    let setting = Setting {
        z: 950,
        alpha: 0.45,
        top: crate::hud::TOP_CLEAR,
        // The bubble stands only in a game hosted here, which runs no clock:
        // a question waits for the player as long as the bubble does, and a
        // bubble folding at every priority would fold at almost every step
        // of a turn. The fold for a question (TOURS.md §1.5) is for a table
        // with a clock, where no bubble stands at all (§1.4).
        pending: asked && !single && !house,
        phone,
        at_table: true,
    };
    if desk.setting != setting {
        desk.setting = setting;
    }
    // The menu's Tour row: the tour from its first chapter, in a house game.
    let pressed_tour = clicks.read().any(|click| {
        let mut at = click.entity;
        for _ in 0..6 {
            if buttons.get(at).is_ok_and(|b| b.action == MenuAction::Tour) {
                return true;
            }
            match parents.get(at) {
                Ok(parent) => at = parent.parent(),
                Err(_) => return false,
            }
        }
        false
    });
    if !house || !offered_here(phone, touch) {
        if desk.run.as_ref().is_some_and(|r| r.tour == Tour::Table) {
            desk.run = None;
            desk.stash = None;
        }
        desk.practice = false;
        return;
    }
    // Leaving T33 closes the form it was read beside.
    let now = desk.run.as_ref().map(|r| r.current().id);
    if watch.last == Some("T33")
        && now != Some("T33")
        && let Some(report) = report.as_deref_mut()
        && report.open
    {
        report.open = false;
    }
    watch.last = now;
    let Some(view) = duel.view.as_ref() else {
        return;
    };
    let seats = view.seats.len();
    // The opening hands are still being decided: nothing can be cast and
    // no question but theirs is asked, so the tour waits to start, and a
    // try-it step that asks for what cannot be done yet is set aside until
    // it can (09.10.: T8 asked for a cast over the mulligan, stuck).
    let opening = !view.deciding.is_empty();
    let park = table_gate(false, desk.run.as_ref(), opening) == TableGate::Park;
    if desk.run.as_ref().is_some_and(|r| r.tour == Tour::Table) && desk.parked != park {
        desk.parked = park;
    }
    let start = (desk.practice && settings.tours.table && desk.run.is_none())
        || pressed_tour
        || watch.deferred;
    if start && table_gate(true, None, opening) == TableGate::WaitToStart {
        watch.deferred |= pressed_tour;
        return;
    }
    if start && let Some(run) = Run::chapter(Tour::Table, 0, phone, seats) {
        desk.practice = false;
        desk.stash = None;
        desk.run = None;
        desk.start(run);
        *watch = Watch::default();
        return;
    }
    // A try-it step: what it asks, watched from the duel; done is the next
    // step at once.
    if let Some(run) = desk.run.as_mut()
        && run.tour == Tour::Table
        && run.mode != Mode::Folded
        && !park
        && let Kind::Try(check) = run.current().kind
    {
        let here = Some((run.chapter, run.step));
        if watch.step != here {
            *watch = Watch {
                step: here,
                arrangement: Some(duel.arrangement),
                last: watch.last,
                ..Watch::default()
            };
        }
        watch.cast_seen |= view.casting.is_some();
        watch.visit_seen |= duel.visiting.is_some();
        let done = match check {
            Check::CastCancelled => watch.cast_seen && view.casting.is_none(),
            Check::ArrangementChanged => watch.arrangement != Some(duel.arrangement),
            Check::Visited => watch.visit_seen && duel.visiting.is_none(),
            Check::LogOpened => duel.log_open,
            Check::ReportOpened => report.as_deref().is_some_and(|r| r.open),
            _ => false,
        };
        if done {
            let mut tours = settings.tours.clone();
            if run.satisfied(&mut tours) == Moved::Over {
                desk.end();
            }
            settings.tours = tours;
            settings.save();
        }
        return;
    }
    // A tip: a just-in-time step whose anchor stands for the first time. It
    // queues behind a narrated step and steps in front of a try-it or a
    // folded one (which comes back after it).
    let tours = &settings.tours;
    if !tours.tips
        || desk
            .run
            .as_ref()
            .is_some_and(|r| r.single || r.mode == Mode::Narrated)
    {
        return;
    }
    for (c, chapter) in Tour::Table.chapters().iter().enumerate() {
        for (s, step) in chapter.steps.iter().enumerate() {
            if step.kind == Kind::Jit
                && !tours.step_seen(Tour::Table, step)
                && step
                    .anchor_on(phone)
                    .is_some_and(|a| super::present(a, &anchors))
            {
                desk.interject(Run::jit(Tour::Table, c, s, phone));
                return;
            }
        }
    }
}

/// Marks the table's nodes with their anchors, riding on the markers their
/// spawners already put there.
#[allow(clippy::type_complexity, clippy::too_many_arguments)] // one query per marker
pub(super) fn mark(
    mut commands: Commands,
    duel: Option<Res<Duel>>,
    shelf: Query<Entity, (With<crate::hud::LedgeShelf>, Without<TourAnchor>)>,
    hand: Query<Entity, (With<crate::hud::HandScroll>, Without<TourAnchor>)>,
    tab: Query<Entity, (With<crate::hud::hand_drawer::HandTab>, Without<TourAnchor>)>,
    stack: Query<Entity, (With<crate::hud::StackPanel>, Without<TourAnchor>)>,
    controls: Query<Entity, (With<crate::hud::StackControls>, Without<TourAnchor>)>,
    sheet: Query<Entity, (With<crate::hud::AbilitySheetRoot>, Without<TourAnchor>)>,
    (drawer, strip, log, panel, pill, reveal, corner, form): (
        Query<Entity, (With<crate::hud::SheetTitle>, Without<TourAnchor>)>,
        Query<Entity, (With<crate::hud::PlayersStrip>, Without<TourAnchor>)>,
        Query<Entity, (With<crate::hud::LogPanel>, Without<TourAnchor>)>,
        Query<Entity, (With<crate::hud::MenuPanel>, Without<TourAnchor>)>,
        Query<
            Entity,
            (
                With<crate::arrangement::ArrangementPill>,
                Without<TourAnchor>,
            ),
        >,
        Query<Entity, (With<crate::hud::revealed::RevealSheet>, Without<TourAnchor>)>,
        Query<Entity, (With<crate::report::ReportCorner>, Without<TourAnchor>)>,
        Query<Entity, (With<crate::report::DeskRoot>, Without<TourAnchor>)>,
    ),
    (buttons, plates, zones): (
        Query<(Entity, &MenuButton), Without<TourAnchor>>,
        Query<(Entity, &crate::hud::PlateTab), Without<TourAnchor>>,
        Query<Entity, (With<crate::hud::TrayZones>, Without<TourAnchor>)>,
    ),
) {
    // `try_`: the HUD rebuilds beside this, unordered, and a node it
    // despawns in this frame is no node to mark (a plain insert panicked).
    let mut put = |entity: Entity, anchor: Anchor| {
        commands.entity(entity).try_insert(TourAnchor(anchor));
    };
    let marked = shelf
        .iter()
        .map(|e| (e, Anchor::Shelf))
        .chain(hand.iter().map(|e| (e, Anchor::Hand)))
        .chain(tab.iter().map(|e| (e, Anchor::HandTab)))
        .chain(stack.iter().map(|e| (e, Anchor::StackPanel)))
        .chain(controls.iter().map(|e| (e, Anchor::StackControls)))
        .chain(sheet.iter().map(|e| (e, Anchor::AbilitySheet)))
        .chain(drawer.iter().map(|e| (e, Anchor::DecisionSheet)))
        .chain(strip.iter().map(|e| (e, Anchor::PlayersStrip)))
        .chain(log.iter().map(|e| (e, Anchor::LogPanel)))
        .chain(panel.iter().map(|e| (e, Anchor::MenuPanel)))
        .chain(pill.iter().map(|e| (e, Anchor::ArrangementPill)))
        .chain(reveal.iter().map(|e| (e, Anchor::RevealSheet)))
        .chain(corner.iter().map(|e| (e, Anchor::ReportCorner)))
        .chain(form.iter().map(|e| (e, Anchor::ReportForm)))
        .chain(zones.iter().map(|e| (e, Anchor::TrayZones)));
    for (entity, anchor) in marked {
        put(entity, anchor);
    }
    for (entity, button) in &buttons {
        match button.action {
            MenuAction::ToggleGameMenu => put(entity, Anchor::Burger),
            MenuAction::ToggleLog => put(entity, Anchor::TrayLog),
            _ => {}
        }
    }
    let me = duel.as_ref().and_then(|d| d.view.as_ref()).map(|v| v.seat);
    for (entity, plate) in &plates {
        if Some(plate.player) == me {
            put(entity, Anchor::MyPlate);
        }
    }
}

/// The first card of the hand carries the hand-card anchor, and only it.
pub(super) fn mark_the_first_card(
    mut commands: Commands,
    cards: Query<(Entity, &UiGlobalTransform, Option<&TourAnchor>), With<crate::hud::HandRowCard>>,
) {
    let first = cards
        .iter()
        .min_by(|a, b| a.1.translation.x.total_cmp(&b.1.translation.x))
        .map(|(e, ..)| e);
    for (entity, _, anchor) in &cards {
        let wants = Some(entity) == first;
        let has = anchor.is_some_and(|a| a.0 == Anchor::HandCard);
        if wants && !has {
            commands
                .entity(entity)
                .try_insert(TourAnchor(Anchor::HandCard));
        } else if !wants && has {
            commands.entity(entity).try_remove::<TourAnchor>();
        }
    }
}

/// Stands the proxies of my mat and the dial where the camera draws them.
pub(super) fn proxies(
    mut commands: Commands,
    duel: Option<Res<Duel>>,
    shown: Option<Res<crate::table::ShownRig>>,
    dial: Option<Res<crate::dial::DialReport>>,
    windows: Query<&Window>,
    mut nodes: Query<(&Proxy, &mut Node)>,
) {
    if nodes.is_empty() {
        for anchor in [Anchor::MyPod, Anchor::Dial] {
            commands.spawn((
                Proxy(anchor),
                TourAnchor(anchor),
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
                Pickable::IGNORE,
            ));
        }
        return;
    }
    let (Some(layout), Some(rig), Ok(window)) = (
        duel.as_ref().and_then(|d| d.layout.as_ref()),
        shown.and_then(|s| s.rig()),
        windows.single(),
    ) else {
        return;
    };
    let lens = crate::table::Lens::new(rig, Vec2::new(window.width(), window.height()));
    let mine = layout
        .on_felt()
        .find(|slot| slot.is_local)
        .and_then(|slot| {
            let (sin, cos) = slot.facing.sin_cos();
            let half = slot.footprint();
            let at = |sx: f32, sy: f32| {
                let local = half * Vec2::new(sx, sy);
                slot.footprint_center()
                    + Vec2::new(
                        cos.mul_add(local.x, sin * local.y),
                        (-sin).mul_add(local.x, cos * local.y),
                    )
            };
            let corners =
                lens.corners([at(-1.0, -1.0), at(1.0, -1.0), at(1.0, 1.0), at(-1.0, 1.0)])?;
            let mut rect = Rect::from_center_size(corners[0], Vec2::ZERO);
            for c in &corners[1..] {
                rect = rect.union_point(*c);
            }
            Some(rect.intersect(Rect::new(0.0, 0.0, window.width(), window.height())))
        });
    let dial = dial.as_deref().and_then(|d| {
        d.centre
            .map(|c| Rect::from_center_size(c, Vec2::splat(d.dial_px.max(1.0))))
    });
    for (proxy, mut node) in &mut nodes {
        let rect = match proxy.0 {
            Anchor::MyPod => mine,
            _ => dial,
        };
        let want = rect.map_or(
            (Val::Px(0.0), Val::Px(0.0), Val::Px(0.0), Val::Px(0.0)),
            |r| {
                (
                    Val::Px(r.min.x),
                    Val::Px(r.min.y),
                    Val::Px(r.width()),
                    Val::Px(r.height()),
                )
            },
        );
        if (node.left, node.top, node.width, node.height) != want {
            node.left = want.0;
            node.top = want.1;
            node.width = want.2;
            node.height = want.3;
        }
    }
}

/// Leaving the table ends the table tour's run (its seen marks stay), and
/// takes the proxies with it.
pub(super) fn leave(
    mut commands: Commands,
    mut desk: ResMut<TourDesk>,
    proxies: Query<Entity, With<Proxy>>,
) {
    for proxy in &proxies {
        commands.entity(proxy).despawn();
    }
    if desk.run.as_ref().is_some_and(|r| r.tour == Tour::Table) {
        desk.run = None;
        desk.stash = None;
        desk.parked = false;
    }
    if desk.stash.as_ref().is_some_and(|r| r.tour == Tour::Table) {
        desk.stash = None;
    }
}
