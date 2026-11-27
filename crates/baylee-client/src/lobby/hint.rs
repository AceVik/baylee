//! A sentence beside the pointer, for a mark too small to carry its words.
//!
//! Built like the card preview in `preview.rs`, and for the same reason: one
//! node outside the retained tree, so a pointer moving on and off a mark
//! draws one panel and does not rebuild the lobby. A finger held on the mark
//! is a hover too, so it reads the same on a phone.

#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;

/// The sentence a mark explains itself with.
#[derive(Component, Clone, PartialEq, Eq)]
pub(crate) struct HoverHint(pub(crate) String);

/// A mark whose hint was dismissed by pressing it; it stays quiet until the
/// pointer leaves that mark (or its in-place rebuild replacement).
#[derive(Clone, PartialEq, Eq)]
struct Dismissed {
    entity: Entity,
    text: String,
    seen_in_hover_map: bool,
}

/// The mark the pointer or keyboard focus is over, and where the pointer was.
#[derive(Resource, Default)]
pub(super) struct Hinted {
    text: Option<String>,
    source: Option<Entity>,
    at: Vec2,
    by_focus: bool,
    by_hover_map: bool,
    dismissed: Option<Dismissed>,
    /// Bumped whenever either changes, so the panel knows to redraw.
    epoch: u64,
}

/// The panel.
#[derive(Component)]
pub(super) struct HintPanel {
    epoch: u64,
    progress: f32,
    above: bool,
}

/// Marks the text node inside [`HintPanel`] so its opacity can animate in.
#[derive(Component)]
pub(super) struct HintPanelWords;

/// The widest a hint is drawn, in logical pixels, before it wraps.
const HINT_WIDTH: f32 = 300.0;

