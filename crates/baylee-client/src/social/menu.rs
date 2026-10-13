//! A chat line's menu and its name (the chat window design §5, §6): a
//! right-press on a line opens Copy · Reply · Whisper · Add friend · Ignore
//! · Block (Retry and Remove on a line that failed to send); a press on a
//! sender's name writes to them.

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

/// Installs the row menu.
pub(crate) fn install(app: &mut App) {
    app.init_resource::<MenuAt>().add_systems(
        Update,
        (clicks_on_lines, presses, sync)
            .chain()
            .run_if(resource_exists::<UiFonts>),
    );
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
    mouse: Res<ButtonInput<MouseButton>>,
) {
    if ui.row_menu.is_some()
        && mouse.just_pressed(MouseButton::Left)
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
) {
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
                left: px(at.0.x / scale),
                top: px(at.0.y / scale),
                min_width: px(160),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(4)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(6)),
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
                    padding: UiRect::axes(px(10), px(4)),
                    border_radius: BorderRadius::all(px(4)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                Feel::new(Color::NONE),
                children![(
                    Text::new(phrase.text(lang)),
                    tf(&fonts, 12.5),
                    TextColor(inks.ink),
                    Pickable::IGNORE,
                )],
            ))
            .id();
        commands.entity(root).add_child(item);
    }
}
