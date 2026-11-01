//! The friends drawer (the beta.8 social design §11.3): requests, table and
//! group invites, friends by presence (where they are: playing, waiting, at
//! a table), groups, recent players, blocked players, adding by name, the
//! status menu and the door to the chat's privacy settings.
//!
//! One overlay for the lobby and the table, sliding in from the right
//! (`Z_MENU`'s place on the global ladder); `f` is unbound at the table
//! (`Action::Friends`), `/friends` and the chat's head open it too.

use super::zoom::{z, zpx};
use crate::ui_button::{Clicks, UiButton};
use baylee_client_core::i18n::Lang;
use baylee_client_core::social::{ChatPhrase, ChatTab, Intent, Social};
use baylee_protocol::social::{Place, Shown, Status};
use bevy::prelude::*;
use bevy::ui::{RelativeCursorPosition, ScrollPosition};

use super::window::ChatSurface;
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

/// The drawer's scrollbar track, shown only when `DrawerList` overflows.
#[derive(Component)]
struct DrawerScrollbar(Entity);

/// Installs the drawer.
pub(crate) fn install(app: &mut App) {
    crate::ui_button::install(app);
    app.init_resource::<DrawerSlide>()
        .add_systems(
            Update,
            (
                drawer_follows_the_room,
                asks,
                presses,
                sync_door,
                sync_drawer,
            )
                .chain()
                .run_if(resource_exists::<UiFonts>),
        )
        // After every rebuild of the frame and before the layout: a shell the
        // lobby stood up again this frame is squeezed before it is ever laid
        // out at full width under the drawer (measured: one frame of it on
        // every lobby press, paused-clock diff, 13.10.).
        .add_systems(
            PostUpdate,
            (slide_the_drawer, clear_the_lane)
                .chain()
                .before(bevy::ui::UiSystems::Prepare)
                .run_if(resource_exists::<UiFonts>),
        )
        .add_systems(
            PostUpdate,
            show_drawer_scrollbar
                .after(bevy::ui::UiSystems::Layout)
                .run_if(resource_exists::<UiFonts>),
        );
}

/// Shows the drawer's scrollbar track only when its list overflows vertically.
fn show_drawer_scrollbar(
    mut bars: Query<(&DrawerScrollbar, &mut Node)>,
    lists: Query<&ComputedNode>,
) {
    for (bar, mut node) in &mut bars {
        let Ok(list) = lists.get(bar.0) else {
            continue;
        };
        let needed = list.content_size.y > list.size.y + 1.0;
        let want = if needed { Display::Flex } else { Display::None };
        if node.display != want {
            node.display = want;
        }
    }
}

/// How far the drawer is in: 0 shut, 1 open (eased when drawn).
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq)]
pub(crate) struct DrawerSlide(pub f32);

impl DrawerSlide {
    /// The room it takes from the right edge now, in UI pixels at `zoom`.
    pub(crate) fn room(self, zoom: f32) -> f32 {
        let t = self.0.clamp(0.0, 1.0);
        let eased = 1.0 - (1.0 - t).powi(3);
        eased * WIDTH * zoom
    }
}

/// A node of the lobby's that gave the drawer its room, and the width it
/// had before.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct Squeezed(pub Val);

