//! A chat line's menu and its name (the chat window design §5, §6): a
//! right-press on a line opens Copy · Reply · Whisper · Add friend · Ignore
//! · Block (Retry and Remove on a line that failed to send); a press on a
//! sender's name writes to them.

use super::zoom::{z, zpx};
use baylee_client_core::i18n::Lang;
use baylee_client_core::social::{ChatPhrase, Intent, LineKind, LineState};
use bevy::picking::events::{Click, Pointer};
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;

use super::rows::{ChatName, ChatRow, LOBBY_INKS, TABLE_INKS};
use super::window::{ChatSurface, G_PANEL};
use super::{ChatUi, Field, SocialHost};
use crate::ambience::Feel;
use crate::hud::{UiFonts, tf};

/// The row menu's root.
#[derive(Component)]
pub(crate) struct RowMenu;

/// A press in the row menu.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub(crate) enum RowPress {
    Copy,
    Whisper(String),
    AddFriend(String),
    Ignore(String),
    Block(String),
    Retry,
    Remove,
}

/// Where the menu stands, in logical window pixels.
#[derive(Resource, Default)]
pub(crate) struct MenuAt(Vec2);

/// Installs the row menu and the history's selection.
pub(crate) fn install(app: &mut App) {
    app.init_resource::<MenuAt>()
        .init_resource::<Selected>()
        .add_systems(
            Update,
            (
                clicks_on_lines,
                presses,
                sync,
                select_lines.run_if(resource_exists::<Messages<Pointer<Press>>>),
                paint_selection,
                copy_selection,
            )
                .chain()
                .run_if(resource_exists::<UiFonts>),
        );
}

/// A line of the history as copy takes it (§6): chat lines and log lines
/// alike, in the list's order.
#[derive(Component, Clone, Debug)]
pub(crate) struct Copyable {
    /// Its place in the list.
    pub order: usize,
    /// What copy puts on the clipboard for it.
    pub text: String,
    /// Its ground before it was selected.
    base: Option<Color>,
}

impl Copyable {
    /// A row at `order` that copies as `text`.
    pub(crate) fn new(order: usize, text: String) -> Self {
        Self {
            order,
            text,
            base: None,
        }
    }
}

/// The history's selection (SOCIAL §11.10, by whole lines in v1): a press
/// on a line anchors it, dragging over others extends it, `Ctrl`/`Cmd`+`C`
/// copies it.
#[derive(Resource, Default, Debug)]
pub(crate) struct Selected {
    /// The list it is in.
    list: Option<Entity>,
    /// Where the press landed, and where the drag is.
    span: Option<(usize, usize)>,
    /// The primary button is down on the history.
    dragging: bool,
}

impl Selected {
    fn range(&self) -> Option<std::ops::RangeInclusive<usize>> {
        self.span.map(|(a, b)| a.min(b)..=a.max(b))
    }
}

/// The copyable row (and its list) a picked entity is in.
fn row_of(
    mut entity: Entity,
    rows: &Query<&Copyable>,
    parents: &Query<&ChildOf>,
) -> Option<(Entity, usize)> {
    for _ in 0..6 {
        if let Ok(row) = rows.get(entity) {
            let list = parents.get(entity).ok()?.parent();
            return Some((list, row.order));
        }
        entity = parents.get(entity).ok()?.parent();
    }
    None
}

/// Press and drag over the history's lines.
#[allow(clippy::too_many_arguments)] // a Bevy system: the pointer's messages
fn select_lines(
    mut presses: MessageReader<Pointer<bevy::picking::events::Press>>,
    mut overs: MessageReader<Pointer<bevy::picking::events::Over>>,
    rows: Query<&Copyable>,
    parents: Query<&ChildOf>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    mut selected: ResMut<Selected>,
    ui: Res<ChatUi>,
) {
    for press in presses.read() {
        if press.button != PointerButton::Primary {
            continue;
        }
        if let Some((list, order)) = row_of(press.entity, &rows, &parents) {
            let again = selected.list == Some(list) && selected.span == Some((order, order));
            selected.list = Some(list);
            // A second press on the one selected line lets it go.
            selected.span = (!again).then_some((order, order));
            selected.dragging = !again;
        }
    }
    let held = mouse.is_some_and(|m| m.pressed(MouseButton::Left));
    if selected.dragging {
        for over in overs.read() {
            if let Some((list, order)) = row_of(over.entity, &rows, &parents)
                && selected.list == Some(list)
                && let Some((anchor, _)) = selected.span
            {
                selected.span = Some((anchor, order));
            }
        }
        if !held {
            selected.dragging = false;
        }
    } else {
        overs.clear();
    }
    // A tab switched or the panel shut: the lines it named are gone.
    if !ui.open && selected.span.is_some() {
        selected.span = None;
    }
}