/// Tracks which mark the pointer (including [`HoverMap`] over [`Disabled`]
/// controls) or keyboard [`InputFocus`] (on [`Disabled`] controls) is over.
#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::type_complexity
)]
pub(super) fn hint_hovers(
    (mut overs, mut outs, presses, clicks): (
        MessageReader<PointerOver>,
        MessageReader<PointerOut>,
        Option<MessageReader<bevy::picking::events::PointerPress>>,
        Option<MessageReader<bevy::picking::events::PointerClick>>,
    ),
    hints: Query<(&HoverHint, Has<crate::hud::Hint>)>,
    disabled: Query<(), With<crate::shellkit::controls::Disabled>>,
    parents: Query<&ChildOf>,
    hovers: Option<Res<bevy::picking::hover::HoverMap>>,
    focus: Option<Res<bevy::input_focus::InputFocus>>,
    (phase, lobby_state, report): (
        Option<Res<State<DuelPhase>>>,
        Option<Res<LobbyState>>,
        Option<Res<crate::report::ReportDesk>>,
    ),
    mut hinted: ResMut<Hinted>,
) {
    let in_duel = phase
        .as_deref()
        .is_some_and(|p| *p.get() != DuelPhase::Closed);
    let over_table = super::table_settings::covers_the_table(lobby_state.as_deref());
    let report_open = report.as_deref().is_some_and(|r| r.open);
    if in_duel && !over_table && !report_open {
        overs.clear();
        outs.clear();
        if let Some(mut presses) = presses {
            presses.clear();
        }
        if let Some(mut clicks) = clicks {
            clicks.clear();
        }
        if hinted.text.is_some()
            || hinted.source.is_some()
            || hinted.by_focus
            || hinted.by_hover_map
            || hinted.dismissed.is_some()
        {
            hinted.text = None;
            hinted.source = None;
            hinted.by_focus = false;
            hinted.by_hover_map = false;
            hinted.dismissed = None;
            hinted.epoch = hinted.epoch.wrapping_add(1);
        }
        return;
    }

    let mut text = hinted.text.clone();
    let mut source = hinted.source;
    let mut at = hinted.at;
    let mut by_focus = hinted.by_focus;
    let mut by_hover_map = hinted.by_hover_map;
    let mut dismissed = hinted.dismissed.clone();
    // A rebuild takes the mark away under a still pointer, and no `Out`
    // says so.
    if source.is_some_and(|entity| {
        !hints
            .get(entity)
            .is_ok_and(|(_, has_hud_hint)| !(in_duel && has_hud_hint))
    }) {
        text = None;
        source = None;
        by_focus = false;
        by_hover_map = false;
    }
    let focused = focus
        .as_deref()
        .and_then(bevy::input_focus::InputFocus::get);
    if by_focus && focused != source {
        text = None;
        source = None;
        by_focus = false;
    }
    let hover_map_entity = hovers.as_deref().and_then(|map| {
        map.values()
            .flat_map(|m| m.iter())
            .filter_map(|(&e, hit)| lineage(e, &hints, &parents, in_duel).map(|h| (h, hit.depth)))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(h, _)| h)
    });
    let out_entities: Vec<Entity> = outs
        .read()
        .filter_map(|out| lineage(out.entity, &hints, &parents, in_duel))
        .collect();
    let over_events: Vec<(Entity, Vec2)> = overs
        .read()
        .filter_map(|over| {
            lineage(over.entity, &hints, &parents, in_duel)
                .map(|entity| (entity, over.pointer.position))
        })
        .collect();

    if let Some(d) = dismissed.as_mut() {
        let alive = hints
            .get(d.entity)
            .is_ok_and(|(_, has_hud_hint)| !(in_duel && has_hud_hint));
        if !alive {
            // Pressing a control that rebuilds the tree (such as the builder's
            // trash menu) replaces its entity under a still pointer. Keep the
            // hint dismissed on the replacement until the pointer leaves it.
            let replacement = hover_map_entity
                .or_else(|| over_events.last().map(|(e, _)| *e))
                .filter(|&e| {
                    hints
                        .get(e)
                        .is_ok_and(|(h, _)| h.0.trim() == d.text.as_str())
                });
            if let Some(new_entity) = replacement {
                d.entity = new_entity;
                d.seen_in_hover_map = hover_map_entity == Some(new_entity);
            } else {
                dismissed = None;
            }
        }
    }
    if let Some(d) = dismissed.as_mut() {
        if hover_map_entity == Some(d.entity) {
            d.seen_in_hover_map = true;
        } else if d.seen_in_hover_map
            || (out_entities.contains(&d.entity)
                && !over_events.iter().any(|(e, _)| *e == d.entity))
            || over_events.iter().any(|(e, _)| *e != d.entity)
        {
            dismissed = None;
        }
    }

    for entity in &out_entities {
        if Some(*entity) == source {
            text = None;
            source = None;
            by_focus = false;
            by_hover_map = false;
        }
    }
    for (entity, position) in over_events {
        if dismissed.as_ref().is_some_and(|d| d.entity == entity) {
            continue;
        }
        if let Ok((hint, _)) = hints.get(entity) {
            let trimmed = hint.0.trim();
            if trimmed.is_empty() {
                text = None;
                source = None;
            } else {
                source = Some(entity);
                text = Some(trimmed.to_owned());
                at = position;
            }
            by_focus = false;
            by_hover_map = false;
        }
    }
    let mut pressed_entities = Vec::new();
    if let Some(mut presses) = presses {
        for press in presses.read() {
            if let Some(entity) = lineage(press.entity, &hints, &parents, in_duel) {
                pressed_entities.push(entity);
            }
        }
    }
    if let Some(mut clicks) = clicks {
        for click in clicks.read() {
            if let Some(entity) = lineage(click.entity, &hints, &parents, in_duel) {
                pressed_entities.push(entity);
            }
        }
    }
    for entity in pressed_entities {
        if let Ok((hint, _)) = hints.get(entity) {
            let trimmed = hint.0.trim();
            if !trimmed.is_empty() {
                dismissed = Some(Dismissed {
                    entity,
                    text: trimmed.to_owned(),
                    seen_in_hover_map: hover_map_entity == Some(entity),
                });
            }
        }
        if source == Some(entity) {
            text = None;
            source = None;
            by_focus = false;
            by_hover_map = false;
        }
    }
    if by_hover_map && hover_map_entity != source {
        text = None;
        source = None;
        by_hover_map = false;
    }
    if source.is_none()
        && let Some(entity) = hover_map_entity
        && dismissed.as_ref().is_none_or(|d| d.entity != entity)
        && let Ok((hint, _)) = hints.get(entity)
    {
        let trimmed = hint.0.trim();
        if !trimmed.is_empty() {
            source = Some(entity);
            text = Some(trimmed.to_owned());
            by_hover_map = true;
            by_focus = false;
        }
    }
    if source.is_none()
        && let Some(entity) = focused
        && disabled.contains(entity)
        && dismissed.as_ref().is_none_or(|d| d.entity != entity)
        && let Ok((hint, has_hud_hint)) = hints.get(entity)
        && !(in_duel && has_hud_hint)
    {
        let trimmed = hint.0.trim();
        if !trimmed.is_empty() {
            source = Some(entity);
            text = Some(trimmed.to_owned());
            by_focus = true;
            by_hover_map = false;
        }
    }
    if let Some(entity) = source
        && let Ok((hint, _)) = hints.get(entity)
    {
        let trimmed = hint.0.trim();
        if trimmed.is_empty() {
            text = None;
            source = None;
            by_focus = false;
            by_hover_map = false;
        } else if text.as_deref() != Some(trimmed) {
            text = Some(trimmed.to_owned());
        }
    }
    let display_changed = text != hinted.text
        || source != hinted.source
        || by_focus != hinted.by_focus
        || by_hover_map != hinted.by_hover_map;
    if display_changed || dismissed != hinted.dismissed {
        hinted.text = text;
        hinted.source = source;
        hinted.at = at;
        hinted.by_focus = by_focus;
        hinted.by_hover_map = by_hover_map;
        hinted.dismissed = dismissed;
        if display_changed {
            hinted.epoch = hinted.epoch.wrapping_add(1);
        }
    }
}