/// The friends list, open by default in the lobby, steps aside as soon as a
/// game starts (before the loading screen `Screen::Seated` / `DuelPhase::Opening`)
/// and comes back with the lobby; if the player expands it during the loading
/// screen, it stays open while loading and steps aside again when the 3D table
/// (`DuelPhase::Playing`) opens.
fn drawer_follows_the_room(
    phase: Option<Res<State<crate::DuelPhase>>>,
    lobby: Option<Res<crate::lobby::LobbyState>>,
    mut ui: ResMut<ChatUi>,
    mut was_starting: Local<Option<bool>>,
    mut was_table: Local<Option<bool>>,
    mut reopen: Local<bool>,
) {
    let seated = lobby.is_some_and(|l| {
        matches!(
            l.lobby.screen(),
            baylee_client_core::lobby::Screen::Seated(_)
        )
    });
    let phase_now = phase.map_or(crate::DuelPhase::Closed, |p| *p.get());
    let starting_or_playing = seated || phase_now != crate::DuelPhase::Closed;
    let on_table = matches!(
        phase_now,
        crate::DuelPhase::Playing | crate::DuelPhase::Finished
    );
    if *was_starting != Some(starting_or_playing) {
        match (*was_starting, starting_or_playing) {
            (Some(false), true) => {
                *reopen = ui.drawer;
                if ui.drawer {
                    ui.drawer = false;
                }
            }
            (Some(true), false) => {
                if *reopen && !ui.drawer {
                    ui.drawer = true;
                }
                *reopen = false;
            }
            _ => {}
        }
        *was_starting = Some(starting_or_playing);
    }
    if *was_table != Some(on_table) {
        if *was_table == Some(false) && on_table && ui.drawer {
            ui.drawer = false;
        }
        *was_table = Some(on_table);
    }
}

/// Where the shell's notice lane stood before the chat moved it.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct LaneBase {
    right: Val,
    bottom: Val,
}

/// Keeps the shell's notices ("Bob sat down at your table") in the content
/// area: left of the friends list and above the chat's pill or window
/// (RC: they were clipped by the list). A phone's lane hangs from the top
/// and is left alone; without a session, or at a table, it stands where the
/// shell put it.
#[allow(clippy::too_many_arguments, clippy::type_complexity)] // a Bevy system: the lane, its node and where it stood
fn clear_the_lane(
    mut commands: Commands,
    host: Res<SocialHost>,
    ui: Res<ChatUi>,
    slide: Res<DrawerSlide>,
    zoom: Res<super::zoom::Zoom>,
    (windows, scale): (Query<&Window>, Option<Res<UiScale>>),
    phase: Option<Res<State<crate::DuelPhase>>>,
    mut lanes: Query<(Entity, &mut Node, Option<&LaneBase>), With<crate::lobby::ToastLane>>,
) {
    let at_table = phase.is_some_and(|p| *p.get() != crate::DuelPhase::Closed);
    let window_h = windows.iter().next().map_or(720.0, |w| {
        crate::hud::scale::desktop_space(w, scale.as_deref()).y
    });
    for (lane, mut node, base) in &mut lanes {
        let base = base.copied().unwrap_or_else(|| {
            let base = LaneBase {
                right: node.right,
                bottom: node.bottom,
            };
            commands.entity(lane).insert(base);
            base
        });
        let (Val::Px(right), Val::Px(bottom)) = (base.right, base.bottom) else {
            continue;
        };
        let (right, bottom) = if host.active() && !at_table {
            let chat = if ui.open && !ui.minimised {
                super::window::lobby_panel_height(window_h) + 16.0
            } else {
                64.0 * zoom.0
            };
            (right + slide.room(zoom.0), bottom.max(chat))
        } else {
            (right, bottom)
        };
        if node.right != Val::Px(right) {
            node.right = Val::Px(right);
        }
        if node.bottom != Val::Px(bottom) {
            node.bottom = Val::Px(bottom);
        }
    }
}

/// How long the drawer takes to slide in or out.
const SLIDE_SECS: f32 = 0.18;

