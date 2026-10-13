//! The friends drawer (the beta.8 social design §11.3): requests, table and
//! group invites, friends by presence (where they are: playing, waiting, at
//! a table), groups, recent players, blocked players, adding by name, the
//! status menu and the door to the chat's privacy settings.
//!
//! One overlay for the lobby and the table, sliding in from the right
//! (`Z_MENU`'s place on the global ladder); `f` is unbound at the table
//! (`Action::Friends`), `/friends` and the chat's head open it too.

use baylee_client_core::i18n::Lang;
use baylee_client_core::social::{ChatPhrase, ChatTab, Intent, Social};
use baylee_protocol::social::{Place, Shown, Status};
use bevy::prelude::*;
use bevy::ui::{RelativeCursorPosition, ScrollPosition};

use super::window::{ChatSurface, G_PANEL};
use super::{ChatUi, Entry, Field, SocialHost, UiAsk};
use crate::ambience::Feel;
use crate::hud::{UiFonts, glyph, icon_tf, palette, tf, tf_bold, tf_italic};
use crate::shellkit::tokens;

/// The drawer's width.
const WIDTH: f32 = 340.0;

/// The drawer's root.
#[derive(Component)]
pub(crate) struct Drawer;

/// The floating friends button (lobby, bottom-right beside the chat pill).
#[derive(Component)]
pub(crate) struct FriendsDoor;

/// A press in the drawer.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub(crate) enum DrawerPress {
    Toggle,
    Close,
    Find,
    Add(String),
    Accept(String),
    Decline(String),
    Cancel(String),
    Whisper(String),
    Watch(String),
    InviteToTable(String),
    More(String),
    Nickname(String),
    Safety(String),
    Unfriend(String),
    Block(String),
    Unblock(String),
    AddRecent(String),
    JoinGroup(String),
    DeclineGroup(String),
    OpenGroup(String),
    GroupMore(String),
    RenameGroup(String),
    InviteToGroup(String),
    LeaveGroup(String),
    DeleteGroup(String),
    AnswerInvite(String, bool),
    NewGroup,
    Status,
    Privacy,
    Entry,
    EntryOk,
    EntryCancel,
}

/// Installs the drawer.
pub(crate) fn install(app: &mut App) {
    app.add_systems(
        Update,
        (asks, presses, sync_door, sync_drawer)
            .chain()
            .run_if(resource_exists::<UiFonts>),
    );
}

/// What the machine asked of the shell: open a conversation, the drawer,
/// join or watch a table.
fn asks(
    mut host: ResMut<SocialHost>,
    mut ui: ResMut<ChatUi>,
    mut duel: Option<ResMut<crate::Duel>>,
    lobby: Option<ResMut<crate::lobby::LobbyState>>,
    mut joins: Option<ResMut<crate::lobby::ChatJoins>>,
    phase: Option<Res<State<crate::DuelPhase>>>,
) {
    if host.asks.is_empty() {
        return;
    }
    let asks = std::mem::take(&mut host.asks);
    let at_table = phase.is_some_and(|p| *p.get() != crate::DuelPhase::Closed);
    let mut lobby = lobby;
    for ask in asks {
        match ask {
            UiAsk::Open(conv) => {
                let tab = ChatTab::of(&conv);
                if let baylee_protocol::social::Conv::Group { id } = &conv {
                    ui.shown_group = Some(id.clone());
                }
                ui.open = true;
                ui.minimised = false;
                if let Some(duel) = duel.as_deref_mut().filter(|_| at_table) {
                    duel.log_open = true;
                }
                super::window::select_tab(&mut host, &mut ui, tab);
            }
            UiAsk::Drawer => ui.drawer = true,
            UiAsk::Join { game, invite } => {
                if let Some(joins) = joins.as_deref_mut()
                    && !at_table
                {
                    joins.0.push((game, invite));
                }
            }
            UiAsk::Watch { game } => {
                if let Some(lobby) = lobby.as_deref_mut()
                    && !at_table
                {
                    lobby.lobby.watch_table(&game);
                }
            }
        }
    }
}

