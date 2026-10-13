//! The one chat window (the chat window design §1–§3, §8): at the table in
//! the log's panel (its frame, size and surface; the log one of its tabs),
//! in the lobby docked bottom-right on the kit's panel. Inside either frame
//! the tab row, the Group tab's bar, the list and the composer are the same
//! code.
//!
//! Everything is retained and redrawn only when what it shows changed: the
//! tab row on its counts, the list on the lines it admits (and, on the Log
//! tab, only the lines that arrived), the composer on its text. At rest
//! nothing here writes.

use baylee_client_core::i18n::Lang;
use baylee_client_core::social::{
    ChatPhrase, ChatTab, ComposerKey, ConvId, GroupRef, Intent, Social, TabFilter, Target,
};
use baylee_protocol::social::Conv;
use bevy::prelude::*;
use bevy::ui::{RelativeCursorPosition, ScrollPosition};

use super::rows::{self, Inks, LOBBY_INKS, RowStyle, TABLE_INKS};
use super::{ChatUi, Field, SocialHost};
use crate::ambience::Feel;
use crate::hud::logpanel::{self, LogFollow, LogList, LogPanel, PhaseRun};
use crate::hud::{UiFonts, glyph, icon_tf, palette, tf, tf_bold, tf_italic};
use crate::{ChatDoor, Duel, DuelPhase};

/// The tab row's height at the table (the bar's button height).
const ROW_H: f32 = 28.0;
/// And in the lobby.
const LOBBY_ROW_H: f32 = 34.0;
/// The Group tab's bar.
const SUB_H: f32 = 24.0;
/// A head button.
const HEAD_BTN: f32 = 22.0;
/// The lobby panel's width.
const LOBBY_W: f32 = 560.0;
/// The room the chat pill takes at the bottom-right, which the friends
/// button stands left of.
pub(crate) const LOBBY_PILL_ROOM: f32 = 120.0;
/// How many chat lines a tab draws (older ones stay in the machine).
const CHAT_ROWS: usize = 200;
/// How many log lines the All tab merges in.
const ALL_LOG_ROWS: usize = 150;
/// The lobby panel's place on the global ladder: over the lobby, under the
/// report form (1000).
pub(crate) const G_PANEL: i32 = 900;

/// A press on one of the window's controls.
#[derive(Component, Clone, Debug, PartialEq)]
pub(crate) enum ChatPress {
    /// A tab.
    Tab(ChatTab),
    /// The head's gear: this tab's filters.
    Gear,
    /// The head's lock: the encryption sheet.
    Lock,
    /// The head's cross.
    Close,
    /// The lobby's minimise.
    Minimise,
    /// The lobby's pill, or its minimised row: open.
    Restore,
    /// The composer's chip: the next target.
    Chip,
    /// The composer's box: take the keys.
    Field,
    /// The send button.
    Send,
    /// A suggestion's row.
    Suggestion(usize),
    /// The Group tab's name: its dropdown.
    GroupMenu,
    /// A group in the dropdown.
    ShowGroup(String),
    /// The dropdown's "New group…".
    NewGroup,
    /// A filter box.
    Filter(FilterKind),
    /// The filter dialog's Reset.
    FilterReset,
    /// The filter dialog's Done.
    FilterDone,
    /// The head's friends button: the drawer.
    Friends,
}

/// One box of the filter dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FilterKind {
    Log,
    Table,
    Team,
    Whispers,
    Groups,
    Notices,
    Presence,
    Timestamps,
}

/// A part of a drawn window, by role.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Part {
    Head,
    Sub,
    List,
    Composer,
}

/// The lobby's panel root.
#[derive(Component)]
pub(crate) struct LobbyChat;

/// Marks a node of the chat's own (the panel, the drawer, a sheet): a
/// press anywhere else takes the keys back from the composer.
#[derive(Component)]
pub(crate) struct ChatSurface;

/// What one frame of the window last drew, part by part.
#[derive(Default, Debug)]
pub(crate) struct Drawn {
    /// The root it drew into.
    root: Option<Entity>,
    head: Option<Entity>,
    sub: Option<Entity>,
    list: Option<Entity>,
    composer: Option<Entity>,
    head_key: u64,
    sub_key: u64,
    list_key: u64,
    composer_key: u64,
    /// On the Log tab: how many of the book's lines are drawn, and where
    /// they left the phase headings.
    log_drawn: usize,
    run: PhaseRun,
}

/// The table's and the lobby's drawn windows.
#[derive(Resource, Default)]
pub(crate) struct Windows {
    table: Drawn,
    lobby: Drawn,
}

/// Which frame a window stands in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Frame {
    Table,
    Lobby,
}

impl Frame {
    fn inks(self) -> Inks {
        match self {
            Self::Table => TABLE_INKS,
            Self::Lobby => LOBBY_INKS,
        }
    }

    fn row_h(self) -> f32 {
        match self {
            Self::Table => ROW_H,
            Self::Lobby => LOBBY_ROW_H,
        }
    }
}

/// Installs the window's systems.
pub(crate) fn install(app: &mut App) {
    app.init_resource::<Windows>().add_systems(
        Update,
        (
            doors,
            keep_the_tab,
            stamp_arrivals,
            mark_read,
            presses,
            sync_table_window
                .after(crate::hud::sync_log)
                .before(crate::hud::follow_the_log),
            sync_lobby_window,
            crate::hud::follow_the_log.run_if(in_state(DuelPhase::Closed)),
            hover_chat_links,
            peek,
            filter_dialog,
        )
            .chain()
            .run_if(resource_exists::<UiFonts>),
    );
}

/// The tabs drawn, in order (§3.1): All; Log while a game is open; Table
/// and Team while seated; Private; Group for a registered account.
pub(crate) fn drawn_tabs(machine: &Social, game_open: bool) -> Vec<ChatTab> {
    let table = machine.table();
    let seated = table.is_some_and(|t| t.table_conv().is_some());
    let team = table.is_some_and(|t| t.team_conv().is_some());
    ChatTab::ALL
        .into_iter()
        .filter(|tab| match tab {
            ChatTab::All | ChatTab::Private => true,
            ChatTab::Log => game_open,
            ChatTab::Table => seated,
            ChatTab::Team => team,
            ChatTab::Group => !machine.me().guest,
        })
        .collect()
}

/// What a tab is called.
pub(crate) fn tab_name(tab: ChatTab, lang: Lang) -> &'static str {
    match tab {
        ChatTab::All => ChatPhrase::TabAll.text(lang),
        ChatTab::Log => ChatPhrase::TabLog.text(lang),
        ChatTab::Table => ChatPhrase::TabTable.text(lang),
        ChatTab::Team => ChatPhrase::TabTeam.text(lang),
        ChatTab::Private => ChatPhrase::TabPrivate.text(lang),
        ChatTab::Group => ChatPhrase::TabGroup.text(lang),
    }
}

/// The conversation a chat tab writes to and reads (§3.1's "Composer
/// target"): the table's, the team's, the last whisper partner's, the
/// shown group's.
pub(crate) fn tab_conv(machine: &Social, ui: &ChatUi, tab: ChatTab) -> Option<ConvId> {
    match tab {
        ChatTab::Table => machine.table().and_then(|t| t.table_conv()),
        ChatTab::Team => machine.table().and_then(|t| t.team_conv()),
        ChatTab::Private => {
            let partner = machine
                .last_whisper_partner()
                .map(str::to_string)
                .or_else(|| {
                    machine
                        .conversations()
                        .iter()
                        .filter(|c| matches!(c.id, Conv::Dm { .. }))
                        .max_by_key(|c| c.lines.last().map_or(0, |l| l.at))
                        .and_then(|c| c.id.partner(&machine.me().tag).map(str::to_string))
                })?;
            Conv::dm(&machine.me().tag, &partner)
        }
        ChatTab::Group => shown_group(machine, ui).map(|id| Conv::Group { id }),
        ChatTab::All | ChatTab::Log => None,
    }
}

/// The group the Group tab shows: the picked one while it exists, else the
/// first.
pub(crate) fn shown_group(machine: &Social, ui: &ChatUi) -> Option<String> {
    let groups = &machine.contacts().groups;
    ui.shown_group
        .as_ref()
        .filter(|id| groups.iter().any(|g| &g.id == *id))
        .cloned()
        .or_else(|| groups.first().map(|g| g.id.clone()))
}