/// What the selection was last painted as: its list and its span.
type Painted = (Option<Entity>, Option<(usize, usize)>);

/// The selected lines wear the selection's ground; the others their own.
fn paint_selection(
    selected: Res<Selected>,
    mut rows: Query<(
        Entity,
        &mut Copyable,
        Option<&mut BackgroundColor>,
        &ChildOf,
    )>,
    mut commands: Commands,
    mut painted: Local<Option<Painted>>,
) {
    let now = (selected.list, selected.span);
    // Rows drawn since (a rebuilt list) are painted too.
    let fresh = rows.iter_mut().any(|(_, row, _, _)| row.is_added());
    if *painted == Some(now) && !fresh {
        return;
    }
    *painted = Some(now);
    let range = selected.range();
    let lit = crate::shellkit::tokens::SELECTED.with_alpha(0.5);
    for (entity, mut row, ground, parent) in &mut rows {
        let on = selected.list == Some(parent.parent())
            && range.as_ref().is_some_and(|r| r.contains(&row.order));
        match ground {
            // A row's own ground (a mention's light) is what it wore before
            // it was selected; a row never selected is left alone.
            Some(mut ground) if on => {
                if ground.0 != lit {
                    row.base = Some(ground.0);
                }
                ground.set_if_neq(BackgroundColor(lit));
            }
            Some(mut ground) => {
                if ground.0 == lit {
                    ground.set_if_neq(BackgroundColor(row.base.unwrap_or(Color::NONE)));
                }
            }
            None if on => {
                row.base = Some(Color::NONE);
                commands.entity(entity).insert(BackgroundColor(lit));
            }
            None => {}
        }
    }
}

