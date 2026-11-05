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

/// The mark the pointer is over, and where the pointer was.
#[derive(Resource, Default)]
pub(super) struct Hinted {
    text: Option<String>,
    source: Option<Entity>,
    at: Vec2,
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

/// Tracks which mark the pointer is over.
pub(super) fn hint_hovers(
    mut overs: MessageReader<PointerOver>,
    mut outs: MessageReader<PointerOut>,
    hints: Query<&HoverHint>,
    parents: Query<&ChildOf>,
    mut hinted: ResMut<Hinted>,
) {
    let mut text = hinted.text.clone();
    let mut source = hinted.source;
    let mut at = hinted.at;
    // A rebuild takes the mark away under a still pointer, and no `Out`
    // says so.
    if source.is_some_and(|entity| hints.get(entity).is_err()) {
        text = None;
        source = None;
    }
    for out in outs.read() {
        if lineage(out.entity, &hints, &parents).is_some_and(|e| Some(e) == source) {
            text = None;
            source = None;
        }
    }
    for over in overs.read() {
        if let Some(entity) = lineage(over.entity, &hints, &parents)
            && let Ok(hint) = hints.get(entity)
        {
            source = Some(entity);
            text = Some(hint.0.clone());
            at = over.pointer.position;
        }
    }
    if text != hinted.text || source != hinted.source {
        hinted.text = text;
        hinted.source = source;
        hinted.at = at;
        hinted.epoch = hinted.epoch.wrapping_add(1);
    }
}

/// The nearest entity at or above this one that carries a hint.
fn lineage(entity: Entity, hints: &Query<&HoverHint>, parents: &Query<&ChildOf>) -> Option<Entity> {
    let mut current = Some(entity);
    while let Some(e) = current {
        if hints.contains(e) {
            return Some(e);
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
    fonts: Res<UiFonts>,
    ui: Option<Res<UiScale>>,
) {
    if hinted.source.is_some_and(|e| hints.get(e).is_err()) {
        hinted.text = None;
        hinted.source = None;
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
        commands.entity(entity).despawn();
    }
    let Some(text) = hinted.text.clone() else {
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
            let inv = c.inverse_scale_factor() / ui_scale;
            let center = t.translation * inv;
            let size = c.size() * inv;
            (center, size)
        });

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
            GlobalZIndex(700),
            // A hint must never eat the click meant for what is under it.
            Pickable::IGNORE,
        ))
        .id();
    let words = commands
        .spawn((
            HintPanelWords,
            Text::new(text),
            tf(&fonts, metrics.small),
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
        commands.entity(entity).despawn();
    }
    *hinted = Hinted::default();
}