/// The drawer's presses.
#[allow(clippy::too_many_lines)] // one arm per control
fn presses(
    pressed: Query<(Entity, &Interaction, &DrawerPress), Changed<Interaction>>,
    mut held: Local<std::collections::HashSet<Entity>>,
    mut host: ResMut<SocialHost>,
    mut ui: ResMut<ChatUi>,
    mut more: Local<Option<String>>,
) {
    let clicked: Vec<DrawerPress> = super::released(&pressed, &mut held);
    let ui = &mut *ui;
    for press in clicked {
        match press {
            DrawerPress::Toggle => ui.drawer = !ui.drawer,
            DrawerPress::Close => {
                ui.drawer = false;
                if ui.field == Field::Find {
                    ui.typing = false;
                }
            }
            DrawerPress::Find => {
                ui.typing = true;
                ui.field = Field::Find;
            }
            DrawerPress::Add(handle) => {
                host.act(Intent::AddFriend { handle });
                ui.find.clear();
            }
            DrawerPress::Accept(tag) => host.act(Intent::Accept { tag }),
            DrawerPress::Decline(tag) => host.act(Intent::Decline { tag }),
            DrawerPress::Cancel(tag) => host.act(Intent::Cancel { tag }),
            DrawerPress::Whisper(tag) => {
                host.act(Intent::Whisper { tag });
                ui.typing = true;
                ui.field = Field::Composer;
            }
            DrawerPress::Watch(tag) => host.act(Intent::Watch { tag }),
            DrawerPress::InviteToTable(tag) => host.act(Intent::InviteToTable { tag }),
            DrawerPress::More(tag) => {
                ui.row_menu = None;
                *more = if more.as_deref() == Some(tag.as_str()) {
                    None
                } else {
                    Some(tag)
                };
                ui.drawer_menu = more.clone();
            }
            DrawerPress::Nickname(tag) => {
                let current = host
                    .machine()
                    .and_then(|m| m.nickname(&tag).map(str::to_string))
                    .unwrap_or_default();
                ui.entry = Some((
                    Entry::Nickname(tag),
                    baylee_client_core::textbuf::TextBuffer::new(&current),
                ));
                ui.field = Field::Entry;
                ui.typing = true;
                *more = None;
                ui.drawer_menu = None;
            }
            DrawerPress::Safety(tag) => {
                ui.sheet = Some(super::sheet::Sheet::Safety(tag));
                *more = None;
                ui.drawer_menu = None;
            }
            DrawerPress::Unfriend(tag) => {
                host.act(Intent::Unfriend { tag });
                *more = None;
                ui.drawer_menu = None;
            }
            DrawerPress::Block(tag) => {
                host.act(Intent::Block { tag });
                *more = None;
                ui.drawer_menu = None;
            }
            DrawerPress::Unblock(tag) => host.act(Intent::Unblock { tag }),
            DrawerPress::AddRecent(handle) => host.act(Intent::AddFriend { handle }),
            DrawerPress::JoinGroup(id) => host.act(Intent::JoinGroup { id }),
            DrawerPress::DeclineGroup(id) => host.act(Intent::DeclineGroup { id }),
            DrawerPress::OpenGroup(id) => {
                ui.shown_group = Some(id.clone());
                host.asks
                    .push(UiAsk::Open(baylee_protocol::social::Conv::Group { id }));
            }
            DrawerPress::GroupMore(id) => {
                let key = format!("grp:{id}");
                *more = if more.as_deref() == Some(key.as_str()) {
                    None
                } else {
                    Some(key)
                };
                ui.drawer_menu = more.clone();
            }
            DrawerPress::RenameGroup(id) => {
                ui.entry = Some((
                    Entry::Rename(id),
                    baylee_client_core::textbuf::TextBuffer::default(),
                ));
                ui.field = Field::Entry;
                ui.typing = true;
                *more = None;
                ui.drawer_menu = None;
            }
            DrawerPress::InviteToGroup(id) => {
                ui.entry = Some((
                    Entry::InviteToGroup(id),
                    baylee_client_core::textbuf::TextBuffer::default(),
                ));
                ui.field = Field::Entry;
                ui.typing = true;
                *more = None;
                ui.drawer_menu = None;
            }
            DrawerPress::LeaveGroup(id) => {
                host.act(Intent::LeaveGroup { id });
                *more = None;
                ui.drawer_menu = None;
            }
            DrawerPress::DeleteGroup(id) => {
                host.act(Intent::DeleteGroup { id });
                *more = None;
                ui.drawer_menu = None;
            }
            DrawerPress::AnswerInvite(id, accept) => host.act(Intent::AnswerInvite { id, accept }),
            DrawerPress::NewGroup => {
                ui.entry = Some((
                    Entry::NewGroup,
                    baylee_client_core::textbuf::TextBuffer::default(),
                ));
                ui.field = Field::Entry;
                ui.typing = true;
            }
            DrawerPress::Status => {
                let next = match host.machine().map(Social::status) {
                    Some(Status::Online) => Status::Away,
                    Some(Status::Away) => Status::Dnd,
                    Some(Status::Dnd) => Status::Invisible,
                    _ => Status::Online,
                };
                host.act(Intent::SetStatus(next));
            }
            DrawerPress::Privacy => ui.sheet = Some(super::sheet::Sheet::Privacy),
            DrawerPress::Entry => {
                ui.typing = true;
                ui.field = Field::Entry;
            }
            DrawerPress::EntryOk => {
                ui.field = Field::Entry;
                super::keys::answer_box(&mut host, ui);
            }
            DrawerPress::EntryCancel => {
                ui.entry = None;
                ui.typing = false;
                ui.field = Field::Composer;
            }
        }
    }
}

