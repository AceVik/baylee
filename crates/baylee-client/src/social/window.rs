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
use crate::ui_button::{Clicks, UiButton};
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
use crate::hud::logpanel::{self, LogFollow, LogList, PhaseRun};
use crate::hud::{UiFonts, glyph, icon_tf, palette, tf, tf_bold, tf_italic};
use crate::{ChatDoor, Duel, DuelPhase};
use composer::composer_parts;
pub(crate) use filter::FilterDialog;
use filter::filter_dialog;
pub(crate) use head::count_pill;
use head::{group_bar, head_parts};
// What the chat's tests read of a line's arrival.
#[cfg(test)]
pub(crate) use list::{ARRIVE_SECS, Arrive, UnreadDivider};
use list::{arrive, fill_list, note_the_marker};
use peek::peek;
use resize::{LobbyChatSizing, lobby_panel_size};

/// The tab row's height in the chat window.
const LOBBY_ROW_H: f32 = 34.0;
/// The Group tab's bar.
const SUB_H: f32 = 24.0;
/// A head button.
const HEAD_BTN: f32 = 22.0;
/// The lobby panel's width.
const LOBBY_W: f32 = 560.0;
/// How many chat lines a tab draws (older ones stay in the machine).
const CHAT_ROWS: usize = 200;
/// How many log lines the All tab merges in.
const ALL_LOG_ROWS: usize = 150;
/// The lobby panel's place on the global ladder: over the lobby, under the
/// report form (1000).
pub(crate) const G_PANEL: i32 = 900;

/// A chat surface's place in the stacking order, as its offset over
/// [`G_PANEL`]: at the table over the table's own bands, in the lobby
/// between the page and the shell's popovers (`shellkit::tokens::z`), so the
/// account menu, a modal and a notice stand over the docked chat (#RC:
/// the account menu opened under the friends list and the chat window
/// hid "Open table").
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Stack(pub(crate) i32);

/// The global index and its [`Stack`] for a surface `offset` over
/// [`G_PANEL`] at the table.
pub(crate) fn stacked(offset: i32) -> (GlobalZIndex, Stack) {
    (GlobalZIndex(G_PANEL + offset), Stack(offset))
}

/// The quiet elevation shared by the lobby window and the table's panel.
fn elevation() -> BoxShadow {
    BoxShadow::new(
        Color::srgba(0.0, 0.0, 0.0, 0.42),
        zpx(0.0),
        zpx(8.0),
        zpx(0.0),
        zpx(22.0),
    )
}

/// Where a surface `offset` over [`G_PANEL`] at the table stands in the
/// lobby: the window and the drawer just over the page, the chat's own
/// menus under the shell's popovers, its sheets with the shell's sheets and
/// its toasts with the shell's toasts.
pub(crate) fn lobby_z(offset: i32) -> i32 {
    use crate::shellkit::tokens::z;
    match offset {
        0 => z::PAGE + 2,
        1 => z::PAGE + 3,
        40 => z::TOAST,
        50 => z::SHEET + 1,
        menu => (z::PAGE + 5 + menu / 15).min(z::POPOVER - 1),
    }
}

/// Moves every chat surface to its band for where the player is, before
/// the stacking is worked out.
pub(crate) fn restack(
    phase: Option<Res<State<DuelPhase>>>,
    mut surfaces: Query<(&Stack, &mut GlobalZIndex)>,
) {
    let at_table = phase.is_some_and(|p| *p.get() != DuelPhase::Closed);
    for (stack, mut z) in &mut surfaces {
        let want = if at_table {
            G_PANEL + stack.0
        } else {
            lobby_z(stack.0)
        };
        if z.0 != want {
            z.0 = want;
        }
    }
}

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
    Global,
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
pub enum Part {
    Head,
    Sub,
    Note,
    List,
    Composer,
}

/// The lobby's panel root.
#[derive(Component)]
pub(crate) struct LobbyChat;

/// The list's track, hidden until the list actually overflows.
#[derive(Component)]
pub(crate) struct ChatScrollbar;

