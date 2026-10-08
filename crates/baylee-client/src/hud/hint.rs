//! A control's name, in words, under a pointer that rests on it.
//!
//! The players' chips and the seats' plates say what they say in icons and
//! numbers (the owner, 08.10.2026: *"compact icons + numbers, tooltips /
//! accessible names"*). [`Hint`] is the sentence behind them — the seat's
//! name, life, every count and the floating mana in words
//! (`client_core::seatplate::SeatPlate::describe`) — and is what the pointer
//! is told when it rests on the control, what `/state` reports as the
//! control's name, and what a screen reader would be handed.
//!
//! A seat's hint ([`HintSeat`]) also says the two states its top-edge lines
//! show — its turn, the table waiting on it — while they hold, read when the
//! bubble opens, so the lines' meaning is never colour alone.
//!
//! One bubble serves every hint. It is placed once, when the hovered control
//! changes, beside where the pointer entered it, and then left alone: a
//! tooltip that chased the pointer would relayout the interface on every
//! frame the pointer moved. Under a finger nothing hovers, so nothing shows.

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;

/// The words a control is named by.
#[derive(Component, Clone, PartialEq, Eq, Debug)]
pub struct Hint(pub String);

/// The one bubble that says the hovered control's [`Hint`].
#[derive(Component, Default)]
pub struct HintBubble {
    /// The control it is saying, so a pointer resting on it costs nothing.
    shown: Option<Entity>,
}

/// The bubble's text size.
const HINT_PT: f32 = 11.0;

/// Where the bubble stands from the pointer.
const HINT_OFFSET: Vec2 = Vec2::new(14.0, 20.0);

/// The widest the bubble grows before it wraps.
const HINT_MAX_W: f32 = 340.0;

/// Shows the hovered control's hint, and hides it when nothing named is
/// under the pointer.
pub fn show_hint(
    mut commands: Commands,
    fonts: Res<UiFonts>,
    windows: Query<&Window>,
    hints: Query<(
        Entity,
        &Hint,
        &bevy::picking::hover::PickingInteraction,
        Option<&HintSeat>,
    )>,
    (duel, settings): (
        Option<Res<Duel>>,
        Option<Res<crate::settings::ClientSettings>>,
    ),
    mut bubble: Query<(
        Entity,
        &mut HintBubble,
        &mut Node,
        &mut Visibility,
        &Children,
    )>,
    mut texts: Query<&mut Text>,
) {
    let hovered = hints
        .iter()
        .find(|(_, _, i, _)| **i == bevy::picking::hover::PickingInteraction::Hovered);
    let Ok((_, mut state, mut node, mut seen, children)) = bubble.single_mut() else {
        if hovered.is_some() {
            spawn_bubble(&mut commands, &fonts);
        }
        return;
    };
    let Some((control, hint, _, seat)) = hovered else {
        if state.shown.is_some() {
            state.shown = None;
            *seen = Visibility::Hidden;
        }
        return;
    };
    if state.shown == Some(control) {
        return;
    }
    state.shown = Some(control);
    if let Some(mut text) = children.first().and_then(|c| texts.get_mut(*c).ok()) {
        text.0.clone_from(&hint.0);
        let view = duel.as_ref().and_then(|d| d.view.as_ref());
        if let (Some(HintSeat(player)), Some(view)) = (seat, view) {
            let lang = settings.as_ref().map_or(Lang::En, |s| Lang::of(&s.lang));
            for kind in [TagKind::Turn, TagKind::Priority] {
                if kind.shows(view, *player) {
                    text.0.push_str(" · ");
                    text.0.push_str(kind.words().text(lang));
                }
            }
        }
    }
    let (window, pointer) = windows
        .single()
        .ok()
        .map(|w| {
            (
                Vec2::new(w.width(), w.height()),
                w.cursor_position().unwrap_or_default(),
            )
        })
        .unwrap_or_default();
    let at = pointer + HINT_OFFSET;
    // Kept inside the window: a bubble past the right edge flips to the
    // pointer's left, one past the bottom stands above it.
    let left = if at.x + HINT_MAX_W > window.x {
        (pointer.x - HINT_OFFSET.x - HINT_MAX_W).max(EDGE)
    } else {
        at.x
    };
    let top = if at.y + 60.0 > window.y {
        (pointer.y - 60.0).max(EDGE)
    } else {
        at.y
    };
    node.left = px(left);
    node.top = px(top);
    *seen = Visibility::Inherited;
}

/// The bubble, hidden until a hint shows.
fn spawn_bubble(commands: &mut Commands, fonts: &UiFonts) {
    commands.spawn((
        HintBubble::default(),
        crate::table::DuelStage,
        Node {
            position_type: PositionType::Absolute,
            max_width: px(HINT_MAX_W),
            padding: UiRect::axes(px(8), px(5)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        BackgroundColor(palette::DIALOG_LIT.with_alpha(0.96)),
        BorderColor::all(palette::DIALOG_LINE),
        GlobalZIndex(G_HINT),
        Visibility::Hidden,
        Pickable::IGNORE,
        children![(
            Text::new(String::new()),
            tf(fonts, HINT_PT),
            TextColor(palette::DIALOG_INK),
            Pickable::IGNORE,
        )],
    ));
}

/// The bubble's rung: over the table's HUD and its sheets, under nothing a
/// player summons on purpose.
const G_HINT: i32 = 6;