/// `Ctrl`/`Cmd`+`C` copies the selected lines, one to a line, while the
/// box does not have the caret (it copies its own selection then).
fn copy_selection(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    selected: Res<Selected>,
    rows: Query<(&Copyable, &ChildOf)>,
    ui: Res<ChatUi>,
    mut host: ResMut<SocialHost>,
) {
    let Some(keys) = keys else {
        return;
    };
    let command = keys.any_pressed([
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ]);
    if !command || !keys.just_pressed(KeyCode::KeyC) || ui.typing {
        return;
    }
    let Some(range) = selected.range() else {
        return;
    };
    let mut lines: Vec<(usize, &str)> = rows
        .iter()
        .filter(|(row, parent)| {
            selected.list == Some(parent.parent()) && range.contains(&row.order)
        })
        .map(|(row, _)| (row.order, row.text.as_str()))
        .collect();
    lines.sort_by_key(|(order, _)| *order);
    if !lines.is_empty() {
        host.copy = Some(
            lines
                .into_iter()
                .map(|(_, text)| text)
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }
}

/// The row (or name) a picked entity belongs to, up its parents.
fn lineage<T: Component + Clone>(
    mut entity: Entity,
    found: &Query<&T>,
    parents: &Query<&ChildOf>,
) -> Option<T> {
    for _ in 0..6 {
        if let Ok(t) = found.get(entity) {
            return Some(t.clone());
        }
        entity = parents.get(entity).ok()?.parent();
    }
    None
}

/// A right-press on a line opens its menu; a press on a name writes to
/// them; a left press anywhere but the menu shuts it.
#[allow(clippy::too_many_arguments)] // a Bevy system: the pointer and what it lands on
fn clicks_on_lines(
    mut clicks: MessageReader<Pointer<Click>>,
    rows: Query<&ChatRow>,
    names: Query<&ChatName>,
    parents: Query<&ChildOf>,
    mut host: ResMut<SocialHost>,
    mut ui: ResMut<ChatUi>,
    mut at: ResMut<MenuAt>,
    menus: Query<&RelativeCursorPosition, With<RowMenu>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
) {
    if ui.row_menu.is_some()
        && mouse.is_some_and(|m| m.just_pressed(MouseButton::Left))
        && !menus.iter().any(RelativeCursorPosition::cursor_over)
    {
        ui.row_menu = None;
    }
    for click in clicks.read() {
        match click.button {
            PointerButton::Secondary => {
                if let Some(row) = lineage(click.entity, &rows, &parents) {
                    ui.row_menu = Some(row.key);
                    at.0 = click.pointer_location.position;
                }
            }
            PointerButton::Primary => {
                if let Some(name) = lineage(click.entity, &names, &parents) {
                    host.act(Intent::Whisper { tag: name.tag });
                    ui.typing = true;
                    ui.field = Field::Composer;
                }
            }
            PointerButton::Middle => {}
        }
    }
}

fn presses(
    pressed: Query<(Entity, &Interaction, &RowPress), Changed<Interaction>>,
    mut held: Local<std::collections::HashSet<Entity>>,
    mut host: ResMut<SocialHost>,
    mut ui: ResMut<ChatUi>,
) {
    let clicked: Vec<RowPress> = super::released(&pressed, &mut held);
    let Some(key) = ui.row_menu.clone() else {
        return;
    };
    for press in clicked {
        match press {
            RowPress::Copy => {
                let text = host.machine().and_then(|m| {
                    m.conversation(&key.conv)
                        .and_then(|c| c.line(&key.id))
                        .map(|l| super::rows::plain_text(m, &key.conv, l))
                });
                host.copy = text;
            }
            RowPress::Whisper(tag) => {
                host.act(Intent::Whisper { tag });
                ui.typing = true;
                ui.field = Field::Composer;
            }
            RowPress::AddFriend(handle) => host.act(Intent::AddFriend { handle }),
            RowPress::Ignore(tag) => host.act(Intent::Ignore { tag }),
            RowPress::Block(tag) => host.act(Intent::Block { tag }),
            RowPress::Retry => host.act(Intent::Retry { line: key.clone() }),
            RowPress::Remove => host.act(Intent::Forget { line: key.clone() }),
        }
        ui.row_menu = None;
    }
}

#[allow(clippy::too_many_arguments)]
fn sync(
    mut commands: Commands,
    host: Res<SocialHost>,
    ui: Res<ChatUi>,
    at: Res<MenuAt>,
    fonts: Res<UiFonts>,
    settings: Option<Res<crate::settings::ClientSettings>>,
    scale: Option<Res<UiScale>>,
    roots: Query<Entity, With<RowMenu>>,
    mut drawn: Local<Option<baylee_client_core::social::LineKey>>,
    zoom: Res<super::zoom::Zoom>,
) {
    let _zoom = super::zoom::Scope::new(zoom.0);
    let line = ui.row_menu.as_ref().and_then(|key| {
        host.machine()
            .and_then(|m| {
                m.conversation(&key.conv)
                    .and_then(|c| c.line(&key.id))
                    .cloned()
            })
            .map(|l| (key.clone(), l))
    });
    let Some((key, line)) = line else {
        for root in &roots {
            commands.entity(root).despawn();
        }
        *drawn = None;
        return;
    };
    if drawn.as_ref() == Some(&key) && !roots.is_empty() {
        return;
    }
    *drawn = Some(key);
    for root in &roots {
        commands.entity(root).despawn();
    }
    let Some(machine) = host.machine() else {
        return;
    };
    let lang = settings.as_ref().map_or(Lang::En, |s| Lang::of(&s.lang));
    let inks = if ui.at_table { TABLE_INKS } else { LOBBY_INKS };
    let scale = scale.map_or(1.0, |s| s.0);
    let mut items: Vec<(ChatPhrase, RowPress)> = vec![(ChatPhrase::CopyButton, RowPress::Copy)];
    if let LineKind::Message { from, mine, .. } = &line.kind {
        if *mine {
            if line.state == LineState::Failed {
                items.push((ChatPhrase::RetryButton, RowPress::Retry));
                items.push((ChatPhrase::RemoveButton, RowPress::Remove));
            }
        } else {
            items.push((
                ChatPhrase::WhisperButton,
                RowPress::Whisper(from.tag.clone()),
            ));
            if !machine.contacts().is_friend(&from.tag) && !machine.me().guest {
                items.push((
                    ChatPhrase::AddFriendButton,
                    RowPress::AddFriend(from.handle.clone()),
                ));
            }
            items.push((ChatPhrase::IgnoreButton, RowPress::Ignore(from.tag.clone())));
            items.push((ChatPhrase::BlockButton, RowPress::Block(from.tag.clone())));
        }
    }
    let root = commands
        .spawn((
            RowMenu,
            ChatSurface,
            RelativeCursorPosition::default(),
            Node {
                position_type: PositionType::Absolute,
                left: zpx(at.0.x / scale),
                top: zpx(at.0.y / scale),
                min_width: zpx(160.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(zpx(4.0)),
                border: UiRect::all(zpx(1.0)),
                border_radius: BorderRadius::all(zpx(6.0)),
                ..default()
            },
            BackgroundColor(inks.ground),
            BorderColor::all(inks.accent.with_alpha(0.6)),
            GlobalZIndex(G_PANEL + 60),
        ))
        .id();
    for (phrase, press) in items {
        let item = commands
            .spawn((
                Button,
                press,
                Node {
                    padding: UiRect::axes(zpx(10.0), zpx(4.0)),
                    border_radius: BorderRadius::all(zpx(4.0)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                Feel::new(Color::NONE),
                children![(
                    Text::new(phrase.text(lang)),
                    tf(&fonts, z(12.5)),
                    TextColor(inks.ink),
                    Pickable::IGNORE,
                )],
            ))
            .id();
        commands.entity(root).add_child(item);
    }
}