/// Top or bottom scroll overflow edge indicator on the chat message list.
#[derive(Component, Clone, Copy)]
pub(crate) struct ChatScrollEdge {
    pub(crate) top: bool,
}

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
    /// The Log tab's bounded window of mounted rows.
    rows: logpanel::virtual_list::VirtualLog,
    /// A chat tab's drawn lines, in the order asked, and everything else
    /// the list was drawn from: a line more on the same list is appended.
    chat_lines: Vec<(String, String, u8, bool)>,
    chat_rows: Vec<Entity>,
    chat_base: u64,
    /// The tab the list shows and the newest arrival stamp drawn on it: a
    /// row newer than that arrives with a movement.
    shown_tab: Option<ChatTab>,
    newest: u64,
}

/// The drawn chat window.
#[derive(Resource, Default)]
pub(crate) struct Windows {
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
        let _ = self;
        LOBBY_INKS
    }

    fn row_h(self) -> f32 {
        let _ = self;
        LOBBY_ROW_H
    }
}

/// Installs the window's systems.
pub(crate) fn install(app: &mut App) {
    crate::ui_button::install(app);
    app.add_systems(PostUpdate, restack.before(bevy::ui::UiSystems::Prepare));
    app.add_systems(
        PostUpdate,
        resize::cursor.after(crate::shellkit::pointer::follow_the_pointer),
    );
    app.init_resource::<Windows>()
        .init_resource::<LobbyChatSizing>()
        .init_resource::<preview::ChatHover>()
        .add_systems(
            Update,
            (
                doors,
                keep_the_tab,
                stamp_arrivals,
                note_the_marker,
                mark_read,
                (presses, composer::composer_cursor_drag).chain(),
                panel_follows_the_window,
                pop_the_lobby_chat,
                arrive,
                resize::resize,
                (
                    sync_lobby_window
                        .after(crate::hud::sync_log)
                        .before(crate::hud::follow_the_log),
                    age_encryption_note,
                    show_scrollbars_only_on_overflow,
                )
                    .chain(),
                follow_in_the_lobby
                    .run_if(not(in_state(DuelPhase::Playing)))
                    .run_if(not(in_state(DuelPhase::Finished))),
                wheel_in_the_lobby
                    .run_if(not(in_state(DuelPhase::Playing)))
                    .run_if(not(in_state(DuelPhase::Finished)))
                    .run_if(resource_exists::<Messages<bevy::picking::events::PointerScroll>>),
                (
                    preview::hover,
                    preview::sync,
                    preview::scryfall_links
                        .run_if(resource_exists::<Messages<bevy::picking::events::PointerClick>>),
                )
                    .chain(),
                lobby_card_links
                    .run_if(not(in_state(DuelPhase::Playing)))
                    .run_if(not(in_state(DuelPhase::Finished))),
                peek,
                filter_dialog,
            )
                .chain()
                .run_if(resource_exists::<UiFonts>),
        );
}

/// The tabs drawn, in order (§3.1): Lobby (everyone online); Table and Team
/// while seated; Private; Group for a registered account.
pub(crate) fn drawn_tabs(machine: &Social, _game_open: bool) -> Vec<ChatTab> {
    let table = machine.table();
    let seated = table.is_some_and(|t| t.table_conv().is_some());
    let team = table.is_some_and(|t| t.team_conv().is_some());
    ChatTab::ALL
        .into_iter()
        .filter(|tab| match tab {
            ChatTab::All | ChatTab::Log => false,
            ChatTab::Global | ChatTab::Private => true,
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
        ChatTab::Global => ChatPhrase::TabGlobal.text(lang),
        ChatTab::Table => ChatPhrase::TabTable.text(lang),
        ChatTab::Team => ChatPhrase::TabTeam.text(lang),
        ChatTab::Private => ChatPhrase::TabPrivate.text(lang),
        ChatTab::Group => ChatPhrase::TabGroup.text(lang),
    }
}

/// The conversation a chat tab writes to and reads (§3.1's "Composer
/// target"): the global lobby's, the table's, the team's, the last whisper
/// partner's, the shown group's.
pub(crate) fn tab_conv(machine: &Social, ui: &ChatUi, tab: ChatTab) -> Option<ConvId> {
    match tab {
        ChatTab::Global => Some(Conv::Global),
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
    host.act(Intent::Panel(conv));
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
        duel.log_open = false;
        duel.ai_log_open = false;
        duel.game_menu = false;
    }
    select_tab(host, ui, tab);
}

/// The doors at the table (§4.1): the log's scroll and `L` open/close the
/// standalone game log panel (and shut the chat if it stood open); the
/// bubble opens the chat window on the last chat tab (or closes it).
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
    if active && duel.game_menu && ui.open {
        ui.open = false;
        ui.typing = false;
        host.act(Intent::Panel(None));
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
            // `duel.log_open` was toggled by the input system for the
            // standalone game log panel; if it opened, close the chat window
            // so the two right-hand panels never overlap.
            if duel.log_open && ui.open {
                ui.open = false;
                ui.typing = false;
                host.act(Intent::Panel(None));
            }
        }
        ChatDoor::Chat => {
            if ui.open && !ui.minimised {
                ui.open = false;
                ui.typing = false;
                host.act(Intent::Panel(None));
            } else {
                open_on_chat(&mut host, ui, Some(&mut duel));
                ui.typing = true;
                ui.field = Field::Composer;
            }
        }
    }
}

