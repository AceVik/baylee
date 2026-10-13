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

use super::zoom::{z, zpx};
use crate::legacy_ui::{LegacyButton, LegacyInteraction};
use baylee_client_core::i18n::Lang;
use baylee_client_core::social::{
    ChatPhrase, ChatTab, ComposerKey, ConvId, GroupRef, Intent, Social, TabFilter, TableContext,
    Target,
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
    /// The encryption line under the tabs: the encryption sheet.
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
    Note,
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
    note: Option<Entity>,
    list: Option<Entity>,
    composer: Option<Entity>,
    head_key: u64,
    sub_key: u64,
    note_key: u64,
    list_key: u64,
    composer_key: u64,
    /// On the Log tab: how many of the book's lines are drawn, and where
    /// they left the phase headings.
    log_lines: usize,
    run: PhaseRun,
    /// A chat tab's drawn lines, in the order asked, and everything else
    /// the list was drawn from: a line more on the same list is appended.
    chat_lines: Vec<(String, String, u8, bool)>,
    chat_base: u64,
    /// The tab the list shows and the newest arrival stamp drawn on it: a
    /// row newer than that arrives with a movement.
    shown_tab: Option<ChatTab>,
    newest: u64,
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
            note_the_marker,
            mark_read,
            presses,
            panel_follows_the_window,
            sync_table_window
                .after(crate::hud::sync_log)
                .before(crate::hud::follow_the_log),
            pop_the_lobby_chat,
            arrive,
            sync_lobby_window,
            follow_in_the_lobby.run_if(in_state(DuelPhase::Closed)),
            wheel_in_the_lobby
                .run_if(in_state(DuelPhase::Closed))
                .run_if(resource_exists::<Messages<bevy::picking::events::PointerScroll>>),
            hover_chat_links,
            lobby_card_links.run_if(in_state(DuelPhase::Closed)),
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
        ChatTab::Table => machine.table().and_then(TableContext::table_conv),
        ChatTab::Team => machine.table().and_then(TableContext::team_conv),
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
    // The "New" divider is noted before the machine reads the tab's
    // conversation (`Intent::Panel` clears its count).
    ui.marker = host.machine().and_then(|m| first_unread(m, ui, tab));
    let conv = host.machine().and_then(|m| tab_conv(m, ui, tab));
    if let Some(conv) = conv {
        host.act(Intent::Panel(Some(conv)));
    }
}