/// The drawer slides in from the right edge under the header and the
/// lobby's content gives it the room, reflowing as it comes (owner,
/// 13.10.); out again, the content takes the room back. At the table (or
/// over the loading screen) it slides over the screen, which does not
/// reflow. `reduce_motion` cuts it.
#[allow(
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::too_many_lines
)] // a Bevy system: the drawer and what makes room for it
fn slide_the_drawer(
    time: Res<Time>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    ui: Res<ChatUi>,
    host: Res<SocialHost>,
    phase: Option<Res<State<crate::DuelPhase>>>,
    lobby: Option<Res<crate::lobby::LobbyState>>,
    zoom: Res<super::zoom::Zoom>,
    mut slide: ResMut<DrawerSlide>,
    mut drawers: Query<(&mut Node, &mut Visibility), With<Drawer>>,
    headers: Query<(
        Entity,
        &crate::shellkit::role::Role,
        &ComputedNode,
        &ChildOf,
    )>,
    kids: Query<&Children>,
    mut content: Query<(&mut Node, Option<&Squeezed>), Without<Drawer>>,
    mut commands: Commands,
    mut wide: Local<f32>,
) {
    let seated = lobby.is_some_and(|l| {
        matches!(
            l.lobby.screen(),
            baylee_client_core::lobby::Screen::Seated(_)
        )
    });
    let in_a_duel = phase.is_some_and(|p| *p.get() != crate::DuelPhase::Closed);
    let at_table = in_a_duel || seated;
    // At a table it ends over the hand zone, whose top is the bar with the
    // HUD's own doors (chat, log, zones, menu): run to the window's foot, it
    // covered them and a press on one answered nothing (#389).
    let foot = if in_a_duel {
        Val::Px(crate::hud::HAND_ZONE_H)
    } else {
        zpx(0.0)
    };
    let open = ui.drawer && host.active();
    let target = if open { 1.0 } else { 0.0 };
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    let t = if still {
        target
    } else {
        let step = time.delta_secs() / SLIDE_SECS;
        if slide.0 < target {
            (slide.0 + step).min(target)
        } else {
            (slide.0 - step).max(target)
        }
    };
    if (slide.0 - t).abs() > f32::EPSILON {
        slide.0 = t;
    }
    let room = slide.room(zoom.0);
    let width = WIDTH * zoom.0;
    let header = if at_table {
        0.0
    } else {
        // A header the lobby stood up this frame has no size yet: the
        // drawer keeps the top it had rather than jumping over it for one
        // frame (measured, paused clock).
        headers
            .iter()
            .find(|(_, role, ..)| **role == crate::shellkit::role::Role::Header)
            .map(|(_, _, node, _)| node.size().y * node.inverse_scale_factor())
            .filter(|h| *h > 0.0)
            .unwrap_or_else(|| {
                drawers
                    .iter()
                    .find_map(|(node, _)| match node.top {
                        Val::Px(top) => Some(top),
                        _ => None,
                    })
                    .unwrap_or(0.0)
            })
    };
    for (mut node, mut seen) in &mut drawers {
        let right = Val::Px(room - width);
        if node.right != right {
            node.right = right;
        }
        if node.top != Val::Px(header) {
            node.top = Val::Px(header);
        }
        if node.bottom != foot {
            node.bottom = foot;
        }
        seen.set_if_neq(if slide.0 > 0.0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        });
    }
    // The lobby's content beside the header (the header keeps the whole
    // width) gives the drawer its room. A sibling written at a percentage of
    // the width gets that share of what is left, in pixels (a width let go
    // to `Auto` shrank a centred room to its content, and its Ready switch
    // lay over the deck); any other keeps its width and takes a margin.
    // Each gets its own back when the drawer is in again.
    let margin = if at_table { 0.0 } else { room };
    for (header, role, computed, parent) in &headers {
        if *role != crate::shellkit::role::Role::Header {
            continue;
        }
        let measured = computed.size().x * computed.inverse_scale_factor();
        if measured > 0.0 {
            *wide = measured;
        }
        let Ok(siblings) = kids.get(parent.parent()) else {
            continue;
        };
        for kid in siblings {
            if *kid == header {
                continue;
            }
            let Ok((mut node, squeezed)) = content.get_mut(*kid) else {
                continue;
            };
            if node.position_type == PositionType::Absolute {
                continue;
            }
            if margin > 0.0 {
                let base = squeezed.map_or(node.width, |s| s.0);
                if squeezed.is_none() {
                    commands.entity(*kid).insert(Squeezed(node.width));
                }
                let (width, right) = match base {
                    // The margin too: a parent that centres its children
                    // would otherwise stand the narrower node in the middle,
                    // half of it under the drawer.
                    Val::Percent(share) if *wide > 0.0 => {
                        (Val::Px(*wide * share / 100.0 - margin), Val::Px(margin))
                    }
                    _ => (Val::Auto, Val::Px(margin)),
                };
                if node.width != width {
                    node.width = width;
                }
                if node.margin.right != right {
                    node.margin.right = right;
                }
            } else if let Some(Squeezed(width)) = squeezed {
                node.width = *width;
                node.margin.right = Val::ZERO;
                commands.entity(*kid).remove::<Squeezed>();
            }
        }
    }
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
                    ui.drawer = false;
                    ui.typing = false;
                }
            }
            UiAsk::Watch { game } => {
                if let Some(lobby) = lobby.as_deref_mut()
                    && !at_table
                {
                    lobby.lobby.watch_table(&game);
                    ui.drawer = false;
                    ui.typing = false;
                }
            }
        }
    }
}