/// Keeps the shown tab one that is drawn (a table ended, a guest).
fn keep_the_tab(
    host: Res<SocialHost>,
    mut ui: ResMut<ChatUi>,
    phase: Option<Res<State<DuelPhase>>>,
) {
    let Some(machine) = host.machine() else {
        return;
    };
    let playing = phase.is_some_and(|p| *p.get() != DuelPhase::Closed);
    let entered_table = !ui.at_table && playing;
    if ui.at_table != playing {
        ui.at_table = playing;
    }
    let tabs = drawn_tabs(machine, playing);
    if entered_table && tabs.contains(&ChatTab::Table) && ui.tab == ChatTab::Global {
        ui.tab = ChatTab::Table;
    }
    if !tabs.contains(&ui.tab) {
        let next = if playing && tabs.contains(&ChatTab::Table) {
            ChatTab::Table
        } else {
            ChatTab::Global
        };
        ui.tab = next;
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

/// Whether the client window is focused (or driven passively by `dev-control`).
pub(super) fn window_focused(windows: &Query<&Window>) -> bool {
    if windows.iter().any(|w| w.focused) {
        return true;
    }
    #[cfg(all(feature = "dev-control", not(target_arch = "wasm32")))]
    {
        crate::devctl::passive()
    }
    #[cfg(not(all(feature = "dev-control", not(target_arch = "wasm32"))))]
    {
        false
    }
}

/// Reading marks read (§3.3): while the panel shows a tab, the
/// conversations its filter admits have nothing unread.
fn mark_read(mut host: ResMut<SocialHost>, ui: Res<ChatUi>, windows: Query<&Window>) {
    if !ui.open || ui.minimised || ui.tab == ChatTab::Log {
        return;
    }
    if !window_focused(&windows) {
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
    mut was: Local<(bool, Option<ConvId>)>,
) {
    let shown = ui.open && !ui.minimised;
    let Some(machine) = host.machine() else {
        return;
    };
    let conv = shown.then(|| tab_conv(machine, &ui, ui.tab)).flatten();
    if was.0 == shown && was.1 == conv {
        return;
    }
    let became_shown = shown && !was.0;
    *was = (shown, conv.clone());
    if !shown {
        host.act(Intent::Panel(None));
        return;
    }
    if became_shown {
        // Shown again: the divider is noted before the machine reads the tab.
        let marker = first_unread(machine, &ui, ui.tab);
        if marker.is_some() {
            ui.marker = marker;
        }
    }
    host.act(Intent::Panel(conv));
}

/// The window's presses.
#[allow(clippy::too_many_lines)] // one arm per control, read top to bottom
fn presses(
    mut clicks: Clicks<ChatPress>,
    mut host: ResMut<SocialHost>,
    mut ui: ResMut<ChatUi>,
    mut duel: Option<ResMut<Duel>>,
    mut prefs: Option<ResMut<crate::prefs::Prefs>>,
) {
    let mut clicked: Vec<ChatPress> = clicks.read();
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
                let _ = &mut duel;
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
        FilterKind::Global => filter.global = !filter.global,
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
pub(super) fn key_of<T: std::hash::Hash>(value: &T) -> u64 {
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

/// Where the lobby's chat pill stands so it clears the memorial's caption,
/// which lies along the same bottom-left corner where the window is short
/// enough: beside it, ten pixels on. `None` where the caption is not there,
/// not laid out yet, or ends well above the pill.
///
/// The pill stood at 12 pixels whatever lay there, and covered half of
/// "Baylee · Forever loved" (seen 29.10.2026 at 1280 × 800).
pub(super) fn pill_left(
    captions: &Query<
        (&ComputedNode, &UiGlobalTransform),
        With<crate::lobby::memorial::MemorialCaption>,
    >,
    window_h: f32,
) -> Option<Val> {
    /// How far above the window's bottom the caption may end and still
    /// share the pill's band.
    const BAND: f32 = 60.0;
    let (node, at) = captions.iter().next()?;
    let scale = node.inverse_scale_factor();
    let size = node.size() * scale;
    if size.x <= 0.0 {
        return None;
    }
    let centre = at.translation * scale;
    let bottom = centre.y + size.y * 0.5;
    (bottom > window_h - BAND).then_some(Val::Px(centre.x + size.x * 0.5 + 10.0))
}

/// The one unified chat window: in the lobby docked bottom-left (minimised
/// to its tab row, or hidden to the chat pill), and at the table docked on
/// the right above the bottom-right strip with a top-left resize handle.
#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::type_complexity
)]
// a Bevy system: each is what it draws from
fn sync_lobby_window(
    mut commands: Commands,
    host: Res<SocialHost>,
    ui: Res<ChatUi>,
    fonts: Res<UiFonts>,
    (settings, phase, lobby): (
        Option<Res<crate::settings::ClientSettings>>,
        Option<Res<State<DuelPhase>>>,
        Option<Res<crate::lobby::LobbyState>>,
    ),
    (duel, texts, folds): (
        Option<Res<Duel>>,
        Option<Res<crate::cardtext::CardTexts>>,
        Option<Res<logpanel::LogFolds>>,
    ),
    (windows_q, scale): (Query<&Window>, Option<Res<UiScale>>),
    mut windows: ResMut<Windows>,
    mut roots: Query<
        (Entity, &mut Node, &mut Visibility, &mut Pop),
        (With<LobbyChat>, Without<logpanel::virtual_list::LogSpacer>),
    >,
    mut lists: Query<(&mut LogFollow, &mut ScrollPosition), With<LogList>>,
    (nodes, captions): (
        Query<&ComputedNode>,
        Query<(&ComputedNode, &UiGlobalTransform), With<crate::lobby::memorial::MemorialCaption>>,
    ),
    mut spacers: Query<&mut Node, With<logpanel::virtual_list::LogSpacer>>,
    mut shape: Local<Option<LobbyShape>>,
    (zoom, sizing): (Res<super::zoom::Zoom>, Res<LobbyChatSizing>),
) {
    let _zoom = super::zoom::Scope::new(zoom.0);
    let at_table =
        phase.is_some_and(|p| matches!(*p.get(), DuelPhase::Playing | DuelPhase::Finished));
    let shell = lobby.is_some_and(|l| {
        !matches!(
            l.lobby.screen(),
            baylee_client_core::lobby::Screen::SignIn { .. }
        )
    });
    let ending = at_table && duel.as_ref().is_some_and(|d| d.ending().is_some());
    let Some(machine) = host.machine().filter(|_| (at_table || shell) && !ending) else {
        for (_, _, mut seen, _) in &mut roots {
            seen.set_if_neq(Visibility::Hidden);
        }
        return;
    };
    let frame = if at_table { Frame::Table } else { Frame::Lobby };
    let minimised = ui.minimised && !at_table;
    let lang = settings.as_ref().map_or(Lang::En, |s| Lang::of(&s.lang));
    let window = windows_q
        .iter()
        .next()
        .map_or(Vec2::new(1280.0, 720.0), |w| {
            crate::hud::scale::desktop_space(w, scale.as_deref())
        });
    let size = lobby_panel_size(
        sizing
            .resized
            .or(settings.as_deref().and_then(|s| s.chat_window)),
        window,
        zoom.0,
    );
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
                    left: zpx(12.0),
                    width: zpx(size.width),
                    flex_direction: FlexDirection::Column,
                    border_radius: BorderRadius::all(zpx(6.0)),
                    ..default()
                },
                BackgroundColor(LOBBY_INKS.ground),
                BorderColor::all(LOBBY_INKS.line),
                elevation(),
                stacked(0),
                Visibility::Visible,
                Pop::default(),
            ))
            .id()
    };
    let (unread, urgent) = machine.unread_where(&TabFilter::standard(ChatTab::All));
    // Only the pill shows the count: an open panel is not drawn again for
    // a line arriving (its list is), nor for the drawer moving it aside.
    let place_tag = u32::from(at_table);
    let now = if ui.open {
        (true, minimised, false, 0, place_tag)
    } else {
        (false, false, urgent, unread, place_tag)
    };
    if let Ok((_, mut node, mut seen, mut pop)) = roots.get_mut(root) {
        // A panel that was closed shrinks away as it was drawn, and only
        // then becomes the pill (in the lobby) or hides (at the table).
        let was_open = shape.is_some_and(|s| s.0);
        let same_place = shape.is_some_and(|s| s.4 == place_tag);
        if was_open && !ui.open && same_place {
            if !pop.closing {
                *pop = Pop::CLOSING;
            }
            if pop.t < 1.0 {
                seen.set_if_neq(Visibility::Visible);
                return;
            }
        }
        if at_table && !ui.open {
            if *shape != Some(now) {
                *shape = Some(now);
                commands.entity(root).despawn_children();
                windows.lobby = Drawn::default();
            }
            seen.set_if_neq(Visibility::Hidden);
            return;
        }
        seen.set_if_neq(Visibility::Visible);
        let grew = !at_table && shape.is_some_and(|s| !s.0 && unread > s.3);
        if *shape != Some(now) && (was_open != ui.open || !same_place || grew || shape.is_none()) {
            *pop = Pop::OPENING;
        } else if pop.closing && ui.open {
            // Opened again while it shrank: it grows back from there.
            *pop = Pop::OPENING;
        }
        let full = ui.open && !minimised;
        let want_h = if full { zpx(size.height) } else { Val::Auto };
        if node.height != want_h {
            node.height = want_h;
        }
        let want_w = if ui.open { zpx(size.width) } else { Val::Auto };
        if node.width != want_w {
            node.width = want_w;
        }
        let (left, right, bottom, radius, border) = if at_table {
            let table_node = logpanel::root_node();
            (
                Val::Auto,
                table_node.right,
                table_node.bottom,
                table_node.border_radius,
                UiRect::all(zpx(1.0)),
            )
        } else {
            let left = if ui.open {
                zpx(12.0)
            } else {
                pill_left(&captions, window.y).unwrap_or(zpx(12.0))
            };
            (
                left,
                Val::Auto,
                zpx(0.0),
                BorderRadius::all(zpx(6.0)),
                UiRect::ZERO,
            )
        };
        if node.left != left {
            node.left = left;
        }
        if node.right != right {
            node.right = right;
        }
        if node.bottom != bottom {
            node.bottom = bottom;
        }
        if node.border_radius != radius {
            node.border_radius = radius;
        }
        if node.border != border {
            node.border = border;
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
        let ids = skeleton(&mut commands, &fonts, frame, lang);
        if minimised {
            commands.entity(root).add_child(ids.head);
            for id in [ids.sub, ids.note, ids.body, ids.composer, ids.light] {
                commands.entity(id).despawn();
            }
            drawn.head = Some(ids.head);
        } else {
            let handle = resize::handle_for(&mut commands, frame);
            commands.entity(root).add_children(&ids.all());
            commands.entity(root).add_child(handle);
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
        host.vault.sync_status(),
        !host.vault.is_central(),
        &ui,
        duel.as_deref(),
        texts.as_deref(),
        frame,
        drawn,
        &mut lists,
        &nodes,
        &mut spacers,
        folds.as_deref(),
    );
}

/// What the panel's frame was drawn as: open, minimised, a whisper
/// waiting, the unread count, whether at a table.
type LobbyShape = (bool, bool, bool, u32, u32);

/// The chat window's opening and closing, on [`crate::hud::motion`]'s
/// curve: it grows from the bottom-left corner in the lobby (or bottom-right
/// at the table), and shrinks back into it.
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
    phase: Option<Res<State<DuelPhase>>>,
    mut roots: Query<(&mut Pop, &mut UiTransform)>,
) {
    use crate::hud::motion;
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    let at_table =
        phase.is_some_and(|p| matches!(*p.get(), DuelPhase::Playing | DuelPhase::Finished));
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
        transform.translation = if at_table {
            motion::from_bottom_right(scale)
        } else {
            motion::from_bottom_left(scale)
        };
    }
}