/// The first line of `tab` that is not read yet, by arrival: where its
/// "New" divider stands.
fn first_unread(
    machine: &Social,
    ui: &ChatUi,
    tab: ChatTab,
) -> Option<(ChatTab, baylee_client_core::social::LineKey)> {
    if !tab.is_chat() && tab != ChatTab::All {
        return None;
    }
    let filter = tab_filter(machine, ui, tab);
    machine
        .conversations()
        .iter()
        .filter(|c| c.unread > 0 && filter.admits_conv(&c.id))
        .filter_map(|c| {
            let first = c.lines.len().checked_sub(c.unread as usize)?;
            c.lines.get(first).map(|l| l.key.clone())
        })
        .min_by_key(|k| ui.arrivals.chat.get(k).copied().unwrap_or(u64::MAX))
        .map(|k| (tab, k))
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
    if ui.at_table != playing {
        ui.at_table = playing;
    }
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
    // What is new to this stamp arrives in its own time order: a session's
    // history loaded from disk reads as it was written, not conversation
    // by conversation.
    let mut fresh: Vec<(u64, &baylee_client_core::social::LineKey)> = machine
        .conversations()
        .iter()
        .flat_map(|conv| conv.lines.iter())
        .filter(|line| !ui.arrivals.chat.contains_key(&line.key))
        .map(|line| (line.at, &line.key))
        .collect();
    fresh.sort_by_key(|(at, _)| *at);
    for (_, key) in fresh {
        ui.arrivals.chat_line(key);
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

/// The machine is told when the window stops showing its conversation and
/// when it shows it again, however that came about (the cross, the header's
/// button, the pill, minimising): a line arriving under the pill is unread.
fn panel_follows_the_window(
    mut host: ResMut<SocialHost>,
    mut ui: ResMut<ChatUi>,
    mut was: Local<bool>,
) {
    let shown = ui.open && !ui.minimised;
    if *was == shown {
        return;
    }
    *was = shown;
    let Some(machine) = host.machine() else {
        return;
    };
    if !shown {
        host.act(Intent::Panel(None));
        return;
    }
    // Shown again: the divider is noted before the machine reads the tab.
    let marker = first_unread(machine, &ui, ui.tab);
    let conv = tab_conv(machine, &ui, ui.tab);
    if marker.is_some() {
        ui.marker = marker;
    }
    if let Some(conv) = conv {
        host.act(Intent::Panel(Some(conv)));
    }
}

/// The window's presses.
#[allow(clippy::too_many_lines)] // one arm per control, read top to bottom
fn presses(
    pressed: Query<(Entity, &LegacyInteraction, &ChatPress), Changed<LegacyInteraction>>,
    mut held: Local<std::collections::HashSet<Entity>>,
    mut host: ResMut<SocialHost>,
    mut ui: ResMut<ChatUi>,
    mut duel: Option<ResMut<Duel>>,
    mut prefs: Option<ResMut<crate::prefs::Prefs>>,
) {
    let mut clicked: Vec<ChatPress> = super::released(&pressed, &mut held);
    clicked.dedup();
    let ui = &mut *ui;
    for press in clicked {
        match press {
            ChatPress::Tab(tab) => {
                select_tab(&mut host, ui, tab);
                ui.filters = None;
            }
            ChatPress::Gear => {
                ui.filters = if ui.filters.is_none() {
                    Some(ui.tab)
                } else {
                    None
                };
            }
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
    zoom: Res<super::zoom::Zoom>,
) {
    let _zoom = super::zoom::Scope::new(zoom.0);
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
        drawn.note = Some(ids.note);
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
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // a Bevy system: each is what it draws from
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
    mut roots: Query<(Entity, &mut Node, &mut Visibility, &mut Pop), With<LobbyChat>>,
    mut lists: Query<&mut LogFollow, With<LogList>>,
    mut shape: Local<Option<LobbyShape>>,
    slide: Res<super::drawer::DrawerSlide>,
    zoom: Res<super::zoom::Zoom>,
) {
    let _zoom = super::zoom::Scope::new(zoom.0);
    let at_table = phase.is_some_and(|p| *p.get() != DuelPhase::Closed);
    let shell = lobby.is_some_and(|l| {
        !matches!(
            l.lobby.screen(),
            baylee_client_core::lobby::Screen::SignIn { .. }
        )
    });
    let Some(machine) = host.machine().filter(|_| !at_table && shell) else {
        for (_, _, mut seen, _) in &mut roots {
            seen.set_if_neq(Visibility::Hidden);
        }
        return;
    };
    let lang = settings.as_ref().map_or(Lang::En, |s| Lang::of(&s.lang));
    let window_h = windows_q.iter().next().map_or(720.0, Window::height);
    let height = lobby_panel_height(window_h);
    let root = if let Some((root, ..)) = roots.iter().next() {
        root
    } else {
        commands
            .spawn((
                LobbyChat,
                ChatSurface,
                RelativeCursorPosition::default(),
                Node {
                    position_type: PositionType::Absolute,
                    bottom: zpx(0.0),
                    right: zpx(12.0),
                    width: zpx(LOBBY_W),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                GlobalZIndex(G_PANEL),
                Visibility::Visible,
                Pop::default(),
            ))
            .id()
    };
    let (unread, urgent) = machine.unread_where(&machine.prefs().filter(ChatTab::All));
    let right: f32 = slide.room(1.0) + 12.0;
    // Only the pill shows the count: an open panel is not drawn again for
    // a line arriving (its list is), nor for the drawer moving it aside.
    let now = if ui.open {
        (true, ui.minimised, false, 0, 0)
    } else {
        (false, false, urgent, unread, 0)
    };
    if let Ok((_, mut node, mut seen, mut pop)) = roots.get_mut(root) {
        seen.set_if_neq(Visibility::Visible);
        // A panel that was closed shrinks away as it was drawn, and only
        // then becomes the pill; a pill that changes, or a panel that opens,
        // pops.
        let was_open = shape.is_some_and(|s| s.0);
        if was_open && !ui.open {
            if !pop.closing {
                *pop = Pop::CLOSING;
            }
            if pop.t < 1.0 {
                return;
            }
        }
        let grew = shape.is_some_and(|s| !s.0 && unread > s.3);
        if *shape != Some(now) && (was_open != ui.open || grew || shape.is_none()) {
            *pop = Pop::OPENING;
        } else if pop.closing && ui.open {
            // Opened again while it shrank: it grows back from there.
            *pop = Pop::OPENING;
        }
        let full = ui.open && !ui.minimised;
        let want_h = if full { Val::Px(height) } else { Val::Auto };
        if node.height != want_h {
            node.height = want_h;
        }
        let want_w = if ui.open { zpx(LOBBY_W) } else { Val::Auto };
        if node.width != want_w {
            node.width = want_w;
        }
        let right = Val::Px(z(right));
        if node.right != right {
            node.right = right;
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
            for id in [ids.sub, ids.note, ids.body, ids.composer] {
                commands.entity(id).despawn();
            }
            drawn.head = Some(ids.head);
        } else {
            commands.entity(root).add_children(&ids.all());
            drawn.head = Some(ids.head);
            drawn.sub = Some(ids.sub);
            drawn.note = Some(ids.note);
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

/// What the lobby panel's frame was drawn as: open, minimised, a whisper
/// waiting, the unread count, the drawer standing.
type LobbyShape = (bool, bool, bool, u32, u32);

/// The lobby window's opening and closing, on [`crate::hud::motion`]'s
/// curve: it grows from the bottom-right corner it hangs off, and shrinks
/// back into it before the pill takes its place.
#[derive(Component, Clone, Copy, PartialEq, Debug)]
pub(crate) struct Pop {
    /// How far it is, 0 → 1.
    pub(crate) t: f32,
    /// Shrinking away rather than growing.
    pub(crate) closing: bool,
}

impl Pop {
    const OPENING: Self = Self {
        t: 0.0,
        closing: false,
    };
    const CLOSING: Self = Self {
        t: 0.0,
        closing: true,
    };
}

impl Default for Pop {
    fn default() -> Self {
        Self::OPENING
    }
}

/// Moves each [`Pop`] a frame on (`reduce_motion`: to its end at once).
pub(crate) fn pop_the_lobby_chat(
    time: Res<Time>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut roots: Query<(&mut Pop, &mut UiTransform)>,
) {
    use crate::hud::motion;
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    for (mut pop, mut transform) in &mut roots {
        if pop.t >= 1.0 && !pop.closing && transform.scale == Vec2::ONE {
            continue;
        }
        let span = if pop.closing {
            motion::ZOOM_OUT
        } else {
            motion::ZOOM_IN
        };
        let t = motion::step(pop.t, span, time.delta_secs(), still);
        if (t - pop.t).abs() > f32::EPSILON {
            pop.t = t;
        }
        let scale = if pop.closing {
            motion::shutting(pop.t)
        } else {
            motion::opening(pop.t)
        };
        let scale = if pop.t >= 1.0 && !pop.closing {
            1.0
        } else {
            scale
        };
        transform.scale = Vec2::splat(scale);
        transform.translation = motion::from_bottom_right(scale);
    }
}

/// The lobby panel's height in a window this tall (SOCIAL §11.4: 38 %,
/// between 320 and 520).
pub(crate) fn lobby_panel_height(window_h: f32) -> f32 {
    (window_h * 0.38).clamp(320.0, 520.0)
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
            LegacyButton,
            ChatPress::Restore,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: zpx(8.0),
                padding: UiRect::axes(zpx(14.0), zpx(8.0)),
                margin: UiRect::bottom(zpx(12.0)),
                border: UiRect::all(zpx(1.0)),
                border_radius: BorderRadius::all(zpx(18.0)),
                ..default()
            },
            BackgroundColor(LOBBY_INKS.ground),
            BorderColor::all(border),
            Feel::new(LOBBY_INKS.ground),
            children![
                (
                    Text::new(glyph::CHAT.to_string()),
                    icon_tf(fonts, z(13.0)),
                    TextColor(LOBBY_INKS.ink),
                    Pickable::IGNORE,
                ),
                (
                    Text::new(label),
                    tf_bold(fonts, z(13.0)),
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
    note: Entity,
    list: Entity,
    composer: Entity,
    body: Entity,
}

impl Skeleton {
    fn all(&self) -> [Entity; 5] {
        [self.head, self.sub, self.note, self.body, self.composer]
    }
}

#[allow(clippy::too_many_lines)] // one window's parts, top to bottom
fn skeleton(commands: &mut Commands, fonts: &UiFonts, frame: Frame, lang: Lang) -> Skeleton {
    let inks = frame.inks();
    let head = commands
        .spawn((
            Part::Head,
            Node {
                height: zpx(frame.row_h()),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Stretch,
                padding: UiRect::new(zpx(4.0), zpx(3.0), zpx(0.0), zpx(0.0)),
                border: UiRect::bottom(zpx(1.0)),
                border_radius: BorderRadius::top(zpx(6.0)),
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
                height: zpx(SUB_H),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::horizontal(zpx(logpanel::LOG_PAD)),
                border: UiRect::bottom(zpx(1.0)),
                ..default()
            },
            BackgroundColor(inks.lit),
            BorderColor::all(inks.line),
        ))
        .id();
    // The encryption's quiet line under the tabs (owner, 13.10.): a press
    // opens the sheet with the safety numbers.
    let note = commands
        .spawn((
            Part::Note,
            LegacyButton,
            ChatPress::Lock,
            Node {
                display: Display::None,
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: zpx(6.0),
                padding: UiRect::axes(zpx(logpanel::LOG_PAD), zpx(3.0)),
                ..default()
            },
            BackgroundColor(inks.ground),
            Feel::new(inks.ground),
        ))
        .id();
    let body = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                flex_grow: 1.0,
                min_height: zpx(0.0),
                padding: UiRect::all(zpx(logpanel::LOG_PAD)),
                column_gap: zpx(4.0),
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
                min_width: zpx(0.0),
                row_gap: zpx(logpanel::LOG_ROW_GAP),
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
                border: UiRect::top(zpx(1.0)),
                ..default()
            },
            BackgroundColor(inks.lit),
            BorderColor::all(inks.line),
        ))
        .id();
    Skeleton {
        head,
        sub,
        note,
        list,
        composer,
        body,
    }
}

/// Fills a window's parts, each only when what it shows changed.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // the window's parts and what they show
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
        let key = key_of(&(counts.clone(), ui.tab, lang as u8, ui.filters.is_some()));
        if key != drawn.head_key {
            drawn.head_key = key;
            commands.entity(head).despawn_children();
            let kids = head_parts(commands, fonts, lang, &counts, ui.tab, frame);
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
                    height: zpx(SUB_H),
                    flex_shrink: 0.0,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    padding: UiRect::horizontal(zpx(logpanel::LOG_PAD)),
                    border: UiRect::bottom(zpx(1.0)),
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
    // The encryption's line.
    if let Some(note) = drawn.note {
        let conv = tab_conv(machine, ui, ui.tab);
        let to = conv.as_ref().and_then(|c| machine.encrypted_to(c));
        let verified = conv
            .as_ref()
            .and_then(|c| c.partner(&machine.me().tag))
            .is_some_and(|t| machine.safety(t).is_some_and(|s| s.verified));
        let key = key_of(&(
            conv.as_ref().map(ToString::to_string),
            to,
            verified,
            lang as u8,
        ));
        if key != drawn.note_key {
            drawn.note_key = key;
            commands.entity(note).despawn_children();
            if conv.is_some() {
                // "End-to-end encrypted · Bob#0002 · 2 devices": a whisper
                // names its partner, a group or table counts its people.
                let partner = conv
                    .as_ref()
                    .and_then(|c| c.partner(&machine.me().tag))
                    .map(|t| machine.person(t).handle);
                let words = match to {
                    Some((people, devices)) if people > 0 => {
                        let who = partner.unwrap_or_else(|| {
                            if people == 1 {
                                ChatPhrase::NoteOnePerson.text(lang).to_string()
                            } else {
                                ChatPhrase::NotePeople.fill(lang, &[&people.to_string()])
                            }
                        });
                        let devices = if devices == 1 {
                            ChatPhrase::NoteOneDevice.text(lang).to_string()
                        } else {
                            ChatPhrase::NoteDevices.fill(lang, &[&devices.to_string()])
                        };
                        format!(
                            "{} · {who} · {devices}",
                            ChatPhrase::EncryptionTitle.text(lang)
                        )
                    }
                    _ => ChatPhrase::EncryptionTitle.text(lang).to_string(),
                };
                let ink = if verified { inks.accent } else { inks.soft };
                let lock = commands
                    .spawn((
                        Text::new(glyph::LOCK.to_string()),
                        icon_tf(fonts, z(8.5)),
                        TextColor(ink),
                        Pickable::IGNORE,
                    ))
                    .id();
                let text = commands
                    .spawn((
                        Text::new(words),
                        tf(fonts, z(10.5)),
                        TextColor(inks.soft),
                        Pickable::IGNORE,
                    ))
                    .id();
                commands.entity(note).add_children(&[lock, text]);
                commands.entity(note).insert(Node {
                    display: Display::Flex,
                    flex_shrink: 0.0,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: zpx(6.0),
                    padding: UiRect::axes(zpx(logpanel::LOG_PAD), zpx(3.0)),
                    ..default()
                });
            } else {
                commands.entity(note).insert(Node {
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
                tf(fonts, z(12.5)),
                TextColor(ink),
                Pickable::IGNORE,
            ))
            .id();
        let button = commands
            .spawn((
                LegacyButton,
                ChatPress::Tab(*tab),
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: zpx(4.0),
                    padding: UiRect::new(zpx(8.0), zpx(8.0), zpx(0.0), zpx(0.0)),
                    border: UiRect::bottom(zpx(2.0)),
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
                LegacyButton,
                press,
                Node {
                    width: zpx(HEAD_BTN),
                    height: zpx(HEAD_BTN),
                    align_self: AlignSelf::Center,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    margin: UiRect::left(zpx(4.0)),
                    border_radius: BorderRadius::all(zpx(4.0)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                Feel::new(Color::NONE),
                children![(
                    Text::new(glyph.to_string()),
                    icon_tf(fonts, z(10.0)),
                    TextColor(inks.soft),
                    Pickable::IGNORE,
                )],
            ))
            .id()
    };
    // The lock is the line under the tabs (owner, 13.10.), not a tool here.
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
                min_width: zpx(14.0),
                height: zpx(14.0),
                padding: UiRect::horizontal(zpx(4.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(zpx(7.0)),
                ..default()
            },
            BackgroundColor(ground),
            Pickable::IGNORE,
            children![(
                Text::new(text),
                tf_bold(fonts, z(9.0)),
                TextColor(palette::DIALOG),
                Pickable::IGNORE,
            )],
        ))
        .id()
}

/// The Group tab's bar: the shown group's name with its dropdown, and its
/// member count.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // one part, top to bottom
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
                tf(fonts, z(11.5)),
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
            LegacyButton,
            ChatPress::GroupMenu,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: zpx(6.0),
                ..default()
            },
            BackgroundColor(Color::NONE),
            Feel::new(Color::NONE),
            children![
                (
                    Text::new(name),
                    tf_bold(fonts, z(11.5)),
                    TextColor(rows::GROUP_INK),
                    Pickable::IGNORE,
                ),
                (
                    Text::new(glyph::CHEVRON_DOWN.to_string()),
                    icon_tf(fonts, z(8.0)),
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
                    top: zpx(SUB_H),
                    left: zpx(0.0),
                    min_width: zpx(220.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(zpx(4.0)),
                    border: UiRect::all(zpx(1.0)),
                    border_radius: BorderRadius::all(zpx(6.0)),
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
            tf(fonts, z(11.0)),
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
            LegacyButton,
            press,
            Node {
                padding: UiRect::axes(zpx(8.0), zpx(3.0)),
                border_radius: BorderRadius::all(zpx(4.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            Feel::new(Color::NONE),
            children![(
                Text::new(label.to_string()),
                tf(fonts, z(11.5)),
                TextColor(inks.ink),
                Pickable::IGNORE,
            )],
        ))
        .id()
}

/// Fills the list: the Log tab from the book (only what arrived, when only
/// that changed), a chat tab from the machine, All by merging both in
/// arrival order.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // one part, top to bottom
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
        if key == drawn.list_key && log_len >= drawn.log_lines {
            if log_len == drawn.log_lines {
                return;
            }
            let lookup = logpanel::LogTexts(texts);
            let wording = logpanel::wording(duel, lang, &lookup);
            let mut run = drawn.run;
            for (order, line) in
                (drawn.log_lines..).zip(duel.log.lines_since(drawn.log_lines, &wording))
            {
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
                commands.entity(row).insert((
                    super::menu::Copyable::new(order, log_copy(&line)),
                    Arrive::default(),
                ));
                commands.entity(list).add_child(row);
            }
            if let Ok(mut follow) = lists.get_mut(list)
                && !follow.stuck
            {
                follow.unseen = true;
            }
            drawn.log_lines = log_len;
            drawn.run = run;
            return;
        }
        drawn.list_key = key;
        drawn.shown_tab = Some(tab);
        drawn.chat_lines.clear();
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
        for (order, line) in lines.iter().enumerate() {
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
            commands
                .entity(row)
                .insert(super::menu::Copyable::new(order, log_copy(line)));
            commands.entity(list).add_child(row);
        }
        drawn.log_lines = log_len;
        drawn.run = run;
        if let Ok(mut follow) = lists.get_mut(list) {
            follow.stuck = true;
        }
        return;
    }
    // A chat tab, or All: a line more on the same list is appended, and
    // anything else draws it whole.
    let mut chat = machine.lines_where(&filter, CHAT_ROWS);
    // A line the machine took after this frame's stamps (the socket is read
    // later in the frame) waits a frame for its own: drawn unstamped it
    // would sort above everything, as if it were the oldest.
    chat.retain(|(_, line)| ui.arrivals.chat.contains_key(&line.key));
    let signature: Vec<(String, String, u8, bool)> = chat
        .iter()
        .map(|(c, l)| (c.to_string(), l.key.id.clone(), l.state as u8, l.hidden))
        .collect();
    let with_log = tab == ChatTab::All && filter.log && duel.is_some();
    let marker = ui
        .marker
        .as_ref()
        .filter(|(t, _)| *t == tab)
        .map(|(_, k)| k);
    let base = key_of(&(
        tab,
        format!("{filter:?}"),
        if with_log { log_len } else { 0 },
        log_identity,
        lang as u8,
        shown_group(machine, ui),
        machine.table().map(|t| (t.game.clone(), t.playing)),
        marker,
    ));
    let key = key_of(&(base, &signature));
    if key == drawn.list_key {
        return;
    }
    let same_list = drawn.shown_tab == Some(tab);
    let newest = drawn.newest;
    let stamp_of = |line: &baylee_client_core::social::ChatLine| {
        ui.arrivals.chat.get(&line.key).copied().unwrap_or(0)
    };
    let style = RowStyle {
        prefix: tab == ChatTab::All,
        timestamps: filter.timestamps,
    };
    let appended = same_list
        && !with_log
        && base == drawn.chat_base
        && !drawn.chat_lines.is_empty()
        && signature.len() > drawn.chat_lines.len()
        && signature.starts_with(&drawn.chat_lines)
        && chat[drawn.chat_lines.len()..]
            .iter()
            .all(|(_, line)| stamp_of(line) > newest);
    if appended {
        for (order, (conv, line)) in chat.iter().enumerate().skip(drawn.chat_lines.len()) {
            let swatch = swatch_of(duel, line.from_tag());
            let row = rows::spawn_row(
                commands, fonts, lang, machine, conv, line, &inks, style, swatch,
            );
            commands.entity(row).insert((
                super::menu::Copyable::new(order, rows::plain_text(machine, conv, line)),
                Arrive::default(),
            ));
            commands.entity(list).add_child(row);
            drawn.newest = drawn.newest.max(stamp_of(line));
        }
        drawn.list_key = key;
        drawn.chat_lines = signature;
        if let Ok(mut follow) = lists.get_mut(list) {
            if ui.follow || follow.stuck {
                follow.stuck = true;
            } else {
                follow.unseen = true;
            }
        }
        return;
    }
    let first_draw = drawn.list_key == 0;
    drawn.list_key = key;
    drawn.chat_base = base;
    drawn.chat_lines = signature;
    drawn.log_lines = 0;
    drawn.shown_tab = Some(tab);
    commands.entity(list).despawn_children();
    let mut entries: Vec<(u64, u8, usize, Entity)> = Vec::new();
    let mut divider_before = None;
    for (i, (conv, line)) in chat.iter().enumerate() {
        let swatch = swatch_of(duel, line.from_tag());
        let row = rows::spawn_row(
            commands, fonts, lang, machine, conv, line, &inks, style, swatch,
        );
        let stamp = stamp_of(line);
        commands.entity(row).insert(super::menu::Copyable::new(
            0,
            rows::plain_text(machine, conv, line),
        ));
        if marker == Some(&line.key) {
            divider_before = Some(row);
        }
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
            commands
                .entity(row)
                .insert(super::menu::Copyable::new(0, log_copy(line)));
            entries.push((ui.arrivals.log_stamp(i), 0, i, row));
        }
    }
    entries.sort_by_key(|(stamp, kind, index, _)| (*stamp, *kind, *index));
    if entries.is_empty() {
        let sentence = empty_sentence(machine, ui, tab, &filter, lang);
        let empty = empty_line(commands, fonts, &sentence, &inks);
        commands.entity(list).add_child(empty);
    }
    // Rows newer than the list last showed arrive; a tab opened, or the
    // first drawing, shows what it has standing still.
    for (stamp, _, _, row) in &entries {
        if same_list && !first_draw && *stamp > newest {
            commands.entity(*row).insert(Arrive::default());
        }
    }
    drawn.newest = entries.iter().map(|(stamp, ..)| *stamp).max().unwrap_or(0);
    let mut kids: Vec<Entity> = entries.into_iter().map(|(_, _, _, e)| e).collect();
    for (order, kid) in kids.iter().enumerate() {
        commands
            .entity(*kid)
            .queue(move |mut entity: EntityWorldMut| {
                if let Some(mut copy) = entity.get_mut::<super::menu::Copyable>() {
                    copy.order = order;
                }
            });
    }
    if let Some(at) = divider_before.and_then(|row| kids.iter().position(|k| *k == row))
        && at > 0
    {
        kids.insert(at, unread_divider(commands, fonts, lang, &inks));
    }
    commands.entity(list).add_children(&kids);
    if let Ok(mut follow) = lists.get_mut(list) {
        if first_draw || ui.follow || follow.stuck {
            follow.stuck = true;
        } else {
            follow.unseen = true;
        }
    }
}

/// The "New" divider (`ChatPhrase::UnreadMarker`): a rule each side of the
/// word, in the accent, above the first line that was unread.
fn unread_divider(commands: &mut Commands, fonts: &UiFonts, lang: Lang, inks: &Inks) -> Entity {
    let rule = |commands: &mut Commands| {
        commands
            .spawn((
                Node {
                    flex_grow: 1.0,
                    height: Val::Px(1.0),
                    ..default()
                },
                BackgroundColor(inks.accent.with_alpha(0.55)),
                Pickable::IGNORE,
            ))
            .id()
    };
    let left = rule(commands);
    let right = rule(commands);
    let word = commands
        .spawn((
            Text::new(ChatPhrase::UnreadMarker.text(lang)),
            tf_bold(fonts, z(inks.pt - 2.0)),
            TextColor(inks.accent),
            Pickable::IGNORE,
        ))
        .id();
    commands
        .spawn((
            UnreadDivider,
            Node {
                align_items: AlignItems::Center,
                column_gap: zpx(8.0),
                margin: UiRect::vertical(zpx(4.0)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .add_children(&[left, word, right])
        .id()
}

/// The divider [`unread_divider`] draws.
#[derive(Component)]
pub(crate) struct UnreadDivider;

/// A row arriving: it rises into its place over [`ARRIVE_SECS`], on the
/// list's transform only, so the scroll that follows the end stays exact.
#[derive(Component, Default)]
pub(crate) struct Arrive(pub(crate) f32);

/// How long a row takes to arrive.
pub(crate) const ARRIVE_SECS: f32 = 0.18;

/// How far below its place a row starts, in logical pixels.
const ARRIVE_FROM: f32 = 10.0;

/// Moves each arriving row a frame on and lets it go at its place
/// (`reduce_motion`: there at once).
pub(crate) fn arrive(
    mut commands: Commands,
    time: Res<Time>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut rows: Query<(Entity, &mut Arrive, &mut UiTransform)>,
) {
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    for (row, mut arrive, mut transform) in &mut rows {
        arrive.0 = crate::hud::motion::step(arrive.0, ARRIVE_SECS, time.delta_secs(), still);
        if arrive.0 >= 1.0 {
            transform.translation = Val2::ZERO;
            commands.entity(row).remove::<Arrive>();
            continue;
        }
        // Ease-out cubic: most of the way at once, settling.
        let left = (1.0 - arrive.0).powi(3);
        transform.translation = Val2::new(Val::ZERO, Val::Px(ARRIVE_FROM * left));
    }
}

/// Notes, as a tab comes into view (or the window back into focus), the
/// first line it had not been read to, before [`mark_read`] reads it: the
/// "New" divider stands above it until the tab goes (§3.3).
fn note_the_marker(
    host: Res<SocialHost>,
    mut ui: ResMut<ChatUi>,
    windows: Query<&Window>,
    mut shown: Local<Option<(ChatTab, Option<String>)>>,
    mut was_focused: Local<bool>,
) {
    let focused = windows.iter().any(|w| w.focused);
    let regained = focused && !*was_focused;
    *was_focused = focused;
    let now = (ui.open && !ui.minimised && ui.tab != ChatTab::Log)
        .then(|| (ui.tab, ui.shown_group.clone()));
    if *shown == now && !regained {
        return;
    }
    let came = *shown != now;
    shown.clone_from(&now);
    let Some((tab, _)) = now else {
        if ui.marker.is_some() {
            ui.marker = None;
        }
        return;
    };
    let found = host.machine().and_then(|m| first_unread(m, &ui, tab));
    // Focus coming back adds a divider for what arrived meanwhile; a tab
    // that came into view keeps the one `select_tab` noted before the
    // machine read it, and drops one of another tab's.
    if found.is_some() {
        if ui.marker != found {
            ui.marker = found;
        }
    } else if came && ui.marker.as_ref().is_some_and(|(t, _)| *t != tab) {
        ui.marker = None;
    }
}

/// A log line as copy takes it (§6): `HH:MM text`, a heading as its words.
fn log_copy(line: &baylee_client_core::gamelog::LogLine) -> String {
    match logpanel::local_clock(line) {
        Some(clock) if !line.header => format!("{clock} {}", line.text),
        _ => line.text.clone(),
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
            tf_italic(fonts, z(inks.pt)),
            TextColor(inks.soft),
            Node {
                margin: UiRect::top(zpx(8.0)),
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
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // one part, top to bottom
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
            LegacyButton,
            ChatPress::Chip,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: zpx(4.0),
                padding: UiRect::axes(zpx(8.0), zpx(2.0)),
                height: zpx(22.0),
                border_radius: BorderRadius::all(zpx(4.0)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(chip_ink.with_alpha(0.15)),
            Feel::new(chip_ink.with_alpha(0.15)),
            children![
                (
                    Text::new(chip_text),
                    tf_bold(fonts, z(11.5)),
                    TextColor(chip_ink),
                    Pickable::IGNORE,
                ),
                (
                    Text::new(glyph::CHEVRON_DOWN.to_string()),
                    icon_tf(fonts, z(7.0)),
                    TextColor(chip_ink),
                    Pickable::IGNORE,
                ),
            ],
        ))
        .id();
    let text = c.text();
    let field = commands
        .spawn((
            LegacyButton,
            ChatPress::Field,
            Text::default(),
            tf(fonts, z(inks.pt)),
            TextColor(inks.ink),
            Node {
                flex_grow: 1.0,
                min_width: zpx(0.0),
                padding: UiRect::axes(zpx(6.0), zpx(4.0)),
                border: UiRect::all(zpx(1.0)),
                border_radius: BorderRadius::all(zpx(4.0)),
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
                tf_italic(fonts, z(inks.pt)),
                TextColor(inks.soft),
            ))
            .id();
        commands.entity(field).add_child(span);
    } else {
        let buffer = c.buffer();
        let caret = buffer.cursor().min(text.len());
        let selection = buffer
            .selection()
            .filter(|r| focused && r.start < r.end && r.end <= text.len());
        // The `[Name]` links the line would carry, in bytes: drawn as links
        // and previewed as the history's are (§12.3).
        let byte = |chars: usize| {
            text.char_indices()
                .nth(chars)
                .map_or(text.len(), |(b, _)| b)
        };
        let links: Vec<(std::ops::Range<usize>, u32, u8)> = c
            .body_of(text, false)
            .links
            .iter()
            .map(|l| (byte(l.at[0])..byte(l.at[1]), l.card, l.face))
            .collect();
        let mut cuts: Vec<usize> = vec![0, text.len()];
        if focused && selection.is_none() {
            cuts.push(caret);
        }
        if let Some(range) = &selection {
            cuts.extend([range.start, range.end]);
        }
        for (range, ..) in &links {
            cuts.extend([range.start, range.end]);
        }
        cuts.sort_unstable();
        cuts.dedup();
        let link_font = TextFont {
            font: bevy::text::FontSource::Handle(fonts.medium.clone()),
            ..tf(fonts, z(inks.pt))
        };
        for pair in cuts.windows(2) {
            let (from, to) = (pair[0], pair[1]);
            if focused && selection.is_none() && from == caret {
                let bar = commands
                    .spawn((
                        TextSpan::new("|"),
                        tf(fonts, z(inks.pt)),
                        TextColor(inks.accent),
                    ))
                    .id();
                commands.entity(field).add_child(bar);
            }
            if from >= to {
                continue;
            }
            let link = links.iter().find(|(r, ..)| r.start <= from && to <= r.end);
            let chosen = selection
                .as_ref()
                .is_some_and(|r| r.start <= from && to <= r.end);
            let (font, colour) = match (chosen, link) {
                (true, _) => (tf(fonts, z(inks.pt)), inks.accent),
                (false, Some(_)) => (link_font.clone(), inks.link),
                (false, None) => (tf(fonts, z(inks.pt)), inks.ink),
            };
            let span = commands
                .spawn((TextSpan::new(&text[from..to]), font, TextColor(colour)))
                .id();
            if let Some((_, card, face)) = link {
                commands.entity(span).insert(rows::ChatCardLink {
                    card: *card,
                    face: *face,
                });
            }
            commands.entity(field).add_child(span);
        }
        if focused && selection.is_none() && caret == text.len() {
            let bar = commands
                .spawn((
                    TextSpan::new("|"),
                    tf(fonts, z(inks.pt)),
                    TextColor(inks.accent),
                ))
                .id();
            commands.entity(field).add_child(bar);
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
            tf(fonts, z(10.5)),
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
            LegacyButton,
            ChatPress::Send,
            Node {
                width: zpx(26.0),
                height: zpx(24.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(zpx(4.0)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(Color::NONE),
            Feel::new(Color::NONE),
            children![(
                Text::new(glyph::SEND.to_string()),
                icon_tf(fonts, z(10.0)),
                TextColor(inks.soft),
                Pickable::IGNORE,
            )],
        ))
        .id();
    let row = commands
        .spawn((Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: zpx(6.0),
            padding: UiRect::axes(zpx(6.0), zpx(5.0)),
            min_height: zpx(36.0),
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
                    bottom: zpx(40.0),
                    left: zpx(60.0),
                    width: zpx(320.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(zpx(4.0)),
                    border: UiRect::all(zpx(1.0)),
                    border_radius: BorderRadius::all(zpx(6.0)),
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
                    LegacyButton,
                    ChatPress::Suggestion(i),
                    Node {
                        padding: UiRect::axes(zpx(8.0), zpx(3.0)),
                        border_radius: BorderRadius::all(zpx(3.0)),
                        ..default()
                    },
                    BackgroundColor(if lit { inks.lit } else { Color::NONE }),
                    children![(
                        Text::new(format!("{}{label}", if lit { "\u{25b8} " } else { "  " })),
                        tf(fonts, z(12.0)),
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

/// In the lobby a chat card link previews as a pool row does (§12.3): the
/// lobby's preview reads the [`crate::lobby::HoverCard`] it carries, the
/// pool's printing when the pool has the card, else its text face.
fn lobby_card_links(
    mut commands: Commands,
    links: Query<(Entity, &rows::ChatCardLink), Added<rows::ChatCardLink>>,
    lobby: Option<Res<crate::lobby::LobbyState>>,
) {
    for (entity, link) in &links {
        let pool = lobby.as_ref().map(|l| l.lobby.builder().pool());
        let card = pool
            .and_then(|pool| pool.iter().find(|c| c.index == link.card))
            .map_or_else(
                || crate::lobby::hover_of_index(link.card),
                crate::lobby::hover_of_card,
            );
        commands.entity(entity).insert(card);
    }
}

/// Hovering a card link in a chat line at the table opens the table's own
/// preview, when the game knows a printing of that card (§12.3).
fn hover_chat_links(
    mut overs: MessageReader<bevy::picking::events::PointerOver>,
    mut outs: MessageReader<bevy::picking::events::PointerOut>,
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
                at: over.pointer.position,
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

#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // the peek's lines, their age and their fade
fn peek(
    mut commands: Commands,
    time: Res<Time>,
    host: Res<SocialHost>,
    mut ui: ResMut<ChatUi>,
    duel: Option<Res<Duel>>,
    fonts: Res<UiFonts>,
    roots: Query<(Entity, Option<&Children>), With<PeekRoot>>,
    mut texts: Query<&mut TextColor>,
    mut seen: Local<Option<(u64, u64)>>,
    mut drawn: Local<usize>,
    zoom: Res<super::zoom::Zoom>,
    phase: Option<Res<State<DuelPhase>>>,
) {
    let _zoom = super::zoom::Scope::new(zoom.0);
    // Only at the table: the lobby has its pill and toasts, and the lines a
    // session loads from disk as it signs in are not arriving.
    let at_table = phase.is_some_and(|p| *p.get() != DuelPhase::Closed);
    let (Some(machine), Some(duel)) = (host.machine().filter(|_| at_table), duel) else {
        *seen = None;
        if !ui.peek.is_empty() {
            ui.peek.clear();
        }
        for (root, _) in &roots {
            commands.entity(root).despawn();
        }
        *drawn = 0;
        return;
    };
    // The newest chat line (never a log line) seen so far, by arrival: a
    // line that arrives past it while the panel is shut peeks.
    if seen.is_none_or(|(revision, _)| revision != machine.revision()) {
        let filter = TabFilter {
            log: false,
            ..TabFilter::standard(ChatTab::All)
        };
        let fresh: Vec<(u64, String)> = machine
            .lines_where(&filter, 8)
            .into_iter()
            .filter(|(c, l)| {
                matches!(
                    &l.kind,
                    baylee_client_core::social::LineKind::Message { mine: false, .. }
                ) && !l.hidden
                    && !machine.conversation(c).is_some_and(|c| c.muted)
            })
            .map(|(c, l)| {
                let stamp = ui.arrivals.chat.get(&l.key).copied().unwrap_or(0);
                let text = rows::plain_text(machine, c, l);
                // Without its time: the peek is now.
                let text = text
                    .split_once(' ')
                    .map_or(text.clone(), |(_, t)| t.trim().to_string());
                (stamp, text)
            })
            .collect();
        let top = fresh.iter().map(|(s, _)| *s).max().unwrap_or(0);
        if let Some((_, before)) = *seen
            && !duel.log_open
        {
            for (stamp, text) in fresh.iter().filter(|(s, _)| *s > before) {
                let _ = stamp;
                ui.peek.push((text.clone(), 0.0));
            }
            while ui.peek.len() > 4 {
                ui.peek.remove(0);
            }
        }
        let before = seen.map_or(0, |(_, b)| b);
        *seen = Some((machine.revision(), top.max(before)));
    }
    if duel.log_open && !ui.peek.is_empty() {
        ui.peek.clear();
    }
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
                    right: zpx(crate::hud::EDGE),
                    bottom: logpanel::root_node().bottom,
                    max_width: zpx(logpanel::LOG_W_MAX),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexEnd,
                    row_gap: zpx(2.0),
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
                    tf(&fonts, z(12.5)),
                    TextColor(palette::DIALOG_INK),
                    TextShadow::default(),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(root).add_child(line);
        }
    }
    // The last 0.6 s fade; nothing writes at rest.
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
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // one part, top to bottom
fn filter_dialog(
    mut commands: Commands,
    host: Res<SocialHost>,
    ui: Res<ChatUi>,
    fonts: Res<UiFonts>,
    settings: Option<Res<crate::settings::ClientSettings>>,
    phase: Option<Res<State<DuelPhase>>>,
    roots: Query<Entity, With<FilterDialog>>,
    mut drawn: Local<Option<(ChatTab, String)>>,
    zoom: Res<super::zoom::Zoom>,
) {
    let _zoom = super::zoom::Scope::new(zoom.0);
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
                right: zpx(40.0),
                bottom: zpx(if at_table { 300.0 } else { 120.0 }),
                width: zpx(340.0),
                flex_direction: FlexDirection::Column,
                row_gap: zpx(6.0),
                padding: UiRect::all(zpx(14.0)),
                border: UiRect::all(zpx(1.0)),
                border_radius: BorderRadius::all(zpx(8.0)),
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
            tf_bold(&fonts, z(13.0)),
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
                LegacyButton,
                ChatPress::Filter(kind),
                Node {
                    padding: UiRect::axes(zpx(4.0), zpx(3.0)),
                    border_radius: BorderRadius::all(zpx(4.0)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                Feel::new(Color::NONE),
                children![(
                    Text::new(format!("{mark}  {}", phrase.text(lang))),
                    tf(&fonts, z(12.5)),
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
            tf_italic(&fonts, z(11.0)),
            TextColor(inks.soft),
            Pickable::IGNORE,
        ))
        .id();
    let buttons = commands
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::FlexEnd,
            column_gap: zpx(8.0),
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

/// The lobby's list follows its end as the table's does: the log's own
/// follower, under a name of its own (one system may stand in a schedule
/// once, and the table's is ordered against).
#[allow(clippy::type_complexity)] // the log follower's own queries
fn follow_in_the_lobby(
    lists: Query<(&mut ScrollPosition, &ComputedNode, &mut LogFollow), With<LogList>>,
    pills: Query<&mut Node, With<logpanel::LogNewBelow>>,
    words: Query<(&mut Node, &logpanel::LogPillWords), Without<logpanel::LogNewBelow>>,
) {
    crate::hud::follow_the_log(lists, pills, words);
}

/// The wheel over the chat's lists in the lobby (the table's `hud::scrolls`
/// takes it there): the list under the pointer scrolls, by lines.
fn wheel_in_the_lobby(
    mut wheels: MessageReader<bevy::picking::events::PointerScroll>,
    parents: Query<&ChildOf>,
    mut lists: Query<(&mut ScrollPosition, &ComputedNode), With<crate::hud::Scrolls>>,
) {
    for wheel in wheels.read() {
        let travel = match wheel.unit {
            bevy::input::mouse::MouseScrollUnit::Line => wheel.y * 24.0,
            bevy::input::mouse::MouseScrollUnit::Pixel => wheel.y,
        };
        let mut current = Some(wheel.entity);
        for _ in 0..10 {
            let Some(entity) = current else { break };
            if let Ok((mut at, node)) = lists.get_mut(entity) {
                let scale = node.inverse_scale_factor();
                let end = (node.content_size().y - node.size().y).max(0.0) * scale;
                let y = (at.y - travel).clamp(0.0, end);
                if (at.y - y).abs() > f32::EPSILON {
                    at.y = y;
                }
                break;
            }
            current = parents.get(entity).ok().map(ChildOf::parent);
        }
    }
}