/// The drawer's presses.
#[allow(clippy::too_many_lines)] // one arm per control
fn presses(
    mut clicks: Clicks<DrawerPress>,
    mut host: ResMut<SocialHost>,
    mut ui: ResMut<ChatUi>,
    mut more: Local<Option<String>>,
) {
    let clicked: Vec<DrawerPress> = clicks.read();
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
                ui.typing = false;
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
                ui.drawer_menu.clone_from(&more);
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
                ui.drawer_menu.clone_from(&more);
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

/// The friends button in the bottom-right corner on screens without a top
/// header (`Screen::Build`, `Screen::Seated`, `DuelPhase::Opening`): online
/// friends and a dot for anything waiting (requests, invites).
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // a Bevy system: what it reads
fn sync_door(
    mut commands: Commands,
    host: Res<SocialHost>,
    ui: Res<ChatUi>,
    fonts: Res<UiFonts>,
    phase: Option<Res<State<crate::DuelPhase>>>,
    lobby: Option<Res<crate::lobby::LobbyState>>,
    doors: Query<(Entity, &mut Visibility), With<FriendsDoor>>,
    mut drawn: Local<Option<(usize, bool, bool)>>,
    zoom: Res<super::zoom::Zoom>,
) {
    let _zoom = super::zoom::Scope::new(zoom.0);
    let on_table = phase.as_ref().is_some_and(|p| {
        matches!(
            *p.get(),
            crate::DuelPhase::Playing | crate::DuelPhase::Finished
        )
    });
    let loading = phase
        .as_ref()
        .is_some_and(|p| *p.get() == crate::DuelPhase::Opening)
        || lobby.as_ref().is_some_and(|l| {
            matches!(
                l.lobby.screen(),
                baylee_client_core::lobby::Screen::Seated(_)
            )
        });
    // The header has the friends button; the deck builder and the loading
    // screen do not, so there the button floats bottom-right.
    let shell = lobby.as_ref().is_some_and(|l| {
        matches!(
            l.lobby.screen(),
            baylee_client_core::lobby::Screen::Build | baylee_client_core::lobby::Screen::Seated(_)
        )
    }) || loading;
    // The panel's head has the friends button while the panel stands.
    let panel = ui.open && !ui.minimised;
    let machine = host
        .machine()
        .filter(|m| shell && !on_table && !panel && !m.me().guest);
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
                UiButton,
                DrawerPress::Toggle,
                Node {
                    position_type: PositionType::Absolute,
                    bottom: zpx(12.0),
                    right: zpx(12.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: zpx(8.0),
                    padding: UiRect::axes(zpx(14.0), zpx(8.0)),
                    border: UiRect::all(zpx(1.0)),
                    border_radius: BorderRadius::all(zpx(18.0)),
                    ..default()
                },
                BackgroundColor(super::rows::LOBBY_GROUND),
                BorderColor::all(tokens::BORDER),
                Feel::new(super::rows::LOBBY_GROUND),
                super::window::stacked(1),
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
            icon_tf(&fonts, z(13.0)),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    let count = commands
        .spawn((
            Text::new(online.to_string()),
            tf_bold(&fonts, z(13.0)),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(door).add_children(&[icon, count]);
    if waiting {
        let dot = commands
            .spawn((
                Node {
                    width: zpx(8.0),
                    height: zpx(8.0),
                    border_radius: BorderRadius::all(zpx(4.0)),
                    ..default()
                },
                BackgroundColor(super::rows::WHISPER_INK),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(door).add_child(dot);
    }
}

/// A contact's circular avatar disc (`28×28`) with their initial and a
/// presence-coloured status ring + indicator dot.
fn person_avatar(commands: &mut Commands, fonts: &UiFonts, name: &str, shown: Shown) -> Entity {
    let (ring, dot_fill) = match shown {
        Shown::Online => (
            Color::srgb(0.420, 0.816, 0.541),
            Color::srgb(0.420, 0.816, 0.541),
        ),
        Shown::Away => (palette::CANDLE, palette::CANDLE),
        Shown::Dnd => (palette::DANGER, palette::DANGER),
        Shown::Offline => (tokens::BORDER, tokens::MUTED),
    };
    let initial = name
        .chars()
        .find(|c| c.is_alphanumeric())
        .map_or_else(|| "?".to_string(), |c| c.to_uppercase().to_string());
    let disc_bg = if shown == Shown::Offline {
        Color::srgba(0.08, 0.12, 0.16, 0.75)
    } else {
        Color::srgba(0.11, 0.19, 0.25, 0.95)
    };
    commands
        .spawn((
            Node {
                width: zpx(28.0),
                height: zpx(28.0),
                border: UiRect::all(zpx(1.5)),
                border_radius: BorderRadius::all(zpx(14.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(disc_bg),
            BorderColor::all(ring),
            Pickable::IGNORE,
            children![
                (
                    Text::new(initial),
                    tf_bold(fonts, z(12.0)),
                    TextColor(if shown == Shown::Offline {
                        tokens::MUTED
                    } else {
                        tokens::INK
                    }),
                    Pickable::IGNORE,
                ),
                (
                    Node {
                        position_type: PositionType::Absolute,
                        right: zpx(-2.0),
                        bottom: zpx(-2.0),
                        width: zpx(9.0),
                        height: zpx(9.0),
                        border: UiRect::all(zpx(1.5)),
                        border_radius: BorderRadius::all(zpx(5.0)),
                        ..default()
                    },
                    BackgroundColor(dot_fill),
                    BorderColor::all(super::rows::LOBBY_GROUND),
                    Pickable::IGNORE,
                ),
            ],
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
            UiButton,
            press,
            Node {
                padding: UiRect::axes(zpx(7.0), zpx(2.0)),
                border: UiRect::all(zpx(1.0)),
                border_radius: BorderRadius::all(zpx(4.0)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(Color::NONE),
            BorderColor::all(tokens::BORDER),
            Feel::new(Color::NONE),
            children![(
                Text::new(label.to_string()),
                tf(fonts, z(11.0)),
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
            UiButton,
            press,
            Node {
                width: zpx(24.0),
                height: zpx(24.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(zpx(5.0)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(Color::NONE),
            Feel::new(Color::NONE),
            children![(
                Text::new(glyph.to_string()),
                icon_tf(fonts, z(11.0)),
                TextColor(tokens::MUTED),
                Pickable::IGNORE,
            )],
        ))
        .id()
}

/// Larger close button (`30×30`) for the drawer's top-right header.
fn close_button(commands: &mut Commands, fonts: &UiFonts) -> Entity {
    commands
        .spawn((
            UiButton,
            DrawerPress::Close,
            Node {
                width: zpx(30.0),
                height: zpx(30.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(zpx(1.0)),
                border_radius: BorderRadius::all(zpx(6.0)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.14, 0.19, 0.90)),
            BorderColor::all(tokens::BORDER),
            Feel::new(Color::srgba(0.08, 0.14, 0.19, 0.90)),
            children![(
                Text::new(glyph::CLOSE.to_string()),
                icon_tf(fonts, z(13.5)),
                TextColor(tokens::INK),
                Pickable::IGNORE,
            )],
        ))
        .id()
}

fn heading(commands: &mut Commands, fonts: &UiFonts, text: String) -> Entity {
    commands
        .spawn((
            Text::new(text.to_uppercase()),
            tf_bold(fonts, z(10.5)),
            TextColor(tokens::MUTED),
            Node {
                margin: UiRect::new(zpx(0.0), zpx(0.0), zpx(10.0), zpx(4.0)),
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
                column_gap: zpx(6.0),
                padding: UiRect::vertical(zpx(3.0)),
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
            tf(fonts, z(12.0)),
            TextColor(colour),
            Node {
                flex_grow: if grow { 1.0 } else { 0.0 },
                min_width: zpx(0.0),
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
            UiButton,
            press,
            Text::default(),
            tf(fonts, z(12.0)),
            TextColor(tokens::INK),
            Node {
                flex_grow: 1.0,
                padding: UiRect::axes(zpx(8.0), zpx(5.0)),
                border: UiRect::all(zpx(1.0)),
                border_radius: BorderRadius::all(zpx(5.0)),
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
            ("|".to_string(), tokens::ACCENT, false),
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
            tf_italic(fonts, z(12.0))
        } else {
            tf(fonts, z(12.0))
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
    mut drawn: Local<Option<DrawerKey>>,
    zoom: Res<super::zoom::Zoom>,
) {
    let _zoom = super::zoom::Scope::new(zoom.0);
    let mut roots = roots;
    // Drawn while the chat is up, open or not: it slides in and out
    // (`slide_the_drawer`) with its rows standing.
    let Some(machine) = host.machine() else {
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
    let root = if let Some((root, _)) = roots.iter_mut().next() {
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
                    top: zpx(0.0),
                    bottom: zpx(0.0),
                    right: zpx(-WIDTH),
                    width: zpx(WIDTH),
                    flex_direction: FlexDirection::Column,
                    border: UiRect::left(zpx(1.0)),
                    ..default()
                },
                // A hair translucent over the painting (owner, 13.10.).
                BackgroundColor(super::rows::LOBBY_GROUND.with_alpha(0.92)),
                BorderColor::all(tokens::BORDER),
                super::window::stacked(10),
                Visibility::Hidden,
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
                padding: UiRect::axes(zpx(14.0), zpx(10.0)),
                border: UiRect::bottom(zpx(1.0)),
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
            tf_bold(&fonts, z(14.0)),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    let close = close_button(&mut commands, &fonts);
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
                min_height: zpx(0.0),
                padding: UiRect::axes(zpx(14.0), zpx(8.0)),
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
            let avatar = person_avatar(&mut commands, &fonts, &person.handle, Shown::Online);
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
            commands.entity(line).add_children(&[avatar, name, yes, no]);
            kids.push(line);
        }
        for person in view.outgoing {
            let line = row(&mut commands);
            let avatar = person_avatar(&mut commands, &fonts, &person.handle, Shown::Offline);
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
            commands.entity(line).add_children(&[avatar, name, undo]);
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
            let card_bg = Color::srgba(0.06, 0.11, 0.15, 0.85);
            // The row lights under the pointer (its buttons are its own).
            commands.entity(line).insert((
                UiButton,
                Feel::new(card_bg),
                BackgroundColor(card_bg),
                BorderColor::all(tokens::BORDER.with_alpha(0.65)),
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: zpx(8.0),
                    padding: UiRect::axes(zpx(8.0), zpx(6.0)),
                    margin: UiRect::vertical(zpx(2.0)),
                    border: UiRect::all(zpx(1.0)),
                    border_radius: BorderRadius::all(zpx(8.0)),
                    flex_wrap: FlexWrap::Wrap,
                    ..default()
                },
            ));
            let name = machine.shown_name(&friend.person);
            let shown_name = if name == friend.person.handle {
                name
            } else {
                format!("{name} ({})", friend.person.handle)
            };
            let presence = person_avatar(commands, &fonts, &shown_name, friend.presence.status);
            let words = commands
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        flex_grow: 1.0,
                        min_width: zpx(0.0),
                        row_gap: zpx(1.5),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .id();
            let top = commands
                .spawn((
                    Text::new(shown_name),
                    tf_bold(&fonts, z(12.0)),
                    TextColor(tokens::INK),
                    Pickable::IGNORE,
                ))
                .id();
            let place = commands
                .spawn((
                    Text::new(place_words(
                        friend.presence.status,
                        friend.presence.place.as_ref(),
                        lang,
                    )),
                    tf(&fonts, z(10.5)),
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
    // Online, by where they are (owner, 13.10.): playing, at a table,
    // in the lobby (and anywhere else).
    let place_of = |f: &&baylee_protocol::social::Friend| match f.presence.place {
        Some(Place::Playing { .. } | Place::Watching { .. }) => 0,
        Some(Place::Room { .. }) => 1,
        _ => 2,
    };
    for (n, title) in [
        (0, ChatPhrase::SectionPlaying),
        (1, ChatPhrase::SectionAtTable),
        (2, ChatPhrase::SectionInLobby),
    ] {
        let part: Vec<&baylee_protocol::social::Friend> = view
            .online
            .iter()
            .copied()
            .filter(|f| place_of(f) == n)
            .collect();
        friend_rows(&mut commands, &mut kids, &part, title);
    }
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
                    UiButton,
                    DrawerPress::OpenGroup(group.id.clone()),
                    Node {
                        flex_grow: 1.0,
                        padding: UiRect::vertical(zpx(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                    Feel::new(Color::NONE),
                    children![(
                        Text::new(format!("# {}  · {}", group.name, group.members.len())),
                        tf(&fonts, z(12.0)),
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
                                name.push_str(" · ");
                                name.push_str(ChatPhrase::OwnerMark.text(lang));
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
            let avatar = person_avatar(&mut commands, &fonts, &player.handle, Shown::Offline);
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
            commands.entity(line).add_children(&[avatar, name, add]);
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
    // A scrollbar beside the list, visible only when content overflows.
    let track = crate::hud::logpanel::scrollbar(
        &mut commands,
        list,
        (
            tokens::BORDER.with_alpha(0.35),
            tokens::ACCENT.with_alpha(0.8),
        ),
        crate::hud::logpanel::at_rest,
    );
    commands.entity(track).insert((
        DrawerScrollbar(list),
        crate::lobby::scrollbars::OnlyWhenNeeded,
    ));
    let middle = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                flex_grow: 1.0,
                min_height: zpx(0.0),
                column_gap: zpx(4.0),
                padding: UiRect::right(zpx(4.0)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .add_children(&[list, track])
        .id();
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
                column_gap: zpx(6.0),
                row_gap: zpx(6.0),
                padding: UiRect::axes(zpx(14.0), zpx(10.0)),
                border: UiRect::top(zpx(1.0)),
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
    commands.entity(root).add_children(&[head, middle, foot]);
}

/// What the drawer was last drawn from: the machine's revision, the search
/// box, the entry box, the keys, the open menu, the field.
type DrawerKey = (u64, String, String, bool, Option<String>, String);

/// The drawer's scrolling list, whose scroll survives its rebuilds.
#[derive(Component)]
pub(crate) struct DrawerList;

#[cfg(test)]
mod tests;