/// The lobby panel's height in a window this tall (SOCIAL §11.4: 38 %,
/// between 320 and 520).
pub(crate) fn lobby_panel_height(window_h: f32) -> f32 {
    (window_h * 0.38).clamp(320.0, 520.0)
}

/// The lobby's chat pill: in the bottom-left corner, with a numbered badge
/// dot (`1`..`99`, `99+`) when unread messages are waiting.
fn chat_pill(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    unread: u32,
    urgent: bool,
) -> Entity {
    let label = ChatPhrase::ChatTitle.text(lang).to_string();
    let border = if urgent {
        rows::WHISPER_INK
    } else {
        LOBBY_INKS.line
    };
    let pill = commands
        .spawn((
            UiButton,
            ChatPress::Restore,
            Visibility::Hidden,
            Node {
                display: Display::None,
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
        .id();
    if unread > 0 {
        let dot = count_pill(commands, fonts, unread, urgent, &LOBBY_INKS);
        commands.entity(pill).add_child(dot);
    }
    pill
}

/// The four parts of a window, spawned empty.
struct Skeleton {
    head: Entity,
    sub: Entity,
    note: Entity,
    list: Entity,
    composer: Entity,
    body: Entity,
    /// The window's light, over its parts (`glass`).
    light: Entity,
}

impl Skeleton {
    fn all(&self) -> [Entity; 6] {
        [
            self.head,
            self.sub,
            self.note,
            self.body,
            self.composer,
            self.light,
        ]
    }
}

#[allow(clippy::too_many_lines)] // one window's parts, top to bottom
fn skeleton(commands: &mut Commands, fonts: &UiFonts, frame: Frame, lang: Lang) -> Skeleton {
    let inks = frame.inks();
    let head = commands
        .spawn((
            Part::Head,
            ZIndex(1),
            Node {
                height: zpx(frame.row_h()),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Stretch,
                // Leave the resize corner clear of the header's controls:
                // top-left at a table, top-right in the lobby.
                padding: UiRect::new(
                    zpx(4.0
                        + if frame == Frame::Table {
                            resize::HANDLE
                        } else {
                            0.0
                        }),
                    zpx(3.0
                        + if frame == Frame::Lobby {
                            resize::HANDLE
                        } else {
                            0.0
                        }),
                    zpx(0.0),
                    zpx(0.0),
                ),
                border: UiRect::bottom(zpx(1.0)),
                border_radius: BorderRadius::top(zpx(6.0)),
                ..default()
            },
            BackgroundColor(inks.lit),
            BorderColor::all(inks.line),
            BoxShadow::new(
                Color::srgba(0.0, 0.0, 0.0, 0.22),
                zpx(0.0),
                zpx(3.0),
                zpx(0.0),
                zpx(6.0),
            ),
        ))
        .id();
    let sub = commands
        .spawn((
            Part::Sub,
            ZIndex(1),
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
            BoxShadow::new(
                Color::srgba(0.0, 0.0, 0.0, 0.22),
                zpx(0.0),
                zpx(3.0),
                zpx(0.0),
                zpx(6.0),
            ),
        ))
        .id();
    // The encryption's quiet line under the tabs (owner, 13.10.): a press
    // opens the sheet with the safety numbers.
    let note = commands
        .spawn((
            Part::Note,
            UiButton,
            ChatPress::Lock,
            ZIndex(1),
            Node {
                display: Display::None,
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: zpx(6.0),
                padding: UiRect::axes(zpx(logpanel::LOG_PAD), zpx(3.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            Feel::new(Color::NONE),
            BoxShadow::new(
                Color::srgba(0.0, 0.0, 0.0, 0.22),
                zpx(0.0),
                zpx(3.0),
                zpx(0.0),
                zpx(6.0),
            ),
        ))
        .id();
    let body = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                flex_grow: 1.0,
                min_height: zpx(0.0),
                padding: UiRect::new(
                    zpx(logpanel::LOG_PAD),
                    zpx(logpanel::LOG_PAD),
                    zpx(logpanel::LOG_PAD),
                    zpx(0.0),
                ),
                column_gap: zpx(4.0),
                ..default()
            },
            BackgroundColor(Color::NONE),
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
    commands.entity(track).insert(ChatScrollbar);
    let pill = logpanel::new_lines_pill(commands, fonts, lang);
    let top_edge = commands
        .spawn((
            ChatScrollEdge { top: true },
            ZIndex(3),
            Node {
                display: Display::None,
                position_type: PositionType::Absolute,
                top: zpx(0.0),
                left: zpx(0.0),
                right: zpx(0.0),
                height: zpx(5.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexStart,
                border: UiRect::top(zpx(1.5)),
                ..default()
            },
            BackgroundColor(inks.ground.with_alpha(0.92)),
            BorderColor::all(inks.accent.with_alpha(0.60)),
            BoxShadow::new(
                inks.accent.with_alpha(0.28),
                zpx(0.0),
                zpx(2.0),
                zpx(0.0),
                zpx(5.0),
            ),
            Pickable::IGNORE,
        ))
        .id();
    let bottom_edge = commands
        .spawn((
            ChatScrollEdge { top: false },
            ZIndex(3),
            Node {
                display: Display::None,
                position_type: PositionType::Absolute,
                bottom: zpx(0.0),
                left: zpx(0.0),
                right: zpx(0.0),
                height: zpx(5.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexEnd,
                border: UiRect::bottom(zpx(1.5)),
                ..default()
            },
            BackgroundColor(inks.ground.with_alpha(0.92)),
            BorderColor::all(inks.accent.with_alpha(0.60)),
            BoxShadow::new(
                inks.accent.with_alpha(0.28),
                zpx(0.0),
                zpx(-2.0),
                zpx(0.0),
                zpx(5.0),
            ),
            Pickable::IGNORE,
        ))
        .id();
    commands
        .entity(body)
        .add_children(&[list, track, pill, top_edge, bottom_edge]);
    let composer = commands
        .spawn((
            Part::Composer,
            ZIndex(1),
            Node {
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                border: UiRect::top(zpx(1.0)),
                ..default()
            },
            BackgroundColor(inks.lit),
            BorderColor::all(inks.line),
            BoxShadow::new(
                Color::srgba(0.0, 0.0, 0.0, 0.24),
                zpx(0.0),
                zpx(-3.0),
                zpx(0.0),
                zpx(6.0),
            ),
        ))
        .id();
    let light = commands
        .spawn(super::glass::layer(super::glass::Glass::Window, zpx(6.0)))
        .id();
    Skeleton {
        head,
        sub,
        note,
        list,
        composer,
        body,
        light,
    }
}

/// Fills a window's parts, each only when what it shows changed.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // the window's parts and what they show
fn fill(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    machine: &Social,
    backup: Option<super::vault_access::SyncStatus>,
    lock_in_composer: bool,
    ui: &ChatUi,
    duel: Option<&Duel>,
    texts: Option<&crate::cardtext::CardTexts>,
    frame: Frame,
    drawn: &mut Drawn,
    lists: &mut Query<(&mut LogFollow, &mut ScrollPosition), With<LogList>>,
    nodes: &Query<&ComputedNode>,
    spacers: &mut Query<&mut Node, With<logpanel::virtual_list::LogSpacer>>,
    folds: Option<&logpanel::LogFolds>,
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
        let show_note = ui.tab != ChatTab::Log
            && (backup.is_some() || conv.as_ref().is_some_and(|c| !matches!(c, Conv::Global)));
        let key = key_of(&(
            conv.as_ref().map(ToString::to_string),
            to,
            verified,
            backup,
            show_note,
            lang as u8,
        ));
        if key != drawn.note_key {
            drawn.note_key = key;
            commands.entity(note).despawn_children();
            if show_note {
                // "End-to-end encrypted · Bob#0002 · 2 devices": a whisper
                // names its partner, a group or table counts its people.
                let partner = conv
                    .as_ref()
                    .and_then(|c| c.partner(&machine.me().tag))
                    .map(|t| machine.person(t).handle);
                let mut words = match to {
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
                if let Some(backup) = backup {
                    if conv
                        .as_ref()
                        .is_none_or(|conv| matches!(conv, Conv::Global))
                    {
                        words = backup.phrase().text(lang).to_string();
                    } else {
                        words.push_str(" · ");
                        words.push_str(backup.phrase().text(lang));
                    }
                }
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
            commands, fonts, lang, machine, ui, duel, texts, frame, drawn, list, lists, nodes,
            spacers, folds,
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
        let selection = c.buffer().selection();
        let key = key_of(&(
            c.text().to_string(),
            caret,
            selection,
            format!("{:?}", c.target()),
            ui.typing && ui.field == Field::Composer,
            suggest.clone(),
            highlight,
            lang as u8,
            ui.hint.map(|(p, _)| p as u8),
            ui.tab,
            lock_in_composer,
        ));
        if key != drawn.composer_key {
            drawn.composer_key = key;
            commands.entity(composer).despawn_children();
            let kids = composer_parts(
                commands,
                fonts,
                lang,
                machine,
                ui,
                &suggest,
                highlight,
                frame,
                lock_in_composer,
            );
            commands.entity(composer).add_children(&kids);
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

/// Whether a computed list needs a vertical scrollbar.
fn overflows(node: &ComputedNode) -> bool {
    node.content_size().y > node.size().y + 0.5
}

/// Automatically dismisses the E2E encryption banner (`Part::Note`) after a
/// few seconds so it frees up vertical space for the chat history.
fn age_encryption_note(
    time: Option<Res<Time>>,
    mut notes: Query<(&Part, &mut Node)>,
    mut elapsed: Local<f32>,
) {
    let dt = time.map_or(0.0, |t| t.delta_secs()).clamp(0.0, 0.1);
    for (part, mut node) in &mut notes {
        if *part != Part::Note {
            continue;
        }
        if node.is_changed() && node.display == Display::Flex {
            *elapsed = 0.0;
            continue;
        }
        if node.display == Display::Flex {
            *elapsed += dt;
            if *elapsed >= 5.0 {
                node.display = Display::None;
            }
        }
    }
}

/// Gives the track its space only when there is somewhere to scroll to, and
/// shows the top/bottom scroll overflow indicators when more messages lie
/// above or below the viewport.
#[allow(clippy::type_complexity)]
fn show_scrollbars_only_on_overflow(
    lists: Query<(&ComputedNode, Option<&ScrollPosition>), With<LogList>>,
    children_q: Query<&Children>,
    parents: Query<&ChildOf>,
    mut tracks: Query<(Entity, &mut Node), (With<ChatScrollbar>, Without<ChatScrollEdge>)>,
    mut edges: Query<(Entity, &ChatScrollEdge, &mut Node), Without<ChatScrollbar>>,
) {
    let body_list = |entity: Entity| -> Option<(&ComputedNode, Option<&ScrollPosition>)> {
        let parent = parents.get(entity).ok()?.parent();
        if let Ok(found) = lists.get(parent) {
            return Some(found);
        }
        let kids = children_q.get(parent).ok()?;
        kids.iter().find_map(|c| lists.get(c).ok())
    };
    for (entity, mut track) in &mut tracks {
        let overflow = body_list(entity).is_some_and(|(node, _)| overflows(node));
        let display = if overflow {
            Display::Flex
        } else {
            Display::None
        };
        if track.display != display {
            track.display = display;
        }
    }
    for (entity, edge, mut node) in &mut edges {
        let show = body_list(entity).is_some_and(|(list_node, scroll)| {
            let scroll_y = scroll.map_or(0.0, |s| s.y);
            let scale = list_node.inverse_scale_factor();
            let max_scroll = (list_node.content_size().y - list_node.size().y).max(0.0) * scale;
            if max_scroll <= 1.5 {
                false
            } else if edge.top {
                scroll_y > 1.5
            } else {
                scroll_y < max_scroll - 1.5
            }
        });
        let display = if show { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
    }
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

mod composer;
mod filter;
mod head;
pub(crate) mod list;
mod peek;
pub(crate) mod preview;
mod resize;
#[cfg(test)]
mod tests;
