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
pub(super) struct HoverHint(pub(super) String);

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
}

/// The widest a hint is drawn, in logical pixels, before it wraps.
const HINT_WIDTH: f32 = 300.0;

/// Tracks which mark the pointer is over.
pub(super) fn hint_hovers(
    mut overs: MessageReader<Pointer<Over>>,
    mut outs: MessageReader<Pointer<Out>>,
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
            at = over.pointer_location.position;
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

/// Draws the hinted sentence beside the pointer.
pub(super) fn hint_panel(
    mut commands: Commands,
    hinted: Res<Hinted>,
    existing: Query<(Entity, &HintPanel)>,
    windows: Query<&Window>,
    fonts: Res<UiFonts>,
) {
    if existing
        .iter()
        .any(|(_, panel)| panel.epoch == hinted.epoch)
    {
        return;
    }
    for (entity, _) in existing {
        commands.entity(entity).despawn();
    }
    let Some(text) = hinted.text.clone() else {
        return;
    };
    let canvas = windows.iter().next().map_or(Vec2::new(1280.0, 800.0), |w| {
        Vec2::new(w.width(), w.height())
    });
    let metrics = Metrics::of(canvas.x);
    let width = HINT_WIDTH.min(canvas.x - 16.0).max(80.0);
    // Below and right of the pointer, pulled back inside the window. Above
    // it near the bottom edge, where below would be off screen: three lines
    // of small text is a safe guess at the height, since a panel cannot be
    // measured before it is laid out.
    let left = (hinted.at.x + 12.0).min(canvas.x - width - 8.0).max(8.0);
    let tall = metrics.small * 4.5 + metrics.pad;
    let top = if hinted.at.y + 20.0 + tall < canvas.y {
        hinted.at.y + 20.0
    } else {
        (hinted.at.y - tall - 12.0).max(8.0)
    };
    let panel = commands
        .spawn((
            HintPanel {
                epoch: hinted.epoch,
            },
            Node {
                position_type: PositionType::Absolute,
                left: px(left),
                top: px(top),
                max_width: px(width),
                padding: UiRect::axes(px(metrics.pad * 0.8), px(metrics.pad * 0.5)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(8)),
                ..default()
            },
            BackgroundColor(palette::PANEL_LIT),
            BorderColor::all(palette::DOCK_EDGE.with_alpha(0.45)),
            soft_shadow(),
            GlobalZIndex(700),
            // A hint must never eat the click meant for what is under it.
            Pickable::IGNORE,
        ))
        .id();
    let words = commands
        .spawn((
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
