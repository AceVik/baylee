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
use crate::shellkit::{Role, tokens};

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
    /// A Replays row's `⋯`, by its index in the list.
    Replay(usize),
}

/// The entity a menu hangs from: placed under its right edge.
#[derive(Component, Clone, Copy)]
pub(crate) struct MenuAnchor(pub Entity);

/// A menu waiting to be placed (spawned hidden, measured first).
#[derive(Component)]
pub(crate) struct Unplaced;

#[derive(Component, Default)]
pub(super) struct Opening(f32);

/// Remembers the open menu's animation progress across UI rebuilds so that
/// keyboard navigation or background updates do not restart its arrival motion.
#[derive(Resource, Default)]
pub(crate) struct MenuMemory {
    pub(super) menu: Option<ShellMenu>,
    pub(super) opening: f32,
}

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
    super::orders::items_of(commands, menu, &super::orders::MENU, "items");
    commands.entity(menu).insert((
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            flex_direction: FlexDirection::Column,
            min_width: kit.m.px(240.0),
            row_gap: kit.m.px(3.0),
            padding: UiRect::all(kit.m.px(8.0)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(tokens::RADIUS_CONTROL)),
            ..default()
        },
        MenuAnchor(anchor),
        Unplaced,
        Opening::default(),
        UiTransform::from_translation(Val2::px(0.0, -5.0)),
        Visibility::Hidden,
    ));
    commands.entity(root).add_children(&[scrim, menu]);
}

/// Places each unplaced menu under its anchor's right edge, flipped above it
/// when it would leave the window, and shows it in the same frame.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(super) fn place(
    mut commands: Commands,
    (windows, ui): (Query<&Window>, Option<Res<UiScale>>),
    mut menus: Query<
        (
            Entity,
            &MenuAnchor,
            &ComputedNode,
            &mut Node,
            &mut Visibility,
            &UiTransform,
        ),
        With<Unplaced>,
    >,
    children: Query<&Children>,
    mut tree: ParamSet<(
        Query<(&ComputedNode, &UiGlobalTransform), Without<Unplaced>>,
        Query<&mut UiGlobalTransform>,
    )>,
) {
    let Some(window) = windows.iter().next() else {
        return;
    };
    let (width, height) = crate::hud::scale::desktop_dimensions(window, ui.as_deref());
    for (entity, anchor, computed, mut node, mut shown, transform) in &mut menus {
        let anchor_info = {
            let anchors = tree.p0();
            anchors.get(anchor.0).ok().map(|(at, place)| {
                (
                    at.size() * at.inverse_scale_factor,
                    place.translation * at.inverse_scale_factor,
                )
            })
        };
        let Some((anchor_size, mid)) = anchor_info else {
            // The opener is gone: nothing to hang from.
            commands.entity(entity).remove::<Unplaced>();
            continue;
        };
        let scale = computed.inverse_scale_factor;
        let size = computed.size() * scale;
        if size.x <= 0.0 {
            continue; // not measured yet
        }
        let right = mid.x + anchor_size.x / 2.0;
        let below = mid.y + anchor_size.y / 2.0 + 4.0;
        let above = mid.y - anchor_size.y / 2.0 - 4.0 - size.y;
        let left = (right - size.x).clamp(0.0, (width - size.x).max(0.0));
        let top = if below + size.y > height && above >= 0.0 {
            above
        } else {
            below.min((height - size.y).max(0.0))
        };
        if node.left != px(left) || node.top != px(top) {
            node.left = px(left);
            node.top = px(top);
            if scale > 0.0 {
                let mut transforms = tree.p1();
                if let Ok(cur_at) = transforms.get(entity) {
                    let cur_min = cur_at.translation * scale - size * 0.5;
                    let arrival = match transform.translation.y {
                        Val::Px(y) => y,
                        _ => 0.0,
                    };
                    let delta = (Vec2::new(left, top + arrival) - cur_min) / scale;
                    for target in std::iter::once(entity).chain(children.iter_descendants(entity)) {
                        if let Ok(mut t) = transforms.get_mut(target) {
                            *t = UiGlobalTransform::from(bevy::math::Affine2 {
                                matrix2: t.matrix2,
                                translation: t.translation + delta,
                            });
                        }
                    }
                }
            }
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

/// Gentle arrival after placement, independent of the display refresh rate.
pub(super) fn animate(
    time: Res<Time>,
    prefs: Res<crate::prefs::Prefs>,
    state: Res<LobbyState>,
    mut memory: ResMut<MenuMemory>,
    mut menus: Query<(&mut Opening, &mut UiTransform, &Visibility)>,
) {
    if memory.menu != state.menu {
        memory.menu = state.menu;
        memory.opening = 0.0;
    }
    for (mut opening, mut transform, visible) in &mut menus {
        if *visible == Visibility::Hidden {
            opening.0 = memory.opening;
        }
        let next = if prefs.all().reduce_motion {
            1.0
        } else if *visible == Visibility::Hidden {
            opening.0
        } else {
            opening.0 + (1.0 - opening.0) * (1.0 - (-24.0 * time.delta_secs()).exp())
        };
        opening.0 = if next > 0.995 { 1.0 } else { next };
        memory.opening = opening.0;
        let translation = Val2::px(0.0, -5.0 * (1.0 - opening.0));
        if transform.translation != translation {
            transform.translation = translation;
        }
    }
}

/// Mark the currently selected option through both fill and a check mark.
pub(super) fn selected(commands: &mut Commands, wanted: Press) {
    commands.queue(move |world: &mut World| {
        let row = world
            .query::<(Entity, &Press, &Role)>()
            .iter(world)
            .find(|(_, action, role)| **action == wanted && **role == Role::MenuItem)
            .map(|(id, _, _)| id);
        let Some(row) = row else {
            return;
        };
        world.entity_mut(row).insert((
            BackgroundColor(tokens::SELECTED),
            crate::ambience::Feel::tinting_to(tokens::SELECTED, tokens::PRIMARY),
        ));
        let icon = world
            .get::<Children>(row)
            .and_then(|children| children.iter().next())
            .and_then(|lead| world.get::<Children>(lead))
            .and_then(|children| children.iter().next());
        if let Some(icon) = icon
            && let Some(mut text) = world.get_mut::<Text>(icon)
        {
            text.0 = "\u{f00c}".into();
        }
    });
}

#[cfg(test)]
mod tests;