/// What a tab shows: the stored filter, with the Group tab narrowed to the
/// shown group.
pub(crate) fn tab_filter(machine: &Social, ui: &ChatUi, tab: ChatTab) -> TabFilter {
    let mut filter = machine.prefs().filter(tab);
    if tab == ChatTab::Group && filter.groups != baylee_client_core::social::GroupPick::None {
        filter.groups = shown_group(machine, ui)
            .map_or(baylee_client_core::social::GroupPick::None, |id| {
                baylee_client_core::social::GroupPick::Only(vec![id])
            });
    }
    filter
}

/// Shows `tab`, telling the machine which conversation is read and written
/// there (the composer's target follows).
pub(crate) fn select_tab(host: &mut SocialHost, ui: &mut ChatUi, tab: ChatTab) {
    ui.tab = tab;
    ui.group_menu = false;
    ui.follow = true;
    if tab.is_chat() {
        ui.last_chat_tab = Some(tab);
    }
    let conv = host.machine().and_then(|m| tab_conv(m, ui, tab));
    if let Some(conv) = conv {
        host.act(Intent::Panel(Some(conv)));
    }
}

/// Opens the window on the last chat tab used (never Log): the chat door,
/// the lobby's pill, `/`, `R`, `Ctrl`/`Cmd`+`Enter` (§3.1).
pub(crate) fn open_on_chat(host: &mut SocialHost, ui: &mut ChatUi, duel: Option<&mut Duel>) {
    let Some(machine) = host.machine() else {
        return;
    };
    let tabs = drawn_tabs(machine, duel.is_some());
    let first = if duel.is_some() {
        ChatTab::Table
    } else {
        ChatTab::Private
    };
    let tab = ui
        .last_chat_tab
        .filter(|t| tabs.contains(t))
        .or_else(|| tabs.contains(&first).then_some(first))
        .unwrap_or(ChatTab::Private);
    ui.open = true;
    ui.minimised = false;
    if let Some(duel) = duel {
        duel.log_open = true;
    }
    select_tab(host, ui, tab);
}

/// The doors at the table (§4.1): the log's scroll and `L` open the panel on
/// Log (or switch an open one to it, or close it there); the bubble opens it
/// on the last chat tab (or closes it there).
fn doors(
    mut host: ResMut<SocialHost>,
    mut ui: ResMut<ChatUi>,
    duel: Option<ResMut<Duel>>,
    phase: Option<Res<State<DuelPhase>>>,
) {
    let Some(mut duel) = duel else {
        return;
    };
    let playing = phase.is_some_and(|p| *p.get() != DuelPhase::Closed);
    let active = host.active() && playing;
    if duel.chat_panel != active {
        duel.chat_panel = active;
    }
    if duel.chat_door.is_none() {
        return;
    }
    let Some(door) = duel.chat_door.take() else {
        return;
    };
    if !active {
        return;
    }
    let ui = &mut *ui;
    match door {
        ChatDoor::Log => {
            // The toggle already happened: `log_open` is the state after it.
            let was_open = !duel.log_open;
            if was_open && ui.tab != ChatTab::Log {
                duel.log_open = true;
                select_tab(&mut host, ui, ChatTab::Log);
            } else if duel.log_open {
                select_tab(&mut host, ui, ChatTab::Log);
            }
        }
        ChatDoor::Chat => {
            if duel.log_open && ui.tab.is_chat() {
                duel.log_open = false;
                ui.typing = false;
                host.act(Intent::Panel(None));
            } else {
                open_on_chat(&mut host, ui, Some(&mut duel));
            }
        }
    }
    ui.open = duel.log_open;
}

/// Keeps the shown tab one that is drawn (a table ended, a guest), and the
/// panel's open state the table's.
fn keep_the_tab(
    host: Res<SocialHost>,
    mut ui: ResMut<ChatUi>,
    duel: Option<Res<Duel>>,
    phase: Option<Res<State<DuelPhase>>>,
) {
    let Some(machine) = host.machine() else {
        return;
    };
    let playing = phase.is_some_and(|p| *p.get() != DuelPhase::Closed);
    let tabs = drawn_tabs(machine, playing);
    if !tabs.contains(&ui.tab) {
        let next = if playing {
            ChatTab::Log
        } else {
            ChatTab::Private
        };
        let next = if tabs.contains(&next) {
            next
        } else {
            ChatTab::All
        };
        ui.tab = next;
    }
    if playing
        && let Some(duel) = duel
        && ui.open != duel.log_open
    {
        ui.open = duel.log_open;
        if !ui.open {
            ui.typing = false;
        }
    }
}

/// Stamps what arrived, in the order it arrived (§5, the All tab).
fn stamp_arrivals(
    host: Res<SocialHost>,
    mut ui: ResMut<ChatUi>,
    duel: Option<Res<Duel>>,
    mut seen: Local<u64>,
) {
    if let Some(duel) = duel.as_ref() {
        let len = duel.log.len();
        ui.bypass_change_detection().arrivals.log_grew(len);
    }
    let Some(machine) = host.machine() else {
        return;
    };
    if *seen == machine.revision() {
        return;
    }
    *seen = machine.revision();
    let ui = ui.bypass_change_detection();
    for conv in machine.conversations() {
        for line in &conv.lines {
            ui.arrivals.chat_line(&line.key);
        }
    }
}

/// Reading marks read (§3.3): while the panel shows a tab, the
/// conversations its filter admits have nothing unread.
fn mark_read(mut host: ResMut<SocialHost>, ui: Res<ChatUi>, windows: Query<&Window>) {
    if !ui.open || ui.minimised || ui.tab == ChatTab::Log {
        return;
    }
    if !windows.iter().any(|w| w.focused) {
        return;
    }
    let Some(machine) = host.machine() else {
        return;
    };
    let filter = tab_filter(machine, &ui, ui.tab);
    let unread: Vec<ConvId> = machine
        .conversations()
        .iter()
        .filter(|c| c.unread > 0 && filter.admits_conv(&c.id))
        .map(|c| c.id.clone())
        .collect();
    for conv in unread {
        host.act(Intent::MarkRead(conv));
    }
}

/// The window's presses.
#[allow(clippy::too_many_lines)] // one arm per control, read top to bottom
fn presses(
    pressed: Query<(&Interaction, &ChatPress), Changed<Interaction>>,
    mut host: ResMut<SocialHost>,
    mut ui: ResMut<ChatUi>,
    mut duel: Option<ResMut<Duel>>,
    mut prefs: Option<ResMut<crate::prefs::Prefs>>,
) {
    let mut clicked: Vec<ChatPress> = pressed
        .iter()
        .filter(|(i, _)| **i == Interaction::Pressed)
        .map(|(_, p)| p.clone())
        .collect();
    clicked.dedup();
    let ui = &mut *ui;
    for press in clicked {
        match press {
            ChatPress::Tab(tab) => {
                select_tab(&mut host, ui, tab);
                ui.filters = None;
            }
            ChatPress::Gear => ui.filters = Some(ui.tab).filter(|_| ui.filters.is_none()),
            ChatPress::Lock => {
                let conv = host.machine().and_then(|m| tab_conv(m, ui, ui.tab));
                ui.sheet = Some(super::sheet::Sheet::Encryption(conv));
            }
            ChatPress::Close => {
                ui.typing = false;
                ui.filters = None;
                ui.group_menu = false;
                ui.open = false;
                if let Some(duel) = duel.as_deref_mut() {
                    duel.log_open = false;
                }
                host.act(Intent::Panel(None));
            }
            ChatPress::Minimise => {
                ui.minimised = true;
                ui.typing = false;
            }
            ChatPress::Restore => {
                if ui.open && !ui.minimised {
                    ui.open = false;
                } else {
                    open_on_chat(&mut host, ui, None);
                    ui.typing = true;
                    ui.field = Field::Composer;
                }
            }
            ChatPress::Chip => {
                if let Some(machine) = host.machine.as_mut() {
                    let (_, effects) =
                        machine.key(ComposerKey::Tab, super::now_ms(), &mut super::OsEntropy);
                    host.carry_out(effects);
                }
            }
            ChatPress::Field => {
                ui.typing = true;
                ui.field = Field::Composer;
                if ui.tab == ChatTab::Log
                    && let Some(machine) = host.machine()
                {
                    // Writing from the Log tab writes to the last chat tab's
                    // conversation; the tab stays.
                    let _ = machine;
                }
            }
            ChatPress::Send => {
                if let Some(machine) = host.machine.as_mut() {
                    let (_, effects) =
                        machine.key(ComposerKey::Enter, super::now_ms(), &mut super::OsEntropy);
                    host.carry_out(effects);
                }
            }
            ChatPress::Suggestion(index) => {
                if let Some(machine) = host.machine.as_mut() {
                    machine.composer_mut().choose(index);
                }
                ui.typing = true;
            }
            ChatPress::GroupMenu => ui.group_menu = !ui.group_menu,
            ChatPress::ShowGroup(id) => {
                ui.shown_group = Some(id);
                ui.group_menu = false;
                select_tab(&mut host, ui, ChatTab::Group);
            }
            ChatPress::NewGroup => {
                ui.group_menu = false;
                ui.entry = Some((
                    super::Entry::NewGroup,
                    baylee_client_core::textbuf::TextBuffer::default(),
                ));
                ui.field = Field::Entry;
                ui.typing = true;
            }
            ChatPress::Filter(kind) => {
                let Some(tab) = ui.filters else { continue };
                let Some(machine) = host.machine() else {
                    continue;
                };
                let mut chat = machine.prefs().clone();
                let mut filter = chat.filter(tab);
                toggle(&mut filter, kind);
                chat.set_filter(tab, filter);
                save_prefs(&mut host, prefs.as_deref_mut(), chat);
            }
            ChatPress::FilterReset => {
                let Some(tab) = ui.filters else { continue };
                let Some(machine) = host.machine() else {
                    continue;
                };
                let mut chat = machine.prefs().clone();
                chat.set_filter(tab, TabFilter::standard(tab));
                save_prefs(&mut host, prefs.as_deref_mut(), chat);
            }
            ChatPress::FilterDone => ui.filters = None,
            ChatPress::Friends => ui.drawer = !ui.drawer,
        }
    }
}