/// The friends button beside the lobby's chat pill: online friends and a
/// dot for anything waiting (requests, invites).
fn sync_door(
    mut commands: Commands,
    host: Res<SocialHost>,
    ui: Res<ChatUi>,
    fonts: Res<UiFonts>,
    phase: Option<Res<State<crate::DuelPhase>>>,
    lobby: Option<Res<crate::lobby::LobbyState>>,
    doors: Query<(Entity, &mut Visibility), With<FriendsDoor>>,
    mut drawn: Local<Option<(usize, bool, bool)>>,
) {
    let at_table = phase.is_some_and(|p| *p.get() != crate::DuelPhase::Closed);
    let shell = lobby.is_some_and(|l| {
        !matches!(
            l.lobby.screen(),
            baylee_client_core::lobby::Screen::SignIn { .. }
        )
    });
    // The panel's head has the friends button while the panel stands.
    let panel = ui.open && !ui.minimised;
    let machine = host
        .machine()
        .filter(|m| shell && !at_table && !panel && !m.me().guest);
    let mut doors = doors;
    let Some(machine) = machine else {
        for (_, mut seen) in &mut doors {
            seen.set_if_neq(Visibility::Hidden);
        }
        return;
    };
    let online = machine.contacts().online_count();
    let waiting = machine.contacts().waiting();
    let now = (online, waiting, ui.drawer);
    let door = if let Some((door, mut seen)) = doors.iter_mut().next() {
        seen.set_if_neq(Visibility::Visible);
        door
    } else {
        *drawn = None;
        commands
            .spawn((
                FriendsDoor,
                ChatSurface,
                RelativeCursorPosition::default(),
                Button,
                DrawerPress::Toggle,
                Node {
                    position_type: PositionType::Absolute,
                    bottom: px(12),
                    right: px(12.0 + super::window::LOBBY_PILL_ROOM),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(8),
                    padding: UiRect::axes(px(14), px(8)),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(18)),
                    ..default()
                },
                BackgroundColor(super::rows::LOBBY_GROUND),
                BorderColor::all(tokens::BORDER),
                Feel::new(super::rows::LOBBY_GROUND),
                GlobalZIndex(G_PANEL + 1),
            ))
            .id()
    };
    if *drawn == Some(now) {
        return;
    }
    *drawn = Some(now);
    commands.entity(door).despawn_children();
    let icon = commands
        .spawn((
            Text::new(glyph::FRIENDS.to_string()),
            icon_tf(&fonts, 13.0),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    let count = commands
        .spawn((
            Text::new(online.to_string()),
            tf_bold(&fonts, 13.0),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(door).add_children(&[icon, count]);
    if waiting {
        let dot = commands
            .spawn((
                Node {
                    width: px(8),
                    height: px(8),
                    border_radius: BorderRadius::all(px(4)),
                    ..default()
                },
                BackgroundColor(super::rows::WHISPER_INK),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(door).add_child(dot);
    }
}

/// A presence dot: shape and colour (§11.2).
fn dot(commands: &mut Commands, shown: Shown) -> Entity {
    let (fill, ring) = match shown {
        Shown::Online => (
            Color::srgb(0.420, 0.816, 0.541),
            Color::srgb(0.420, 0.816, 0.541),
        ),
        Shown::Away => (palette::CANDLE.with_alpha(0.5), palette::CANDLE),
        Shown::Dnd => (Color::NONE, palette::DANGER),
        Shown::Offline => (Color::NONE, tokens::MUTED),
    };
    commands
        .spawn((
            Node {
                width: px(9),
                height: px(9),
                border: UiRect::all(px(1.5)),
                border_radius: BorderRadius::all(px(5)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(fill),
            BorderColor::all(ring),
            Pickable::IGNORE,
        ))
        .id()
}

/// Where a friend is, in words.
pub(crate) fn place_words(shown: Shown, place: Option<&Place>, lang: Lang) -> String {
    if shown == Shown::Offline {
        return ChatPhrase::PlaceOffline.text(lang).to_string();
    }
    let status = match shown {
        Shown::Away => Some(ChatPhrase::StatusAway.text(lang)),
        Shown::Dnd => Some(ChatPhrase::StatusDnd.text(lang)),
        _ => None,
    };
    let place = match place {
        Some(Place::Lobby) | None => ChatPhrase::PlaceLobby.text(lang).to_string(),
        Some(Place::Room { name, .. }) => ChatPhrase::PlaceRoom.fill(lang, &[name]),
        Some(Place::Playing { name, .. }) => ChatPhrase::PlacePlaying.fill(lang, &[name]),
        Some(Place::Watching { name, .. }) => ChatPhrase::PlaceWatching.fill(lang, &[name]),
        Some(Place::Building) => ChatPhrase::PlaceBuilding.text(lang).to_string(),
        Some(Place::House) => ChatPhrase::PlaceHouse.text(lang).to_string(),
    };
    match status {
        Some(status) => format!("{status} · {place}"),
        None => place,
    }
}

fn small_button(
    commands: &mut Commands,
    fonts: &UiFonts,
    label: &str,
    press: DrawerPress,
) -> Entity {
    commands
        .spawn((
            Button,
            press,
            Node {
                padding: UiRect::axes(px(7), px(2)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(Color::NONE),
            BorderColor::all(tokens::BORDER),
            Feel::new(Color::NONE),
            children![(
                Text::new(label.to_string()),
                tf(fonts, 11.0),
                TextColor(tokens::INK),
                Pickable::IGNORE,
            )],
        ))
        .id()
}

fn icon_button(
    commands: &mut Commands,
    fonts: &UiFonts,
    glyph: char,
    press: DrawerPress,
) -> Entity {
    commands
        .spawn((
            Button,
            press,
            Node {
                width: px(22),
                height: px(22),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px(4)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(Color::NONE),
            Feel::new(Color::NONE),
            children![(
                Text::new(glyph.to_string()),
                icon_tf(fonts, 10.0),
                TextColor(tokens::MUTED),
                Pickable::IGNORE,
            )],
        ))
        .id()
}

fn heading(commands: &mut Commands, fonts: &UiFonts, text: String) -> Entity {
    commands
        .spawn((
            Text::new(text.to_uppercase()),
            tf_bold(fonts, 10.5),
            TextColor(tokens::MUTED),
            Node {
                margin: UiRect::new(px(0), px(0), px(10), px(4)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
}

fn row(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(6),
                padding: UiRect::vertical(px(3)),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
}

fn label(
    commands: &mut Commands,
    fonts: &UiFonts,
    text: String,
    colour: Color,
    grow: bool,
) -> Entity {
    commands
        .spawn((
            Text::new(text),
            tf(fonts, 12.0),
            TextColor(colour),
            Node {
                flex_grow: if grow { 1.0 } else { 0.0 },
                min_width: px(0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
}

/// A one-line box: its text with a caret when it has the keys.
pub(crate) fn text_box(
    commands: &mut Commands,
    fonts: &UiFonts,
    buffer: &baylee_client_core::textbuf::TextBuffer,
    focused: bool,
    placeholder: &str,
    press: DrawerPress,
) -> Entity {
    let text = buffer.text();
    let (before, after) = text.split_at(buffer.cursor().min(text.len()));
    let field = commands
        .spawn((
            Button,
            press,
            Text::default(),
            tf(fonts, 12.0),
            TextColor(tokens::INK),
            Node {
                flex_grow: 1.0,
                padding: UiRect::axes(px(8), px(5)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(5)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.25)),
            BorderColor::all(if focused {
                tokens::ACCENT
            } else {
                tokens::BORDER
            }),
        ))
        .id();
    let spans: Vec<(String, Color, bool)> = if text.is_empty() && !focused {
        vec![(placeholder.to_string(), tokens::MUTED, true)]
    } else if focused {
        vec![
            (before.to_string(), tokens::INK, false),
            ("\u{258f}".to_string(), tokens::ACCENT, false),
            (after.to_string(), tokens::INK, false),
        ]
    } else {
        vec![(text.to_string(), tokens::INK, false)]
    };
    for (part, colour, italic) in spans {
        if part.is_empty() {
            continue;
        }
        let font = if italic {
            tf_italic(fonts, 12.0)
        } else {
            tf(fonts, 12.0)
        };
        let span = commands
            .spawn((TextSpan::new(part), font, TextColor(colour)))
            .id();
        commands.entity(field).add_child(span);
    }
    field
}

/// The drawer itself: rebuilt when the machine or what the drawer shows
/// changed.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // one section after another
fn sync_drawer(
    mut commands: Commands,
    host: Res<SocialHost>,
    ui: Res<ChatUi>,
    fonts: Res<UiFonts>,
    settings: Option<Res<crate::settings::ClientSettings>>,
    phase: Option<Res<State<crate::DuelPhase>>>,
    roots: Query<(Entity, &mut Visibility), With<Drawer>>,
    scrolls: Query<&ScrollPosition, With<DrawerList>>,
    mut drawn: Local<Option<(u64, String, String, bool, Option<String>, String)>>,
) {
    let mut roots = roots;
    let Some(machine) = host.machine().filter(|_| ui.drawer) else {
        for (_, mut seen) in &mut roots {
            seen.set_if_neq(Visibility::Hidden);
        }
        *drawn = None;
        return;
    };
    let lang = settings.as_ref().map_or(Lang::En, |s| Lang::of(&s.lang));
    let at_table = phase.is_some_and(|p| *p.get() != crate::DuelPhase::Closed);
    let entry_text = ui
        .entry
        .as_ref()
        .map(|(e, b)| format!("{e:?}{}{}", b.text(), b.cursor()))
        .unwrap_or_default();
    let key = (
        machine.revision(),
        format!("{}{}", ui.find.text(), ui.find.cursor()),
        entry_text,
        ui.typing,
        ui.drawer_menu.clone(),
        format!("{:?}", ui.field),
    );
    let root = if let Some((root, mut seen)) = roots.iter_mut().next() {
        seen.set_if_neq(Visibility::Visible);
        root
    } else {
        *drawn = None;
        commands
            .spawn((
                Drawer,
                ChatSurface,
                RelativeCursorPosition::default(),
                Node {
                    position_type: PositionType::Absolute,
                    top: px(0),
                    bottom: px(0),
                    right: px(0),
                    width: px(WIDTH),
                    flex_direction: FlexDirection::Column,
                    border: UiRect::left(px(1)),
                    ..default()
                },
                BackgroundColor(super::rows::LOBBY_GROUND),
                BorderColor::all(tokens::BORDER),
                GlobalZIndex(G_PANEL + 10),
            ))
            .id()
    };
    if drawn.as_ref() == Some(&key) {
        return;
    }
    let keep = scrolls.iter().next().cloned().unwrap_or_default();
    *drawn = Some(key);
    commands.entity(root).despawn_children();
    let me = machine.me();
    // The head.
    let head = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::axes(px(14), px(10)),
                border: UiRect::bottom(px(1)),
                flex_shrink: 0.0,
                ..default()
            },
            BorderColor::all(tokens::BORDER),
        ))
        .id();
    let title = commands
        .spawn((
            Text::new(format!(
                "{} · {}",
                ChatPhrase::FriendsTitle.text(lang),
                me.handle
            )),
            tf_bold(&fonts, 14.0),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    let close = icon_button(&mut commands, &fonts, glyph::CLOSE, DrawerPress::Close);
    commands.entity(head).add_children(&[title, close]);
    // The list.
    let list = commands
        .spawn((
            DrawerList,
            crate::hud::Scrolls,
            keep,
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                min_height: px(0),
                padding: UiRect::axes(px(14), px(8)),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    let mut kids: Vec<Entity> = Vec::new();
    // The entry box, when one is asked for.
    if let Some((entry, buffer)) = ui.entry.as_ref() {
        let words = match entry {
            Entry::NewGroup => ChatPhrase::GroupNameLabel.text(lang).to_string(),
            Entry::Rename(_) => ChatPhrase::RenameGroup.text(lang).to_string(),
            Entry::Nickname(tag) => {
                ChatPhrase::NicknameLabel.fill(lang, &[&machine.person(tag).handle])
            }
            Entry::InviteToGroup(_) => ChatPhrase::InviteLabel.text(lang).to_string(),
        };
        let caption = label(&mut commands, &fonts, words, tokens::MUTED, false);
        let field = text_box(
            &mut commands,
            &fonts,
            buffer,
            ui.typing && ui.field == Field::Entry,
            "",
            DrawerPress::Entry,
        );
        let ok = small_button(
            &mut commands,
            &fonts,
            ChatPhrase::OkButton.text(lang),
            DrawerPress::EntryOk,
        );
        let cancel = small_button(
            &mut commands,
            &fonts,
            ChatPhrase::CancelButton.text(lang),
            DrawerPress::EntryCancel,
        );
        let line = row(&mut commands);
        commands.entity(line).add_children(&[field, ok, cancel]);
        kids.extend([caption, line]);
    }
    // Find or add.
    if !me.guest {
        let field = text_box(
            &mut commands,
            &fonts,
            &ui.find,
            ui.typing && ui.field == Field::Find,
            ChatPhrase::FindOrAdd.text(lang),
            DrawerPress::Find,
        );
        let line = row(&mut commands);
        commands.entity(line).add_child(field);
        kids.push(line);
        let typed = ui.find.text().trim().to_string();
        if typed.contains('#') && typed.len() > 2 {
            let add = small_button(
                &mut commands,
                &fonts,
                &ChatPhrase::AddHandle.fill(lang, &[&typed]),
                DrawerPress::Add(typed),
            );
            kids.push(add);
        }
    }
    let view = machine.friends();
    let filter = ui.find.text().trim().to_lowercase();
    let shown = |handle: &str| {
        filter.is_empty() || filter.contains('#') || handle.to_lowercase().contains(&filter)
    };
    // Requests.
    if !view.incoming.is_empty() || !view.outgoing.is_empty() {
        kids.push(heading(
            &mut commands,
            &fonts,
            format!(
                "{} · {}",
                ChatPhrase::SectionRequests.text(lang),
                view.incoming.len()
            ),
        ));
        for person in view.incoming {
            let line = row(&mut commands);
            let name = label(
                &mut commands,
                &fonts,
                person.handle.clone(),
                tokens::INK,
                true,
            );
            let yes = small_button(
                &mut commands,
                &fonts,
                ChatPhrase::Accept.text(lang),
                DrawerPress::Accept(person.tag.clone()),
            );
            let no = small_button(
                &mut commands,
                &fonts,
                ChatPhrase::Decline.text(lang),
                DrawerPress::Decline(person.tag.clone()),
            );
            commands.entity(line).add_children(&[name, yes, no]);
            kids.push(line);
        }
        for person in view.outgoing {
            let line = row(&mut commands);
            let name = label(
                &mut commands,
                &fonts,
                format!(
                    "{}  · {}",
                    person.handle,
                    ChatPhrase::RequestSent.text(lang)
                ),
                tokens::MUTED,
                true,
            );
            let undo = small_button(
                &mut commands,
                &fonts,
                ChatPhrase::Withdraw.text(lang),
                DrawerPress::Cancel(person.tag.clone()),
            );
            commands.entity(line).add_children(&[name, undo]);
            kids.push(line);
        }
    }
    // Table invites.
    for held in view.invites {
        let invite = &held.invite;
        let phrase = match invite.kind {
            baylee_protocol::social::InviteKind::Join => ChatPhrase::ToastJoinInvite,
            baylee_protocol::social::InviteKind::Watch => ChatPhrase::ToastWatchInvite,
        };
        let line = row(&mut commands);
        let text = label(
            &mut commands,
            &fonts,
            phrase.fill(lang, &[&invite.from.handle, &invite.table.name]),
            tokens::INK,
            true,
        );
        let go = match invite.kind {
            baylee_protocol::social::InviteKind::Join => ChatPhrase::JoinButton,
            baylee_protocol::social::InviteKind::Watch => ChatPhrase::WatchButton,
        };
        let yes = small_button(
            &mut commands,
            &fonts,
            go.text(lang),
            DrawerPress::AnswerInvite(invite.id.clone(), true),
        );
        let no = small_button(
            &mut commands,
            &fonts,
            ChatPhrase::NotNow.text(lang),
            DrawerPress::AnswerInvite(invite.id.clone(), false),
        );
        commands.entity(line).add_children(&[text, yes, no]);
        kids.push(line);
    }
    // Group invites.
    for invite in view.group_invites {
        let line = row(&mut commands);
        let text = label(
            &mut commands,
            &fonts,
            ChatPhrase::GroupInviteFrom.fill(lang, &[&invite.name, &invite.inviter.handle]),
            super::rows::GROUP_INK,
            true,
        );
        let yes = small_button(
            &mut commands,
            &fonts,
            ChatPhrase::Accept.text(lang),
            DrawerPress::JoinGroup(invite.group.clone()),
        );
        let no = small_button(
            &mut commands,
            &fonts,
            ChatPhrase::Decline.text(lang),
            DrawerPress::DeclineGroup(invite.group.clone()),
        );
        commands.entity(line).add_children(&[text, yes, no]);
        kids.push(line);
    }
    let my_table = machine
        .table()
        .filter(|t| !t.watching)
        .map(|t| t.game.clone());
    // Friends: online, then offline.
    let friend_rows = |commands: &mut Commands,
                       kids: &mut Vec<Entity>,
                       friends: &[&baylee_protocol::social::Friend],
                       title: ChatPhrase| {
        let friends: Vec<&&baylee_protocol::social::Friend> =
            friends.iter().filter(|f| shown(&f.person.handle)).collect();
        if friends.is_empty() {
            return;
        }
        kids.push(heading(
            commands,
            &fonts,
            format!("{} · {}", title.text(lang), friends.len()),
        ));
        for friend in friends {
            let tag = friend.person.tag.clone();
            let line = row(commands);
            let presence = dot(commands, friend.presence.status);
            let name = machine.shown_name(&friend.person);
            let shown_name = if name == friend.person.handle {
                name
            } else {
                format!("{name} ({})", friend.person.handle)
            };
            let words = commands
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        flex_grow: 1.0,
                        min_width: px(0),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .id();
            let top = label(commands, &fonts, shown_name, tokens::INK, false);
            let place = commands
                .spawn((
                    Text::new(place_words(
                        friend.presence.status,
                        friend.presence.place.as_ref(),
                        lang,
                    )),
                    tf(&fonts, 10.5),
                    TextColor(tokens::MUTED),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(words).add_children(&[top, place]);
            commands.entity(line).add_children(&[presence, words]);
            let online = friend.presence.status != Shown::Offline;
            if online {
                let whisper = icon_button(
                    commands,
                    &fonts,
                    glyph::CHAT,
                    DrawerPress::Whisper(tag.clone()),
                );
                commands.entity(line).add_child(whisper);
            }
            if let Some(Place::Playing { game: Some(_), .. }) = &friend.presence.place
                && !at_table
            {
                let watch = small_button(
                    commands,
                    &fonts,
                    ChatPhrase::WatchButton.text(lang),
                    DrawerPress::Watch(tag.clone()),
                );
                commands.entity(line).add_child(watch);
            }
            if online && my_table.is_some() {
                let invite = small_button(
                    commands,
                    &fonts,
                    ChatPhrase::InviteToTable.text(lang),
                    DrawerPress::InviteToTable(tag.clone()),
                );
                commands.entity(line).add_child(invite);
            }
            let more = icon_button(
                commands,
                &fonts,
                glyph::MORE,
                DrawerPress::More(tag.clone()),
            );
            commands.entity(line).add_child(more);
            kids.push(line);
            if ui.drawer_menu.as_deref() == Some(tag.as_str()) {
                let menu = row(commands);
                for (phrase, press) in [
                    (ChatPhrase::WhisperButton, DrawerPress::Whisper(tag.clone())),
                    (
                        ChatPhrase::NicknameButton,
                        DrawerPress::Nickname(tag.clone()),
                    ),
                    (ChatPhrase::SafetyButton, DrawerPress::Safety(tag.clone())),
                    (ChatPhrase::RemoveFriend, DrawerPress::Unfriend(tag.clone())),
                    (ChatPhrase::BlockButton, DrawerPress::Block(tag.clone())),
                ] {
                    let b = small_button(commands, &fonts, phrase.text(lang), press);
                    commands.entity(menu).add_child(b);
                }
                kids.push(menu);
            }
        }
    };
    friend_rows(
        &mut commands,
        &mut kids,
        &view.online,
        ChatPhrase::SectionOnline,
    );
    // Groups.
    if !me.guest && !view.groups.is_empty() {
        kids.push(heading(
            &mut commands,
            &fonts,
            format!(
                "{} · {}",
                ChatPhrase::SectionGroups.text(lang),
                view.groups.len()
            ),
        ));
        for group in view.groups {
            let conv = baylee_protocol::social::Conv::Group {
                id: group.id.clone(),
            };
            let unread = machine.conversation(&conv).map_or(0, |c| c.unread);
            let line = row(&mut commands);
            let open = commands
                .spawn((
                    Button,
                    DrawerPress::OpenGroup(group.id.clone()),
                    Node {
                        flex_grow: 1.0,
                        padding: UiRect::vertical(px(2)),
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                    Feel::new(Color::NONE),
                    children![(
                        Text::new(format!("# {}  · {}", group.name, group.members.len())),
                        tf(&fonts, 12.0),
                        TextColor(super::rows::GROUP_INK),
                        Pickable::IGNORE,
                    )],
                ))
                .id();
            commands.entity(line).add_child(open);
            if unread > 0 {
                let pill = super::window::count_pill(
                    &mut commands,
                    &fonts,
                    unread,
                    false,
                    &super::rows::LOBBY_INKS,
                );
                commands.entity(line).add_child(pill);
            }
            let more = icon_button(
                &mut commands,
                &fonts,
                glyph::MORE,
                DrawerPress::GroupMore(group.id.clone()),
            );
            commands.entity(line).add_child(more);
            kids.push(line);
            if ui.drawer_menu.as_deref() == Some(format!("grp:{}", group.id).as_str()) {
                let owner = group
                    .members
                    .iter()
                    .any(|m| m.owner && m.person.tag == me.tag);
                let menu = row(&mut commands);
                let mut items = vec![
                    (
                        ChatPhrase::InviteToGroup,
                        DrawerPress::InviteToGroup(group.id.clone()),
                    ),
                    (
                        ChatPhrase::LeaveGroup,
                        DrawerPress::LeaveGroup(group.id.clone()),
                    ),
                ];
                if owner {
                    items.insert(
                        0,
                        (
                            ChatPhrase::RenameGroup,
                            DrawerPress::RenameGroup(group.id.clone()),
                        ),
                    );
                    items.push((
                        ChatPhrase::DeleteGroup,
                        DrawerPress::DeleteGroup(group.id.clone()),
                    ));
                }
                for (phrase, press) in items {
                    let b = small_button(&mut commands, &fonts, phrase.text(lang), press);
                    commands.entity(menu).add_child(b);
                }
                kids.push(menu);
                // Its members.
                let names: Vec<String> =
                    group
                        .members
                        .iter()
                        .map(|m| {
                            let mut name = machine.shown_name(&m.person);
                            if m.owner {
                                name.push_str(&format!(" · {}", ChatPhrase::OwnerMark.text(lang)));
                            }
                            name
                        })
                        .chain(group.invited.iter().map(|p| {
                            format!("{} {}", p.handle, ChatPhrase::InvitedMark.text(lang))
                        }))
                        .collect();
                let members = label(
                    &mut commands,
                    &fonts,
                    names.join(", "),
                    tokens::MUTED,
                    false,
                );
                kids.push(members);
            }
        }
    }
    friend_rows(
        &mut commands,
        &mut kids,
        &view.offline,
        ChatPhrase::SectionOffline,
    );
    // Recent players.
    let recent: Vec<_> = machine
        .recent_players()
        .iter()
        .filter(|r| !machine.contacts().is_friend(&r.tag) && r.tag != me.tag)
        .take(8)
        .collect();
    if !me.guest && !recent.is_empty() {
        kids.push(heading(
            &mut commands,
            &fonts,
            ChatPhrase::RecentPlayers.text(lang).to_string(),
        ));
        for player in recent {
            let line = row(&mut commands);
            let name = label(
                &mut commands,
                &fonts,
                player.handle.clone(),
                tokens::INK,
                true,
            );
            let add = small_button(
                &mut commands,
                &fonts,
                ChatPhrase::AddFriendButton.text(lang),
                DrawerPress::AddRecent(player.handle.clone()),
            );
            commands.entity(line).add_children(&[name, add]);
            kids.push(line);
        }
    }
    // Blocked.
    if !view.blocked.is_empty() {
        kids.push(heading(
            &mut commands,
            &fonts,
            ChatPhrase::SectionBlocked.text(lang).to_string(),
        ));
        for person in view.blocked {
            let line = row(&mut commands);
            let name = label(
                &mut commands,
                &fonts,
                person.handle.clone(),
                tokens::MUTED,
                true,
            );
            let undo = small_button(
                &mut commands,
                &fonts,
                ChatPhrase::UnblockButton.text(lang),
                DrawerPress::Unblock(person.tag.clone()),
            );
            commands.entity(line).add_children(&[name, undo]);
            kids.push(line);
        }
    }
    commands.entity(list).add_children(&kids);
    // The foot.
    let status = match machine.status() {
        Status::Online => ChatPhrase::StatusOnline,
        Status::Away => ChatPhrase::StatusAway,
        Status::Dnd => ChatPhrase::StatusDnd,
        Status::Invisible => ChatPhrase::StatusInvisible,
    };
    let foot = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                flex_wrap: FlexWrap::Wrap,
                column_gap: px(6),
                row_gap: px(6),
                padding: UiRect::axes(px(14), px(10)),
                border: UiRect::top(px(1)),
                flex_shrink: 0.0,
                ..default()
            },
            BorderColor::all(tokens::BORDER),
        ))
        .id();
    if !me.guest {
        let new = small_button(
            &mut commands,
            &fonts,
            ChatPhrase::NewGroupButton.text(lang),
            DrawerPress::NewGroup,
        );
        commands.entity(foot).add_child(new);
    }
    let status = small_button(
        &mut commands,
        &fonts,
        &ChatPhrase::StatusLabel.fill(lang, &[status.text(lang)]),
        DrawerPress::Status,
    );
    let privacy = small_button(
        &mut commands,
        &fonts,
        ChatPhrase::PrivacyButton.text(lang),
        DrawerPress::Privacy,
    );
    commands.entity(foot).add_children(&[status, privacy]);
    commands.entity(root).add_children(&[head, list, foot]);
}

/// The drawer's scrolling list, whose scroll survives its rebuilds.
#[derive(Component)]
pub(crate) struct DrawerList;