/// The nearest entity at or above this one that carries a shell hint.
/// Over a table (`in_duel`), controls that also carry [`crate::hud::Hint`] are
/// owned by the HUD's tooltip system so two tooltips never stack.
fn lineage(
    entity: Entity,
    hints: &Query<(&HoverHint, Has<crate::hud::Hint>)>,
    parents: &Query<&ChildOf>,
    in_duel: bool,
) -> Option<Entity> {
    let mut current = Some(entity);
    while let Some(e) = current {
        if let Ok((_, has_hud_hint)) = hints.get(e) {
            return (!(in_duel && has_hud_hint)).then_some(e);
        }
        current = parents.get(e).ok().map(ChildOf::parent);
    }
    None
}

/// Draws the hinted sentence beside its source control (or the pointer), with a
/// smooth, fast entrance animation.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(super) fn hint_panel(
    mut commands: Commands,
    time: Res<Time>,
    mut hinted: ResMut<Hinted>,
    mut existing: Query<(
        Entity,
        &mut HintPanel,
        &mut UiTransform,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
    mut word_colors: Query<&mut TextColor, With<HintPanelWords>>,
    hints: Query<&HoverHint>,
    nodes: Query<(&ComputedNode, &UiGlobalTransform)>,
    windows: Query<&Window>,
    fonts: Option<Res<UiFonts>>,
    ui: Option<Res<UiScale>>,
) {
    if hinted.text.is_none() && hinted.source.is_none() && existing.is_empty() {
        return;
    }
    if hinted.source.is_some_and(|e| hints.get(e).is_err()) {
        hinted.text = None;
        hinted.source = None;
        hinted.by_focus = false;
        hinted.by_hover_map = false;
        hinted.epoch = hinted.epoch.wrapping_add(1);
    }
    if existing
        .iter()
        .any(|(_, panel, ..)| panel.epoch == hinted.epoch)
    {
        let dt = time.delta_secs().min(0.05);
        for (_, mut panel, mut transform, mut bg, mut border) in &mut existing {
            if panel.progress < 1.0 {
                panel.progress = (panel.progress + dt / 0.11).min(1.0);
                let ease = 1.0 - (1.0 - panel.progress).powi(3);
                let dy = if panel.above {
                    (1.0 - ease) * 5.0
                } else {
                    (1.0 - ease) * -5.0
                };
                transform.translation = Val2::px(0.0, dy);
                transform.scale = Vec2::splat(0.94 + 0.06 * ease);
                bg.0 = palette::PANEL_LIT.with_alpha(palette::PANEL_LIT.alpha() * ease);
                *border = BorderColor::all(palette::DOCK_EDGE.with_alpha(0.55 * ease));
                for mut tc in &mut word_colors {
                    tc.0 = palette::INK.with_alpha(ease);
                }
            }
        }
        return;
    }
    for (entity, ..) in &existing {
        commands.entity(entity).try_despawn();
    }
    let (Some(text), Some(fonts)) = (hinted.text.clone(), fonts.as_deref()) else {
        return;
    };
    let canvas = windows.iter().next().map_or(Vec2::new(1280.0, 800.0), |w| {
        crate::hud::scale::space(w, ui.as_deref())
    });
    let ui_scale = ui.as_deref().map_or(1.0, |scale| scale.0);
    let at = hinted.at / ui_scale;
    let metrics = Metrics::of(canvas.x);
    let width = HINT_WIDTH.min(canvas.x - 16.0).max(80.0);

    // Anchor relative to the hovered element's bounding box when laid out,
    // using `bottom` near the bottom edge and `right` near the right edge so
    // tooltips on corner buttons ("Über", Version, header tools) sit flush.
    let source_box = hinted
        .source
        .and_then(|src| nodes.get(src).ok())
        .filter(|(c, _)| c.size().x > 0.0 && c.size().y > 0.0)
        .map(|(c, t)| {
            let inv = c.inverse_scale_factor();
            let center = t.translation * inv;
            let size = c.size() * inv;
            (center, size)
        });
    if hinted.source.is_some() && source_box.is_none() && (hinted.by_focus || hinted.by_hover_map) {
        return;
    }

    let (left, right, top, bottom, above) = if let Some((center, size)) = source_box {
        let box_top = center.y - size.y * 0.5;
        let box_bottom = center.y + size.y * 0.5;
        let box_left = center.x - size.x * 0.5;
        let box_right = center.x + size.x * 0.5;
        let place_above = box_bottom > canvas.y - 110.0;
        let (top, bottom) = if place_above {
            (Val::Auto, px((canvas.y - box_top + 8.0).max(8.0)))
        } else {
            (px((box_bottom + 8.0).max(8.0)), Val::Auto)
        };
        let (left, right) = if center.x > canvas.x * 0.65 {
            (Val::Auto, px((canvas.x - box_right).max(10.0)))
        } else if center.x < canvas.x * 0.35 {
            (px(box_left.max(10.0)), Val::Auto)
        } else {
            (
                px((center.x - 36.0).clamp(10.0, (canvas.x - width - 10.0).max(10.0))),
                Val::Auto,
            )
        };
        (left, right, top, bottom, place_above)
    } else {
        let place_above = at.y > canvas.y - 100.0;
        let (top, bottom) = if place_above {
            (Val::Auto, px((canvas.y - at.y + 14.0).max(8.0)))
        } else {
            (px((at.y + 18.0).max(8.0)), Val::Auto)
        };
        let (left, right) = if at.x > canvas.x * 0.65 {
            (Val::Auto, px((canvas.x - at.x).max(10.0)))
        } else {
            (
                px((at.x + 12.0).min(canvas.x - width - 8.0).max(8.0)),
                Val::Auto,
            )
        };
        (left, right, top, bottom, place_above)
    };

    let init_dy = if above { 4.0 } else { -4.0 };
    let panel = commands
        .spawn((
            HintPanel {
                epoch: hinted.epoch,
                progress: 0.25,
                above,
            },
            Node {
                position_type: PositionType::Absolute,
                left,
                right,
                top,
                bottom,
                max_width: px(width),
                padding: UiRect::axes(px(metrics.pad * 0.72), px(metrics.pad * 0.42)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(8)),
                ..default()
            },
            UiTransform {
                translation: Val2::px(0.0, init_dy),
                scale: Vec2::splat(0.95),
                ..default()
            },
            BackgroundColor(palette::PANEL_LIT.with_alpha(0.96)),
            BorderColor::all(palette::DOCK_EDGE.with_alpha(0.55)),
            soft_shadow(),
            GlobalZIndex(crate::shellkit::tokens::z::TOOLTIP),
            // A hint must never eat the click meant for what is under it.
            Pickable::IGNORE,
        ))
        .id();
    let words = commands
        .spawn((
            HintPanelWords,
            Text::new(text),
            tf(fonts, metrics.small),
            TextColor(palette::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(panel).add_child(words);
}

/// Takes the hint down with the lobby.
pub(super) fn despawn_hint(
    mut commands: Commands,
    panels: Query<Entity, With<HintPanel>>,
    mut hinted: ResMut<Hinted>,
) {
    for entity in panels {
        commands.entity(entity).try_despawn();
    }
    *hinted = Hinted::default();
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::input_focus::{FocusCause, InputFocus};
    use bevy::state::app::StatesPlugin;

    #[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
    #[test]
    #[allow(clippy::too_many_lines)]
    fn hint_waits_for_layout_clears_over_table_and_skips_hud_hints_in_duels() {
        let (mut app, fonts) = crate::shellkit::header::tests::layout_app();
        app.add_plugins(StatesPlugin)
            .init_state::<DuelPhase>()
            .init_resource::<Hinted>()
            .init_resource::<InputFocus>()
            .insert_resource(LobbyState::from_settings(
                crate::settings::ClientSettings::default(),
            ))
            .insert_resource(fonts)
            .add_systems(Update, (hint_hovers, hint_panel).chain());

        // Spawn a Disabled control with HoverHint and focus it BEFORE PostUpdate layout.
        let btn = app
            .world_mut()
            .spawn((
                Node {
                    left: px(240),
                    top: px(160),
                    width: px(120),
                    height: px(40),
                    position_type: PositionType::Absolute,
                    ..default()
                },
                crate::shellkit::controls::Disabled,
                HoverHint("Choose a gateway first".to_string()),
            ))
            .id();
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(btn, FocusCause::Navigated);

        // Run only `Update` once before layout has computed `btn`'s size: must NOT pin at (8, 18).
        bevy::ecs::system::RunSystemOnce::run_system_once(app.world_mut(), hint_hovers).unwrap();
        bevy::ecs::system::RunSystemOnce::run_system_once(app.world_mut(), hint_panel).unwrap();
        app.world_mut().flush();
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<HintPanel>>()
                .iter(app.world())
                .count(),
            0,
            "HintPanel waits for layout instead of spawning at top-left (8px, 18px)"
        );

        // Full frame runs PostUpdate layout, then the next frame spawns HintPanel at `btn`.
        app.update();
        app.update();
        let panel = app
            .world_mut()
            .query_filtered::<&Node, With<HintPanel>>()
            .single(app.world())
            .expect("HintPanel spawned once laid out");
        assert!(
            matches!(panel.top, Val::Px(y) if y > 180.0),
            "HintPanel anchors below the laid-out control ({:?})",
            panel.top
        );

        // Entering a duel (with Settings closed) clears Hinted and despawns HintPanel,
        // and subsequent frames over the 3D table do not mutate Hinted at rest.
        app.world_mut()
            .resource_mut::<NextState<DuelPhase>>()
            .set(DuelPhase::Playing);
        app.update();
        app.update();
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<HintPanel>>()
                .iter(app.world())
                .count(),
            0,
            "HintPanel never stays stranded over the 3D table"
        );
        let rest_tick = app
            .world()
            .get_resource_ref::<Hinted>()
            .map(|r| r.last_changed());
        app.update();
        assert_eq!(
            app.world()
                .get_resource_ref::<Hinted>()
                .map(|r| r.last_changed()),
            rest_tick,
            "resting duel frame does not mutate Hinted"
        );

        // Even when Report is open over the table, controls carrying `hud::Hint` (social)
        // are skipped by `hint_hovers` so two tooltips never stack.
        let mut desk = crate::report::ReportDesk::default();
        desk.open = true;
        app.insert_resource(desk);
        let dual_btn = app
            .world_mut()
            .spawn((
                Node {
                    width: px(80),
                    height: px(32),
                    ..default()
                },
                crate::shellkit::controls::Disabled,
                HoverHint("Social hint".to_string()),
                crate::hud::Hint("Social hint".to_string()),
            ))
            .id();
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(dual_btn, FocusCause::Navigated);
        app.update();
        app.update();
        assert!(
            app.world().resource::<Hinted>().source.is_none(),
            "dual HoverHint + hud::Hint control defers to HUD tooltip in duels"
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn press_dismisses_hint_until_pointer_leaves_mark() {
        use bevy::picking::backend::HitData;
        use bevy::picking::events::{PointerClick, PointerPress};
        use bevy::picking::hover::HoverMap;
        use bevy::picking::pointer::{Location, PointerButton, PointerId};

        fn aim_at<E: crate::test_pointer::Aim>(entity: Entity, event: E) -> E::Event {
            crate::test_pointer::aim(
                PointerId::Mouse,
                Location {
                    target: bevy::camera::NormalizedRenderTarget::Window(
                        bevy::window::WindowRef::Entity(Entity::PLACEHOLDER)
                            .normalize(None)
                            .unwrap(),
                    ),
                    position: Vec2::new(1120.0, 24.0),
                },
                event,
                entity,
            )
        }

        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Hinted>()
            .init_resource::<HoverMap>()
            .add_message::<PointerOver>()
            .add_message::<PointerOut>()
            .add_message::<PointerPress>()
            .add_message::<PointerClick>()
            .add_systems(Update, hint_hovers);

        let bell = app
            .world_mut()
            .spawn(HoverHint("Notifications".to_string()))
            .id();
        let hit = HitData::new(Entity::PLACEHOLDER, 0.0, None, None);

        // Hovering the mark shows its hint.
        app.world_mut()
            .resource_mut::<HoverMap>()
            .0
            .entry(PointerId::Mouse)
            .or_default()
            .insert(bell, hit.clone());
        app.world_mut()
            .write_message(aim_at(bell, crate::test_pointer::Over { hit: hit.clone() }));
        app.update();
        assert_eq!(app.world().resource::<Hinted>().source, Some(bell));
        assert_eq!(
            app.world().resource::<Hinted>().text.as_deref(),
            Some("Notifications")
        );

        // Pressing the mark dismisses the hint immediately.
        app.world_mut().write_message(aim_at(
            bell,
            crate::test_pointer::Press {
                button: PointerButton::Primary,
                hit: hit.clone(),
                count: 1,
            },
        ));
        app.update();
        assert!(app.world().resource::<Hinted>().source.is_none());
        assert!(app.world().resource::<Hinted>().text.is_none());

        // HoverMap still holding the same mark on later frames must not bring
        // the hint back, and resting frames must not mutate `Hinted`.
        let rest_tick = app
            .world()
            .get_resource_ref::<Hinted>()
            .map(|r| r.last_changed());
        app.update();
        assert!(app.world().resource::<Hinted>().source.is_none());
        assert!(app.world().resource::<Hinted>().text.is_none());
        assert_eq!(
            app.world()
                .get_resource_ref::<Hinted>()
                .map(|r| r.last_changed()),
            rest_tick,
            "still pointer over dismissed mark writes nothing at rest"
        );

        // Rebuilding the mark in place under the still pointer keeps the hint
        // dismissed until the pointer leaves the replacement too.
        app.world_mut().despawn(bell);
        let bell_rebuilt = app
            .world_mut()
            .spawn(HoverHint("Notifications".to_string()))
            .id();
        {
            let mut map = app.world_mut().resource_mut::<HoverMap>();
            map.0.clear();
            map.0
                .entry(PointerId::Mouse)
                .or_default()
                .insert(bell_rebuilt, hit.clone());
        }
        app.world_mut().write_message(aim_at(
            bell_rebuilt,
            crate::test_pointer::Over { hit: hit.clone() },
        ));
        app.update();
        assert!(app.world().resource::<Hinted>().source.is_none());
        assert!(app.world().resource::<Hinted>().text.is_none());

        // Leaving the mark clears dismissal; re-entering brings the hint back.
        app.world_mut().resource_mut::<HoverMap>().0.clear();
        app.world_mut().write_message(aim_at(
            bell_rebuilt,
            crate::test_pointer::Out { hit: hit.clone() },
        ));
        app.update();
        assert!(app.world().resource::<Hinted>().source.is_none());

        app.world_mut()
            .resource_mut::<HoverMap>()
            .0
            .entry(PointerId::Mouse)
            .or_default()
            .insert(bell_rebuilt, hit.clone());
        app.world_mut()
            .write_message(aim_at(bell_rebuilt, crate::test_pointer::Over { hit }));
        app.update();
        assert_eq!(app.world().resource::<Hinted>().source, Some(bell_rebuilt));
        assert_eq!(
            app.world().resource::<Hinted>().text.as_deref(),
            Some("Notifications")
        );
    }
}