/// Flips one box of a filter.
fn toggle(filter: &mut TabFilter, kind: FilterKind) {
    use baylee_client_core::social::GroupPick;
    match kind {
        FilterKind::Log => filter.log = !filter.log,
        FilterKind::Table => filter.table = !filter.table,
        FilterKind::Team => filter.team = !filter.team,
        FilterKind::Whispers => filter.whispers = !filter.whispers,
        FilterKind::Groups => {
            filter.groups = if filter.groups == GroupPick::None {
                GroupPick::All
            } else {
                GroupPick::None
            };
        }
        FilterKind::Notices => filter.notices = !filter.notices,
        FilterKind::Presence => filter.friends_presence = !filter.friends_presence,
        FilterKind::Timestamps => filter.timestamps = !filter.timestamps,
    }
}

/// Hands the machine the changed chat settings and keeps them with the
/// account (`Preferences.chat`).
pub(crate) fn save_prefs(
    host: &mut SocialHost,
    prefs: Option<&mut crate::prefs::Prefs>,
    chat: baylee_client_core::social::ChatPrefs,
) {
    if let Some(prefs) = prefs {
        prefs.edit().chat = chat.clone();
    }
    host.act(Intent::SetPrefs(chat));
}

/// A cheap fingerprint of anything hashable, for the redraw keys.
fn key_of<T: std::hash::Hash>(value: &T) -> u64 {
    use std::hash::Hasher as _;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

/// The swatch a sender wears: their seat's colour, when they hold a chair
/// at this table (§5).
fn swatch_of(duel: Option<&Duel>, tag: Option<&str>) -> Option<Color> {
    let duel = duel?;
    let statics = duel.statics.as_ref()?;
    let tag = tag?;
    let seat = statics
        .seats
        .iter()
        .find(|s| baylee_client_core::social::tag_of(&s.display_name) == Some(tag))?;
    let reader = duel.view.as_ref().map(|v| v.seat)?;
    Some(crate::hud::seat_colour(reader, Some(statics), seat.player))
}

/// The table's window: the log's panel, filled with the tabs, the list and
/// the composer while a social session exists.
#[allow(clippy::too_many_arguments)] // a Bevy system: each is what it draws from
fn sync_table_window(
    mut commands: Commands,
    host: Res<SocialHost>,
    ui: Res<ChatUi>,
    duel: Option<Res<Duel>>,
    fonts: Res<UiFonts>,
    settings: Option<Res<crate::settings::ClientSettings>>,
    texts: Option<Res<crate::cardtext::CardTexts>>,
    mut windows: ResMut<Windows>,
    panels: Query<(Entity, Option<&Children>), With<LogPanel>>,
    parts: Query<&Part>,
    mut lists: Query<&mut LogFollow, With<LogList>>,
) {
    let (Some(machine), Some(duel)) = (host.machine(), duel) else {
        windows.table = Drawn::default();
        return;
    };
    let Ok((panel, children)) = panels.single() else {
        windows.table = Drawn::default();
        return;
    };
    if !duel.log_open || duel.ending().is_some() {
        return;
    }
    let lang = settings.as_ref().map_or(Lang::En, |s| Lang::of(&s.lang));
    let drawn = &mut windows.table;
    let ours = children.is_some_and(|kids| kids.iter().any(|k| parts.contains(k)));
    if drawn.root != Some(panel) || !ours {
        if let Some(kids) = children {
            for kid in kids {
                commands.entity(*kid).despawn();
            }
        }
        *drawn = Drawn::default();
        drawn.root = Some(panel);
        let ids = skeleton(&mut commands, &fonts, Frame::Table, lang);
        commands
            .entity(panel)
            .insert(ChatSurface)
            .add_children(&ids.all());
        commands
            .entity(panel)
            .insert(RelativeCursorPosition::default());
        drawn.head = Some(ids.head);
        drawn.sub = Some(ids.sub);
        drawn.list = Some(ids.list);
        drawn.composer = Some(ids.composer);
    }
    fill(
        &mut commands,
        &fonts,
        lang,
        machine,
        &ui,
        Some(&duel),
        texts.as_deref(),
        Frame::Table,
        drawn,
        &mut lists,
    );
}

/// The lobby's window: docked bottom-right, against the friends drawer;
/// minimised to its tab row, or hidden to the chat pill.
#[allow(clippy::too_many_arguments)] // a Bevy system: each is what it draws from
fn sync_lobby_window(
    mut commands: Commands,
    host: Res<SocialHost>,
    ui: Res<ChatUi>,
    fonts: Res<UiFonts>,
    settings: Option<Res<crate::settings::ClientSettings>>,
    phase: Option<Res<State<DuelPhase>>>,
    lobby: Option<Res<crate::lobby::LobbyState>>,
    windows_q: Query<&Window>,
    mut windows: ResMut<Windows>,
    mut roots: Query<(Entity, &mut Node, &mut Visibility), With<LobbyChat>>,
    mut lists: Query<&mut LogFollow, With<LogList>>,
    mut shape: Local<Option<(bool, bool, bool, u32, i32)>>,
) {
    let at_table = phase.is_some_and(|p| *p.get() != DuelPhase::Closed);
    let shell = lobby.is_some_and(|l| {
        !matches!(
            l.lobby.screen(),
            baylee_client_core::lobby::Screen::SignIn { .. }
        )
    });
    let Some(machine) = host.machine().filter(|_| !at_table && shell) else {
        for (_, _, mut seen) in &mut roots {
            seen.set_if_neq(Visibility::Hidden);
        }
        return;
    };
    let lang = settings.as_ref().map_or(Lang::En, |s| Lang::of(&s.lang));
    let window_h = windows_q.iter().next().map_or(720.0, |w| w.height());
    let height = (window_h * 0.38).clamp(320.0, 520.0);
    let root = if let Some((root, _, _)) = roots.iter().next() {
        root
    } else {
        commands
            .spawn((
                LobbyChat,
                ChatSurface,
                RelativeCursorPosition::default(),
                Node {
                    position_type: PositionType::Absolute,
                    bottom: px(0),
                    right: px(12),
                    width: px(LOBBY_W),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                GlobalZIndex(G_PANEL),
                Visibility::Visible,
            ))
            .id()
    };
    let (unread, urgent) = machine.unread_where(&machine.prefs().filter(ChatTab::All));
    let right = if ui.drawer { 352 } else { 12 };
    let now = (ui.open, ui.minimised, urgent, unread, right);
    if let Ok((_, mut node, mut seen)) = roots.get_mut(root) {
        seen.set_if_neq(Visibility::Visible);
        let full = ui.open && !ui.minimised;
        let want_h = if full { px(height) } else { Val::Auto };
        if node.height != want_h {
            node.height = want_h;
        }
        let want_w = if ui.open { px(LOBBY_W) } else { Val::Auto };
        if node.width != want_w {
            node.width = want_w;
        }
        if node.right != px(right as f32) {
            node.right = px(right as f32);
        }
    }
    let drawn = &mut windows.lobby;
    if *shape != Some(now) || drawn.root != Some(root) {
        *shape = Some(now);
        commands.entity(root).despawn_children();
        *drawn = Drawn::default();
        drawn.root = Some(root);
        if !ui.open {
            let pill = chat_pill(&mut commands, &fonts, lang, unread, urgent);
            commands.entity(root).add_child(pill);
            return;
        }
        let ids = skeleton(&mut commands, &fonts, Frame::Lobby, lang);
        if ui.minimised {
            commands.entity(root).add_child(ids.head);
            for id in [ids.sub, ids.list, ids.composer] {
                commands.entity(id).despawn();
            }
            drawn.head = Some(ids.head);
        } else {
            commands.entity(root).add_children(&ids.all());
            drawn.head = Some(ids.head);
            drawn.sub = Some(ids.sub);
            drawn.list = Some(ids.list);
            drawn.composer = Some(ids.composer);
        }
    }
    if !ui.open {
        return;
    }
    fill(
        &mut commands,
        &fonts,
        lang,
        machine,
        &ui,
        None,
        None,
        Frame::Lobby,
        drawn,
        &mut lists,
    );
}

/// The lobby's chat pill: `Chat · 3` in the bottom-right corner.
fn chat_pill(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    unread: u32,
    urgent: bool,
) -> Entity {
    let label = if unread > 0 {
        format!("{} · {}", ChatPhrase::ChatTitle.text(lang), unread.min(99))
    } else {
        ChatPhrase::ChatTitle.text(lang).to_string()
    };
    let border = if urgent {
        rows::WHISPER_INK
    } else {
        LOBBY_INKS.line
    };
    commands
        .spawn((
            Button,
            ChatPress::Restore,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(8),
                padding: UiRect::axes(px(14), px(8)),
                margin: UiRect::bottom(px(12)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(18)),
                ..default()
            },
            BackgroundColor(LOBBY_INKS.ground),
            BorderColor::all(border),
            Feel::new(LOBBY_INKS.ground),
            children![
                (
                    Text::new(glyph::CHAT.to_string()),
                    icon_tf(fonts, 13.0),
                    TextColor(LOBBY_INKS.ink),
                    Pickable::IGNORE,
                ),
                (
                    Text::new(label),
                    tf_bold(fonts, 13.0),
                    TextColor(LOBBY_INKS.ink),
                    Pickable::IGNORE,
                ),
            ],
        ))
        .id()
}

/// The four parts of a window, spawned empty.
struct Skeleton {
    head: Entity,
    sub: Entity,
    list: Entity,
    composer: Entity,
    body: Entity,
}

impl Skeleton {
    fn all(&self) -> [Entity; 4] {
        [self.head, self.sub, self.body, self.composer]
    }
}

fn skeleton(commands: &mut Commands, fonts: &UiFonts, frame: Frame, lang: Lang) -> Skeleton {
    let inks = frame.inks();
    let head = commands
        .spawn((
            Part::Head,
            Node {
                height: px(frame.row_h()),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Stretch,
                padding: UiRect::new(px(4), px(3), px(0), px(0)),
                border: UiRect::bottom(px(1)),
                border_radius: BorderRadius::top(px(6)),
                ..default()
            },
            BackgroundColor(inks.lit),
            BorderColor::all(inks.line),
        ))
        .id();
    let sub = commands
        .spawn((
            Part::Sub,
            Node {
                display: Display::None,
                height: px(SUB_H),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::horizontal(px(logpanel::LOG_PAD)),
                border: UiRect::bottom(px(1)),
                ..default()
            },
            BackgroundColor(inks.lit),
            BorderColor::all(inks.line),
        ))
        .id();
    let body = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                flex_grow: 1.0,
                min_height: px(0),
                padding: UiRect::all(px(logpanel::LOG_PAD)),
                column_gap: px(4),
                ..default()
            },
            BackgroundColor(if frame == Frame::Lobby {
                inks.ground
            } else {
                Color::NONE
            }),
        ))
        .id();
    let list = commands
        .spawn((
            Part::List,
            LogList,
            LogFollow::default(),
            crate::hud::Scrolls,
            ScrollPosition::default(),
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                min_width: px(0),
                row_gap: px(logpanel::LOG_ROW_GAP),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    let track = logpanel::scrollbar(
        commands,
        list,
        (inks.line.with_alpha(0.35), inks.accent.with_alpha(0.8)),
        logpanel::at_rest,
    );
    let pill = logpanel::new_lines_pill(commands, fonts, lang);
    commands.entity(body).add_children(&[list, track, pill]);
    let composer = commands
        .spawn((
            Part::Composer,
            Node {
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                border: UiRect::top(px(1)),
                ..default()
            },
            BackgroundColor(inks.lit),
            BorderColor::all(inks.line),
        ))
        .id();
    Skeleton {
        head,
        sub,
        list,
        composer,
        body,
    }
}

/// Fills a window's parts, each only when what it shows changed.
#[allow(clippy::too_many_arguments)] // the window's parts and what they show
fn fill(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    machine: &Social,
    ui: &ChatUi,
    duel: Option<&Duel>,
    texts: Option<&crate::cardtext::CardTexts>,
    frame: Frame,
    drawn: &mut Drawn,
    lists: &mut Query<&mut LogFollow, With<LogList>>,
) {
    let inks = frame.inks();
    let game_open = duel.is_some();
    let tabs = drawn_tabs(machine, game_open);
    // The head.
    if let Some(head) = drawn.head {
        let counts: Vec<(ChatTab, u32, bool)> = tabs
            .iter()
            .map(|t| {
                if t.is_chat() {
                    let (n, urgent) = machine.unread_where(&tab_filter(machine, ui, *t));
                    (*t, n, urgent)
                } else {
                    (*t, 0, false)
                }
            })
            .collect();
        let lock = tab_conv(machine, ui, ui.tab).is_some();
        let key = key_of(&(
            counts.clone(),
            ui.tab,
            lock,
            lang as u8,
            ui.filters.is_some(),
        ));
        if key != drawn.head_key {
            drawn.head_key = key;
            commands.entity(head).despawn_children();
            let kids = head_parts(commands, fonts, lang, &counts, ui.tab, lock, frame);
            commands.entity(head).add_children(&kids);
        }
    }
    // The Group tab's bar.
    if let Some(sub) = drawn.sub {
        let groups: Vec<(String, String, u32)> = machine
            .contacts()
            .groups
            .iter()
            .map(|g| {
                let conv = Conv::Group { id: g.id.clone() };
                let unread = machine.conversation(&conv).map_or(0, |c| c.unread);
                (g.id.clone(), g.name.clone(), unread)
            })
            .collect();
        let shown = shown_group(machine, ui);
        let key = key_of(&(
            ui.tab == ChatTab::Group,
            groups.clone(),
            shown.clone(),
            ui.group_menu,
            lang as u8,
        ));
        if key != drawn.sub_key {
            drawn.sub_key = key;
            commands.entity(sub).despawn_children();
            if ui.tab == ChatTab::Group {
                commands.entity(sub).insert(Node {
                    display: Display::Flex,
                    height: px(SUB_H),
                    flex_shrink: 0.0,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    padding: UiRect::horizontal(px(logpanel::LOG_PAD)),
                    border: UiRect::bottom(px(1)),
                    ..default()
                });
                let kids = group_bar(
                    commands,
                    fonts,
                    lang,
                    machine,
                    &groups,
                    shown.as_deref(),
                    ui.group_menu,
                    &inks,
                );
                commands.entity(sub).add_children(&kids);
            } else {
                commands.entity(sub).insert(Node {
                    display: Display::None,
                    ..default()
                });
            }
        }
    }
    // The list.
    if let Some(list) = drawn.list {
        fill_list(
            commands, fonts, lang, machine, ui, duel, texts, frame, drawn, list, lists,
        );
    }
    // The composer.
    if let Some(composer) = drawn.composer {
        let c = machine.composer();
        let suggest: Vec<String> = c
            .suggestions()
            .map(|s| {
                s.items
                    .iter()
                    .filter_map(|i| c.suggestion_label(i).map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        let highlight = c.suggestions().map_or(0, |s| s.highlight);
        let caret = c.buffer().cursor();
        let key = key_of(&(
            c.text().to_string(),
            caret,
            format!("{:?}", c.target()),
            ui.typing && ui.field == Field::Composer,
            suggest.clone(),
            highlight,
            lang as u8,
            ui.hint.map(|(p, _)| p as u8),
            ui.tab,
        ));
        if key != drawn.composer_key {
            drawn.composer_key = key;
            commands.entity(composer).despawn_children();
            let kids = composer_parts(
                commands, fonts, lang, machine, ui, &suggest, highlight, frame,
            );
            commands.entity(composer).add_children(&kids);
        }
    }
}

/// The tab row's contents: the tabs left, the head's buttons right.
fn head_parts(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    counts: &[(ChatTab, u32, bool)],
    active: ChatTab,
    lock: bool,
    frame: Frame,
) -> Vec<Entity> {
    let inks = frame.inks();
    let mut out = Vec::new();
    for (tab, unread, urgent) in counts {
        let on = *tab == active;
        let ink = if on || *unread > 0 {
            inks.ink
        } else {
            inks.soft
        };
        let label = commands
            .spawn((
                Text::new(tab_name(*tab, lang)),
                tf(fonts, 12.5),
                TextColor(ink),
                Pickable::IGNORE,
            ))
            .id();
        let button = commands
            .spawn((
                Button,
                ChatPress::Tab(*tab),
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(4),
                    padding: UiRect::new(px(8), px(8), px(0), px(0)),
                    border: UiRect::bottom(px(2)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                BorderColor::all(if on { inks.accent } else { Color::NONE }),
                Feel::new(Color::NONE),
            ))
            .add_child(label)
            .id();
        if *unread > 0 {
            let pill = count_pill(commands, fonts, *unread, *urgent, &inks);
            commands.entity(button).add_child(pill);
        }
        out.push(button);
    }
    let spacer = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    out.push(spacer);
    let tool = |commands: &mut Commands, glyph: char, press: ChatPress| {
        commands
            .spawn((
                Button,
                press,
                Node {
                    width: px(HEAD_BTN),
                    height: px(HEAD_BTN),
                    align_self: AlignSelf::Center,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    margin: UiRect::left(px(4)),
                    border_radius: BorderRadius::all(px(4)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                Feel::new(Color::NONE),
                children![(
                    Text::new(glyph.to_string()),
                    icon_tf(fonts, 10.0),
                    TextColor(inks.soft),
                    Pickable::IGNORE,
                )],
            ))
            .id()
    };
    if lock {
        out.push(tool(commands, glyph::LOCK, ChatPress::Lock));
    }
    out.push(tool(commands, glyph::FRIENDS, ChatPress::Friends));
    out.push(tool(commands, glyph::GEAR, ChatPress::Gear));
    if frame == Frame::Lobby {
        out.push(tool(commands, glyph::MINUS, ChatPress::Minimise));
    }
    out.push(tool(commands, glyph::CLOSE, ChatPress::Close));
    out
}

/// An unread count's pill: `CANDLE` ground, rose for a whisper or a
/// mention, `99+` at most (§3.2).
pub(crate) fn count_pill(
    commands: &mut Commands,
    fonts: &UiFonts,
    count: u32,
    urgent: bool,
    inks: &Inks,
) -> Entity {
    let text = if count > 99 {
        "99+".to_string()
    } else {
        count.to_string()
    };
    let ground = if urgent {
        rows::WHISPER_INK
    } else {
        inks.accent
    };
    commands
        .spawn((
            Node {
                min_width: px(14),
                height: px(14),
                padding: UiRect::horizontal(px(4)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px(7)),
                ..default()
            },
            BackgroundColor(ground),
            Pickable::IGNORE,
            children![(
                Text::new(text),
                tf_bold(fonts, 9.0),
                TextColor(palette::DIALOG),
                Pickable::IGNORE,
            )],
        ))
        .id()
}

/// The Group tab's bar: the shown group's name with its dropdown, and its
/// member count.
#[allow(clippy::too_many_arguments)]
fn group_bar(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    machine: &Social,
    groups: &[(String, String, u32)],
    shown: Option<&str>,
    menu: bool,
    inks: &Inks,
) -> Vec<Entity> {
    let Some(shown) = shown else {
        let label = commands
            .spawn((
                Text::new(ChatPhrase::NoGroupYet.text(lang)),
                tf(fonts, 11.5),
                TextColor(inks.soft),
                Pickable::IGNORE,
            ))
            .id();
        let new = text_button(
            commands,
            fonts,
            ChatPhrase::NewGroupButton.text(lang),
            ChatPress::NewGroup,
            inks,
        );
        return vec![label, new];
    };
    let name = groups
        .iter()
        .find(|(id, _, _)| id == shown)
        .map_or_else(String::new, |(_, n, _)| n.clone());
    let group = machine.contacts().group(shown);
    let members = group.map_or(0, |g| g.members.len());
    let online = group.map_or(0, |g| {
        g.members
            .iter()
            .filter(|m| {
                m.person.tag == machine.me().tag
                    || machine.contacts().friend(&m.person.tag).is_some_and(|f| {
                        f.presence.status != baylee_protocol::social::Shown::Offline
                    })
            })
            .count()
    });
    let picker = commands
        .spawn((
            Button,
            ChatPress::GroupMenu,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(6),
                ..default()
            },
            BackgroundColor(Color::NONE),
            Feel::new(Color::NONE),
            children![
                (
                    Text::new(name),
                    tf_bold(fonts, 11.5),
                    TextColor(rows::GROUP_INK),
                    Pickable::IGNORE,
                ),
                (
                    Text::new(glyph::CHEVRON_DOWN.to_string()),
                    icon_tf(fonts, 8.0),
                    TextColor(inks.soft),
                    Pickable::IGNORE,
                ),
            ],
        ))
        .id();
    if menu {
        let list = commands
            .spawn((
                ChatSurface,
                Node {
                    position_type: PositionType::Absolute,
                    top: px(SUB_H),
                    left: px(0),
                    min_width: px(220),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(px(4)),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(6)),
                    ..default()
                },
                BackgroundColor(inks.ground),
                BorderColor::all(inks.line),
                GlobalZIndex(G_PANEL + 20),
            ))
            .id();
        for (id, name, unread) in groups {
            let tick = if id == shown { "\u{2713} " } else { "   " };
            let label = if *unread > 0 {
                format!("{tick}{name}  ({unread})")
            } else {
                format!("{tick}{name}")
            };
            let row = text_button(
                commands,
                fonts,
                &label,
                ChatPress::ShowGroup(id.clone()),
                inks,
            );
            commands.entity(list).add_child(row);
        }
        let new = text_button(
            commands,
            fonts,
            ChatPhrase::NewGroupButton.text(lang),
            ChatPress::NewGroup,
            inks,
        );
        commands.entity(list).add_child(new);
        commands.entity(picker).add_child(list);
    }
    let count = commands
        .spawn((
            Text::new(
                ChatPhrase::GroupCount.fill(lang, &[&members.to_string(), &online.to_string()]),
            ),
            tf(fonts, 11.0),
            TextColor(inks.soft),
            Pickable::IGNORE,
        ))
        .id();
    vec![picker, count]
}

/// A plain text button.
pub(crate) fn text_button(
    commands: &mut Commands,
    fonts: &UiFonts,
    label: &str,
    press: ChatPress,
    inks: &Inks,
) -> Entity {
    commands
        .spawn((
            Button,
            press,
            Node {
                padding: UiRect::axes(px(8), px(3)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            Feel::new(Color::NONE),
            children![(
                Text::new(label.to_string()),
                tf(fonts, 11.5),
                TextColor(inks.ink),
                Pickable::IGNORE,
            )],
        ))
        .id()
}

/// Fills the list: the Log tab from the book (only what arrived, when only
/// that changed), a chat tab from the machine, All by merging both in
/// arrival order.
#[allow(clippy::too_many_arguments)]
fn fill_list(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    machine: &Social,
    ui: &ChatUi,
    duel: Option<&Duel>,
    texts: Option<&crate::cardtext::CardTexts>,
    frame: Frame,
    drawn: &mut Drawn,
    list: Entity,
    lists: &mut Query<&mut LogFollow, With<LogList>>,
) {
    let inks = frame.inks();
    let tab = ui.tab;
    let filter = tab_filter(machine, ui, tab);
    let log_len = duel.map_or(0, |d| d.log.len());
    let log_identity = duel.map(|d| {
        (
            d.log.rewrites(),
            d.view.as_ref().map(|v| v.seat.get()),
            texts.map_or(0, crate::cardtext::CardTexts::generation),
        )
    });
    // On the Log tab only the book matters, and it only grows.
    if tab == ChatTab::Log {
        let Some(duel) = duel else { return };
        let key = key_of(&(tab, log_identity, lang as u8));
        if key == drawn.list_key && log_len >= drawn.log_drawn {
            if log_len == drawn.log_drawn {
                return;
            }
            let lookup = logpanel::LogTexts(texts);
            let wording = logpanel::wording(duel, lang, &lookup);
            let mut run = drawn.run;
            for line in duel.log.lines_since(drawn.log_drawn, &wording) {
                if let Some(phase) = run.heading(&line) {
                    let heading =
                        logpanel::spawn_phase(commands, fonts, lang, phase, &logpanel::PANEL_INKS);
                    commands.entity(list).add_child(heading);
                }
                let swatch = logpanel::subject_ink(&wording, line.subject);
                let row = logpanel::spawn_line(
                    commands,
                    fonts,
                    lang,
                    &line,
                    &logpanel::PANEL_INKS,
                    Some(swatch),
                );
                commands.entity(list).add_child(row);
            }
            if let Ok(mut follow) = lists.get_mut(list)
                && !follow.stuck
            {
                follow.unseen = true;
            }
            drawn.log_drawn = log_len;
            drawn.run = run;
            return;
        }
        drawn.list_key = key;
        commands.entity(list).despawn_children();
        let lookup = logpanel::LogTexts(texts);
        let wording = logpanel::wording(duel, lang, &lookup);
        let lines = duel.log.lines(&wording);
        if lines.is_empty() {
            let empty = empty_line(
                commands,
                fonts,
                baylee_client_core::i18n::Phrase::GameLogEmpty.text(lang),
                &inks,
            );
            commands.entity(list).add_child(empty);
        }
        let mut run = PhaseRun::default();
        for line in &lines {
            if let Some(phase) = run.heading(line) {
                let heading =
                    logpanel::spawn_phase(commands, fonts, lang, phase, &logpanel::PANEL_INKS);
                commands.entity(list).add_child(heading);
            }
            let swatch = logpanel::subject_ink(&wording, line.subject);
            let row = logpanel::spawn_line(
                commands,
                fonts,
                lang,
                line,
                &logpanel::PANEL_INKS,
                Some(swatch),
            );
            commands.entity(list).add_child(row);
        }
        drawn.log_drawn = log_len;
        drawn.run = run;
        if let Ok(mut follow) = lists.get_mut(list) {
            follow.stuck = true;
        }
        return;
    }
    // A chat tab, or All: drawn whole whenever a line it shows changed.
    let chat = machine.lines_where(&filter, CHAT_ROWS);
    let signature: Vec<(String, String, u8, bool)> = chat
        .iter()
        .map(|(c, l)| (c.to_string(), l.key.id.clone(), l.state as u8, l.hidden))
        .collect();
    let with_log = tab == ChatTab::All && filter.log && duel.is_some();
    let key = key_of(&(
        tab,
        format!("{filter:?}"),
        signature,
        if with_log { log_len } else { 0 },
        log_identity,
        lang as u8,
        shown_group(machine, ui),
        machine.table().map(|t| (t.game.clone(), t.playing)),
    ));
    if key == drawn.list_key {
        return;
    }
    let first_draw = drawn.list_key == 0;
    drawn.list_key = key;
    drawn.log_drawn = 0;
    commands.entity(list).despawn_children();
    let style = RowStyle {
        prefix: tab == ChatTab::All,
        timestamps: filter.timestamps,
    };
    let mut entries: Vec<(u64, u8, usize, Entity)> = Vec::new();
    for (i, (conv, line)) in chat.iter().enumerate() {
        let swatch = swatch_of(duel, line.from_tag());
        let row = rows::spawn_row(
            commands, fonts, lang, machine, conv, line, &inks, style, swatch,
        );
        let stamp = ui.arrivals.chat.get(&line.key).copied().unwrap_or(0);
        entries.push((stamp, 1, i, row));
    }
    if with_log && let Some(duel) = duel {
        let lookup = logpanel::LogTexts(texts);
        let wording = logpanel::wording(duel, lang, &lookup);
        let lines = duel.log.lines(&wording);
        let from = lines.len().saturating_sub(ALL_LOG_ROWS);
        for (i, line) in lines.iter().enumerate().skip(from) {
            // Phase captions are dropped on All; turn rules stay (§5).
            let swatch = logpanel::subject_ink(&wording, line.subject);
            let row = logpanel::spawn_line(
                commands,
                fonts,
                lang,
                line,
                &logpanel::PANEL_INKS,
                Some(swatch),
            );
            entries.push((ui.arrivals.log_stamp(i), 0, i, row));
        }
    }
    entries.sort_by_key(|(stamp, kind, index, _)| (*stamp, *kind, *index));
    if entries.is_empty() {
        let sentence = empty_sentence(machine, ui, tab, &filter, lang);
        let empty = empty_line(commands, fonts, &sentence, &inks);
        commands.entity(list).add_child(empty);
    }
    let kids: Vec<Entity> = entries.into_iter().map(|(_, _, _, e)| e).collect();
    commands.entity(list).add_children(&kids);
    if let Ok(mut follow) = lists.get_mut(list) {
        if first_draw || ui.follow || follow.stuck {
            follow.stuck = true;
        } else {
            follow.unseen = true;
        }
    }
}

/// What an empty tab says (§11).
fn empty_sentence(
    machine: &Social,
    ui: &ChatUi,
    tab: ChatTab,
    filter: &TabFilter,
    lang: Lang,
) -> String {
    if filter.is_empty() {
        return ChatPhrase::EmptyFiltered.text(lang).to_string();
    }
    match tab {
        ChatTab::Table => {
            if machine.table().is_some_and(|t| t.playing) {
                ChatPhrase::EmptyTable.text(lang).to_string()
            } else {
                ChatPhrase::EmptyRoom.text(lang).to_string()
            }
        }
        ChatTab::Team => ChatPhrase::EmptyTeam.text(lang).to_string(),
        ChatTab::Private => ChatPhrase::EmptyPrivate.text(lang).to_string(),
        ChatTab::Group => match shown_group(machine, ui)
            .and_then(|id| machine.contacts().group(&id).map(|g| g.name.clone()))
        {
            Some(name) => ChatPhrase::EmptyGroup.fill(lang, &[&name]),
            None => ChatPhrase::EmptyGroupNone.text(lang).to_string(),
        },
        ChatTab::All | ChatTab::Log => ChatPhrase::EmptyAll.text(lang).to_string(),
    }
}

fn empty_line(commands: &mut Commands, fonts: &UiFonts, text: &str, inks: &Inks) -> Entity {
    commands
        .spawn((
            Text::new(text.to_string()),
            tf_italic(fonts, inks.pt),
            TextColor(inks.soft),
            Node {
                margin: UiRect::top(px(8)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
}

/// What the chip says for a target.
pub(crate) fn target_label(machine: &Social, target: &Target, lang: Lang) -> (String, Color, Inks) {
    let inks = TABLE_INKS;
    match target {
        Target::Table => (ChatPhrase::TabTable.text(lang).to_string(), inks.ink, inks),
        Target::Team => (
            ChatPhrase::TabTeam.text(lang).to_string(),
            rows::TEAM_INK,
            inks,
        ),
        Target::Whisper(handle) => (
            ChatPhrase::ToTarget.fill(lang, &[handle]),
            rows::WHISPER_INK,
            inks,
        ),
        Target::Group(GroupRef::Number(n)) => {
            let name = machine
                .contacts()
                .groups
                .get(usize::from(*n).saturating_sub(1))
                .map_or_else(|| format!("/{n}"), |g| g.name.clone());
            (name, rows::GROUP_INK, inks)
        }
        Target::Group(GroupRef::Current) => (
            ChatPhrase::TabGroup.text(lang).to_string(),
            rows::GROUP_INK,
            inks,
        ),
        Target::Sticky => ("…".to_string(), inks.soft, inks),
    }
}

/// The composer: the chip, the box, the hint and send; the suggestions
/// stand above it.
#[allow(clippy::too_many_arguments)]
fn composer_parts(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    machine: &Social,
    ui: &ChatUi,
    suggest: &[String],
    highlight: usize,
    frame: Frame,
) -> Vec<Entity> {
    let inks = frame.inks();
    let c = machine.composer();
    let focused = ui.typing && ui.field == Field::Composer;
    let (chip_text, chip_ink, _) = target_label(machine, c.target(), lang);
    let chip = commands
        .spawn((
            Button,
            ChatPress::Chip,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(4),
                padding: UiRect::axes(px(8), px(2)),
                height: px(22),
                border_radius: BorderRadius::all(px(4)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(chip_ink.with_alpha(0.15)),
            Feel::new(chip_ink.with_alpha(0.15)),
            children![
                (
                    Text::new(chip_text),
                    tf_bold(fonts, 11.5),
                    TextColor(chip_ink),
                    Pickable::IGNORE,
                ),
                (
                    Text::new(glyph::CHEVRON_DOWN.to_string()),
                    icon_tf(fonts, 7.0),
                    TextColor(chip_ink),
                    Pickable::IGNORE,
                ),
            ],
        ))
        .id();
    let text = c.text();
    let field = commands
        .spawn((
            Button,
            ChatPress::Field,
            Text::default(),
            tf(fonts, inks.pt),
            TextColor(inks.ink),
            Node {
                flex_grow: 1.0,
                min_width: px(0),
                padding: UiRect::axes(px(6), px(4)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BorderColor::all(if focused { inks.accent } else { Color::NONE }),
        ))
        .id();
    if text.is_empty() && !focused {
        let placeholder = placeholder(machine, c.target(), lang);
        let span = commands
            .spawn((
                TextSpan::new(placeholder),
                tf_italic(fonts, inks.pt),
                TextColor(inks.soft),
            ))
            .id();
        commands.entity(field).add_child(span);
    } else {
        let buffer = c.buffer();
        let caret = buffer.cursor().min(text.len());
        let selection = buffer.selection();
        let (before, after) = text.split_at(caret);
        let mut spans: Vec<(String, Color)> = Vec::new();
        match selection {
            Some(range) if focused && range.start < range.end && range.end <= text.len() => {
                spans.push((text[..range.start].to_string(), inks.ink));
                spans.push((text[range.clone()].to_string(), inks.accent));
                spans.push((text[range.end..].to_string(), inks.ink));
            }
            _ => {
                spans.push((before.to_string(), inks.ink));
                if focused {
                    spans.push(("\u{258f}".to_string(), inks.accent));
                }
                spans.push((after.to_string(), inks.ink));
            }
        }
        for (part, colour) in spans {
            if part.is_empty() {
                continue;
            }
            let span = commands
                .spawn((TextSpan::new(part), tf(fonts, inks.pt), TextColor(colour)))
                .id();
            commands.entity(field).add_child(span);
        }
    }
    let count = c.chars();
    let hint_text = if count >= 900 {
        format!("{count} / 1000")
    } else if let Some((phrase, _)) = ui.hint {
        phrase.text(lang).to_string()
    } else {
        ChatPhrase::ComposeHint.text(lang).to_string()
    };
    let hint = commands
        .spawn((
            Text::new(hint_text),
            tf(fonts, 10.5),
            TextColor(if ui.hint.is_some() {
                palette::DANGER
            } else {
                inks.soft
            }),
            Node {
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let send = commands
        .spawn((
            Button,
            ChatPress::Send,
            Node {
                width: px(26),
                height: px(24),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px(4)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(Color::NONE),
            Feel::new(Color::NONE),
            children![(
                Text::new(glyph::SEND.to_string()),
                icon_tf(fonts, 10.0),
                TextColor(inks.soft),
                Pickable::IGNORE,
            )],
        ))
        .id();
    let row = commands
        .spawn((Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(6),
            padding: UiRect::axes(px(6), px(5)),
            min_height: px(36),
            ..default()
        },))
        .add_children(&[chip, field, hint, send])
        .id();
    let mut out = vec![row];
    if !suggest.is_empty() {
        let pop = commands
            .spawn((
                ChatSurface,
                Node {
                    position_type: PositionType::Absolute,
                    bottom: px(40),
                    left: px(60),
                    width: px(320),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(px(4)),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(6)),
                    ..default()
                },
                BackgroundColor(inks.ground),
                BorderColor::all(inks.accent.with_alpha(0.6)),
                GlobalZIndex(G_PANEL + 30),
            ))
            .id();
        for (i, label) in suggest.iter().take(6).enumerate() {
            let lit = i == highlight;
            let row = commands
                .spawn((
                    Button,
                    ChatPress::Suggestion(i),
                    Node {
                        padding: UiRect::axes(px(8), px(3)),
                        border_radius: BorderRadius::all(px(3)),
                        ..default()
                    },
                    BackgroundColor(if lit { inks.lit } else { Color::NONE }),
                    children![(
                        Text::new(format!("{}{label}", if lit { "\u{25b8} " } else { "  " })),
                        tf(fonts, 12.0),
                        TextColor(if lit { inks.ink } else { inks.soft }),
                        Pickable::IGNORE,
                    )],
                ))
                .id();
            commands.entity(pop).add_child(row);
        }
        out.push(pop);
    }
    out
}

/// The box's placeholder (§8).
fn placeholder(machine: &Social, target: &Target, lang: Lang) -> String {
    match target {
        Target::Whisper(handle) => ChatPhrase::WriteWhisper.fill(lang, &[handle]),
        Target::Sticky => ChatPhrase::TypeWhisper.text(lang).to_string(),
        other => {
            let (name, _, _) = target_label(machine, other, lang);
            ChatPhrase::WriteTo.fill(lang, &[&name])
        }
    }
}

/// Hovering a card link in a chat line at the table opens the table's own
/// preview, when the game knows a printing of that card (§12.3).
fn hover_chat_links(
    mut overs: MessageReader<bevy::picking::events::Pointer<bevy::picking::events::Over>>,
    mut outs: MessageReader<bevy::picking::events::Pointer<bevy::picking::events::Out>>,
    links: Query<&rows::ChatCardLink>,
    duel: Option<ResMut<Duel>>,
) {
    let Some(mut duel) = duel else {
        overs.clear();
        outs.clear();
        return;
    };
    for over in overs.read() {
        let Ok(link) = links.get(over.entity) else {
            continue;
        };
        let identity = duel.view.as_ref().and_then(|view| {
            view.battlefield
                .iter()
                .chain(view.stack.iter())
                .chain(view.graveyards.iter().flatten())
                .chain(view.exile.iter().flatten())
                .chain(view.command.iter().flatten())
                .find_map(|o| o.card.filter(|c| c.index.get() == link.card))
        });
        if let Some(identity) = identity {
            duel.hovered_log = Some(crate::hud::LogHover {
                span: over.entity,
                link: crate::hud::LogLink {
                    card: Some(baylee_view::CardIdentity {
                        face: link.face,
                        ..identity
                    }),
                    token: None,
                },
                at: over.pointer_location.position,
                card: None,
            });
        }
    }
    for out in outs.read() {
        if links.contains(out.entity) && duel.hovered_log.is_some_and(|h| h.span == out.entity) {
            duel.hovered_log = None;
        }
    }
}

/// The peek (§2.3): while the table's panel is shut, new chat lines show in
/// its footprint for 10 s and fade in 0.6 s; nothing there takes a press.
#[derive(Component)]
pub(crate) struct PeekRoot;

#[allow(clippy::too_many_arguments)]
fn peek(
    mut commands: Commands,
    time: Res<Time>,
    host: Res<SocialHost>,
    mut ui: ResMut<ChatUi>,
    duel: Option<Res<Duel>>,
    fonts: Res<UiFonts>,
    settings: Option<Res<crate::settings::ClientSettings>>,
    roots: Query<(Entity, Option<&Children>), With<PeekRoot>>,
    mut texts: Query<&mut TextColor>,
    mut last: Local<Option<(u64, usize)>>,
    mut drawn: Local<usize>,
) {
    let Some(machine) = host.machine() else {
        return;
    };
    let Some(duel) = duel else {
        return;
    };
    let lang = settings.as_ref().map_or(Lang::En, |s| Lang::of(&s.lang));
    // Which chat lines are new since the last look (never log lines).
    let chat_filter = TabFilter {
        log: false,
        ..TabFilter::standard(ChatTab::All)
    };
    if *last != Some((machine.revision(), 0)) {
        let lines = machine.lines_where(&chat_filter, 8);
        let newest: Vec<(String, u64)> = lines
            .iter()
            .filter(|(c, l)| {
                !matches!(
                    &l.kind,
                    baylee_client_core::social::LineKind::Message { mine: true, .. }
                ) && matches!(l.kind, baylee_client_core::social::LineKind::Message { .. })
                    && !l.hidden
                    && !machine.conversation(c).is_some_and(|c| c.muted)
            })
            .map(|(c, l)| {
                let stamp = ui.arrivals.chat.get(&l.key).copied().unwrap_or(0);
                (
                    rows::plain_text(machine, c, l)
                        .split_once(' ')
                        .map_or(String::new(), |(_, rest)| rest.trim().to_string()),
                    stamp,
                )
            })
            .collect();
        let seen_up_to = last.map_or(u64::MAX, |(_, s)| s as u64);
        let top = newest.iter().map(|(_, s)| *s).max().unwrap_or(0);
        if !duel.log_open && last.is_some() {
            for (text, stamp) in &newest {
                if *stamp > seen_up_to {
                    ui.peek.push((text.clone(), 0.0));
                }
            }
            while ui.peek.len() > 4 {
                ui.peek.remove(0);
            }
        }
        let _ = lang;
        *last = Some((machine.revision(), 0));
        if let Some((_, s)) = last.as_mut() {
            *s = usize::try_from(top).unwrap_or(0);
        }
    }
    if duel.log_open && !ui.peek.is_empty() {
        ui.peek.clear();
    }
    // Age the lines; drop the faded.
    if !ui.peek.is_empty() {
        let dt = time.delta_secs();
        for (_, age) in &mut ui.peek {
            *age += dt;
        }
        ui.peek.retain(|(_, age)| *age < 10.6);
    }
    let root = if let Ok((root, _)) = roots.single() {
        root
    } else {
        commands
            .spawn((
                PeekRoot,
                Node {
                    position_type: PositionType::Absolute,
                    right: px(crate::hud::EDGE),
                    bottom: logpanel::root_node().bottom,
                    max_width: px(logpanel::LOG_W),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexEnd,
                    ..default()
                },
                ZIndex(crate::hud::Z_LOG),
                Pickable::IGNORE,
            ))
            .id()
    };
    if *drawn != ui.peek.len() {
        *drawn = ui.peek.len();
        commands.entity(root).despawn_children();
        for (text, _) in &ui.peek {
            let line = commands
                .spawn((
                    Text::new(text.clone()),
                    tf(&fonts, 12.5),
                    TextColor(palette::DIALOG_INK),
                    TextShadow::default(),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(root).add_child(line);
        }
    }
    // Fade the last 0.6 s.
    if let Ok((_, Some(kids))) = roots.single() {
        for (kid, (_, age)) in kids.iter().zip(ui.peek.iter()) {
            if *age > 10.0
                && let Ok(mut colour) = texts.get_mut(kid)
            {
                let alpha = (1.0 - (*age - 10.0) / 0.6).clamp(0.0, 1.0);
                colour.set_if_neq(TextColor(palette::DIALOG_INK.with_alpha(alpha)));
            }
        }
    }
}

/// The filter dialog's root.
#[derive(Component)]
pub(crate) struct FilterDialog;

/// The tab's filter dialog (§3.5): a small sheet over the panel, a box per
/// kind of line and the timestamps, Reset and Done.
#[allow(clippy::too_many_arguments)]
fn filter_dialog(
    mut commands: Commands,
    host: Res<SocialHost>,
    ui: Res<ChatUi>,
    fonts: Res<UiFonts>,
    settings: Option<Res<crate::settings::ClientSettings>>,
    phase: Option<Res<State<DuelPhase>>>,
    roots: Query<Entity, With<FilterDialog>>,
    mut drawn: Local<Option<(ChatTab, String)>>,
) {
    let Some((machine, tab)) = host.machine().zip(ui.filters) else {
        for root in &roots {
            commands.entity(root).despawn();
        }
        *drawn = None;
        return;
    };
    let filter = machine.prefs().filter(tab);
    let key = (tab, format!("{filter:?}"));
    if drawn.as_ref() == Some(&key) && !roots.is_empty() {
        return;
    }
    *drawn = Some(key);
    for root in &roots {
        commands.entity(root).despawn();
    }
    let lang = settings.as_ref().map_or(Lang::En, |s| Lang::of(&s.lang));
    let at_table = phase.is_some_and(|p| *p.get() != DuelPhase::Closed);
    let inks = if at_table { TABLE_INKS } else { LOBBY_INKS };
    let root = commands
        .spawn((
            FilterDialog,
            ChatSurface,
            RelativeCursorPosition::default(),
            Node {
                position_type: PositionType::Absolute,
                right: px(40),
                bottom: px(if at_table { 300.0 } else { 120.0 }),
                width: px(340),
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                padding: UiRect::all(px(14)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(8)),
                ..default()
            },
            BackgroundColor(inks.ground),
            BorderColor::all(inks.accent.with_alpha(0.6)),
            GlobalZIndex(G_PANEL + 45),
        ))
        .id();
    let title = commands
        .spawn((
            Text::new(ChatPhrase::FilterTitle.fill(lang, &[tab_name(tab, lang)])),
            tf_bold(&fonts, 13.0),
            TextColor(inks.ink),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(title);
    let groups_on = filter.groups != baylee_client_core::social::GroupPick::None;
    let mut boxes = vec![
        (FilterKind::Table, ChatPhrase::KindTable, filter.table),
        (FilterKind::Team, ChatPhrase::KindTeam, filter.team),
        (
            FilterKind::Whispers,
            ChatPhrase::KindWhispers,
            filter.whispers,
        ),
        (FilterKind::Groups, ChatPhrase::KindGroups, groups_on),
        (FilterKind::Notices, ChatPhrase::KindNotices, filter.notices),
    ];
    if at_table {
        boxes.insert(0, (FilterKind::Log, ChatPhrase::KindLog, filter.log));
    } else {
        boxes.push((
            FilterKind::Presence,
            ChatPhrase::KindPresence,
            filter.friends_presence,
        ));
    }
    boxes.push((
        FilterKind::Timestamps,
        ChatPhrase::KindTimestamps,
        filter.timestamps,
    ));
    for (kind, phrase, on) in boxes {
        let mark = if on { "\u{2611}" } else { "\u{2610}" };
        let row = commands
            .spawn((
                Button,
                ChatPress::Filter(kind),
                Node {
                    padding: UiRect::axes(px(4), px(3)),
                    border_radius: BorderRadius::all(px(4)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                Feel::new(Color::NONE),
                children![(
                    Text::new(format!("{mark}  {}", phrase.text(lang))),
                    tf(&fonts, 12.5),
                    TextColor(if on { inks.ink } else { inks.soft }),
                    Pickable::IGNORE,
                )],
            ))
            .id();
        commands.entity(root).add_child(row);
    }
    let note = commands
        .spawn((
            Text::new(ChatPhrase::FilterNote.text(lang)),
            tf_italic(&fonts, 11.0),
            TextColor(inks.soft),
            Pickable::IGNORE,
        ))
        .id();
    let buttons = commands
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::FlexEnd,
            column_gap: px(8),
            ..default()
        })
        .id();
    let reset = text_button(
        &mut commands,
        &fonts,
        ChatPhrase::Reset.text(lang),
        ChatPress::FilterReset,
        &inks,
    );
    let done = text_button(
        &mut commands,
        &fonts,
        ChatPhrase::Done.text(lang),
        ChatPress::FilterDone,
        &inks,
    );
    commands.entity(buttons).add_children(&[reset, done]);
    commands.entity(root).add_children(&[note, buttons]);
}
