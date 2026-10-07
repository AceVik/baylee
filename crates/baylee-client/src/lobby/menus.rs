//! The lobby's menus and popovers from a `⋯` or a caret (the shell design,
//! §2.4 Menu; N-3): one open at a time, drawn at the screen's top level so
//! no panel's clip cuts it, spawned hidden, measured, and placed on the next
//! frame under its opener's right edge — flipped up where the window ends.
//!
//! What is open is [`LobbyState::menu`]; a press anywhere else (the scrim
//! behind every menu) closes it.

#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;
use crate::shellkit::controls::Kit;
use crate::shellkit::surfaces::{self, MenuItem};
use crate::shellkit::tokens;

/// Which menu is open.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ShellMenu {
    /// The Decks screen's sort.
    DeckSort,
    /// A deck tile's `⋯`, by its index in the deck list.
    Deck(usize),
    /// A house tile's `⋯`, by its index in the house list.
    House(usize),
    /// The tables list's sort.
    TableSort,
    /// Play the house's difficulty caret.
    Difficulty,
    /// The Play hero's `⋯` (Change deck · Edit) on narrow frames.
    Hero,
    /// A seat's `⋯` in the room (host), by seat.
    Seat(u8),
    /// An empty seat's AI ▾ in the room, by seat.
    SeatAi(u8),
}

/// The entity a menu hangs from: placed under its right edge.
#[derive(Component, Clone, Copy)]
pub(crate) struct MenuAnchor(pub Entity);

/// A menu waiting to be placed (spawned hidden, measured first).
#[derive(Component)]
pub(crate) struct Unplaced;

/// Draws `items` as the open menu hanging from `anchor`, with a scrim that
/// closes it, at the root of the tree.
pub(super) fn draw<'a>(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    anchor: Entity,
    items: impl IntoIterator<Item = MenuItem<'a, Press>>,
) {
    let scrim = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                ..default()
            },
            BackgroundColor(Color::NONE),
            GlobalZIndex(tokens::z::POPOVER - 1),
            Press::Shared(SharedPress::CloseMenu),
        ))
        .id();
    let menu = surfaces::menu(commands, kit, items);
    commands.entity(menu).insert((
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            flex_direction: FlexDirection::Column,
            min_width: kit.m.px(200.0),
            padding: UiRect::axes(px(0), kit.m.px(4.0)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(tokens::RADIUS_CONTROL)),
            ..default()
        },
        MenuAnchor(anchor),
        Unplaced,
        Visibility::Hidden,
    ));
    commands.entity(root).add_children(&[scrim, menu]);
}

/// Places each unplaced menu under its anchor's right edge, flipped above it
/// when it would leave the window, and shows it.
pub(super) fn place(
    mut commands: Commands,
    windows: Query<&Window>,
    mut menus: Query<
        (
            Entity,
            &MenuAnchor,
            &ComputedNode,
            &mut Node,
            &mut Visibility,
        ),
        With<Unplaced>,
    >,
    anchors: Query<(&ComputedNode, &UiGlobalTransform), Without<Unplaced>>,
) {
    let Some(window) = windows.iter().next() else {
        return;
    };
    let (width, height) = (window.width(), window.height());
    for (entity, anchor, computed, mut node, mut shown) in &mut menus {
        let Ok((at, place)) = anchors.get(anchor.0) else {
            // The opener is gone: nothing to hang from.
            commands.entity(entity).remove::<Unplaced>();
            continue;
        };
        let scale = computed.inverse_scale_factor;
        let size = computed.size() * scale;
        if size.x <= 0.0 {
            continue; // not measured yet
        }
        let anchor_size = at.size() * at.inverse_scale_factor;
        let mid = place.translation * at.inverse_scale_factor;
        let right = mid.x + anchor_size.x / 2.0;
        let below = mid.y + anchor_size.y / 2.0 + 4.0;
        let above = mid.y - anchor_size.y / 2.0 - 4.0 - size.y;
        let left = (right - size.x).clamp(0.0, (width - size.x).max(0.0));
        let top = if below + size.y > height && above >= 0.0 {
            above
        } else {
            below.min((height - size.y).max(0.0))
        };
        // Two passes: the place is written, laid out on the next frame, and
        // only then shown — so the menu is never drawn where it was spawned.
        if node.left != px(left) || node.top != px(top) {
            node.left = px(left);
            node.top = px(top);
            continue;
        }
        *shown = Visibility::Inherited;
        commands.entity(entity).remove::<Unplaced>();
    }
}

/// One menu item, its press, no key cap.
pub(super) fn item(text: &str, press: Press) -> MenuItem<'_, Press> {
    MenuItem {
        text,
        keys: None,
        destructive: false,
        action: press,
    }
}

/// The destructive item, last after a rule.
pub(super) fn danger(text: &str, press: Press) -> MenuItem<'_, Press> {
    MenuItem {
        text,
        keys: None,
        destructive: true,
        action: press,
    }
}
