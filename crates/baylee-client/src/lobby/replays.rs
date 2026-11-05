//! The Replays tab (`Hub::Replays`, `.claude/b8/REPLAY-DESIGN.md` §7.2) and
//! the door from the lobby into a replay (`Screen::Replay`).
//!
//! One list, grouped by day, newest first: this computer's kept records of
//! games against the house, and the hosted games the account sat in, read
//! from the gateway's `GET /replays` a page at a time. Desktop only: the web
//! and mobile builds draw no tab.

use std::sync::Arc;

use baylee_client_core::lobby::{ReplayHandover, ReplayRef};
#[cfg(not(target_arch = "wasm32"))]
use baylee_client_core::replay::Perspectives;
use baylee_client_core::replay::library::{
    Entry, EntrySource, Filters, Library, Outcome, ReplayPage, ResultFilter, SourceFilter,
};
use baylee_client_core::replay::words::ReplayPhrase;
use baylee_client_core::replay::{Perspective, Verify};

use super::menus::{self, ShellMenu};
#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;
use crate::shellkit::controls::{self, Kit, Live, Weight};
use crate::shellkit::{px_fixed, tokens};

/// Whether this build shows the Replays tab: desktop only.
pub(crate) const SHOWN: bool = cfg!(not(any(
    target_arch = "wasm32",
    target_os = "android",
    target_os = "ios"
)));

/// The record of the game just played here, held while its replay opens
/// from the end screen.
#[derive(Clone, Debug)]
#[cfg_attr(target_arch = "wasm32", allow(dead_code))] // a browser keeps no records to open
pub(crate) struct MemoryRecord {
    /// The record, inflated.
    pub(crate) bytes: Arc<Vec<u8>>,
    /// What each seat was called at the table.
    pub(crate) names: Vec<String>,
    /// Its file among the kept records.
    pub(crate) file: String,
}

/// How far the online list has been read.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum Online {
    /// Not asked yet on this visit.
    #[default]
    Unasked,
    /// A page is on its way, from this offset.
    Asking(u64),
    /// Read.
    Read,
}

/// Sort order for the Replays list.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum ReplaysSort {
    /// Newest matches first (grouped by day).
    #[default]
    Newest,
    /// Longest duration first.
    Longest,
    /// Highest turn count first.
    MostTurns,
}

impl ReplaysSort {
    const fn next(self) -> Self {
        match self {
            Self::Newest => Self::Longest,
            Self::Longest => Self::MostTurns,
            Self::MostTurns => Self::Newest,
        }
    }

    fn label(self, lang: Lang) -> &'static str {
        match self {
            Self::Newest => Phrase::ReplaysSortNewest,
            Self::Longest => Phrase::ReplaysSortLongest,
            Self::MostTurns => Phrase::ReplaysSortMostTurns,
        }
        .text(lang)
    }
}

/// Display density for the Replays list.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum ReplaysView {
    /// Detailed match cards with player/commander pills & mana discs.
    #[default]
    Detailed,
    /// Compact single-line rows for scanning large match histories.
    Compact,
}

/// Filter by table size (1v1 Duel vs 3+ Multiplayer).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum ReplayPlayersFilter {
    #[default]
    All,
    Duel,
    Multiplayer,
}

impl ReplayPlayersFilter {
    const fn next(self) -> Self {
        match self {
            Self::All => Self::Duel,
            Self::Duel => Self::Multiplayer,
            Self::Multiplayer => Self::All,
        }
    }

    fn label(self, lang: Lang) -> &'static str {
        match self {
            Self::All => Phrase::ReplaysFilterAll,
            Self::Duel => Phrase::ReplaysFilterDuel,
            Self::Multiplayer => Phrase::ReplaysFilterMultiplayer,
        }
        .text(lang)
    }

    fn matches(self, players_len: usize) -> bool {
        match self {
            Self::All => true,
            Self::Duel => players_len <= 2,
            Self::Multiplayer => players_len >= 3,
        }
    }
}

/// The Replays tab's state, and what a replay being opened needs.
#[derive(Default)]
pub(crate) struct ReplaysUi {
    /// Every entry.
    pub(crate) library: Library,
    /// Which source and result are shown (the query is the search field's).
    pub(crate) filters: Filters,
    /// Active sort order.
    pub(crate) sort: ReplaysSort,
    /// Active display mode (Detailed cards vs Compact rows).
    pub(crate) view: ReplaysView,
    /// Active table-size filter.
    pub(crate) players_filter: ReplayPlayersFilter,
    /// Whether this visit has read the kept records.
    pub(crate) local_read: bool,
    /// How far the online list is read.
    pub(crate) online: Online,
    /// The entry being watched: what it offers and where it was played.
    pub(crate) watching: Option<Entry>,
    /// The record of the game just played, for a replay from its end.
    pub(crate) memory: Option<MemoryRecord>,
    /// A replay to open once the table now standing has closed.
    pub(crate) next: Option<ReplayHandover>,
    /// How far the archived online list is read (asked when the Archived
    /// filter is first shown on a visit).
    pub(crate) archived: Online,
    /// The consent the end sheet shows for the game just played (§18.2):
    /// the game's id and whether the player shares; ticked by default.
    pub(crate) end_share: Option<(String, bool)>,
    /// The end sheet's ways are drawn again (the consent was flipped).
    pub(crate) end_redraw: bool,
}

impl ReplaysUi {
    /// The list is read again on the next visit.
    pub(crate) fn forget_visit(&mut self) {
        self.local_read = false;
        self.online = Online::Unasked;
        self.archived = Online::Unasked;
    }
}

/// A control of the Replays tab.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ReplaysPress {
    /// Show this source.
    Source(SourceFilter),
    /// Show this result (`None`: any).
    Result(Option<Outcome>),
    /// Cycle the sort order (`Newest` -> `Longest` -> `MostTurns`).
    CycleSort,
    /// Switch between `Detailed` and `Compact` view modes.
    SetView(ReplaysView),
    /// Cycle the player-count filter (`All` -> `Duel` -> `Multiplayer`).
    CyclePlayers,
    /// Clear the replay search input.
    ClearSearch,
    /// Jump to `Hub::Play` to start a game.
    GoPlay,
    /// Watch entry `i` from its default perspective.
    Watch(usize),
    /// Watch entry `i` from this perspective.
    WatchFrom(usize, Perspective),
    /// Archive entry `i`: out of the list, under Archived (nothing is
    /// deleted: a local record moves to its archive folder, a hosted one
    /// is archived on the gateway, whose record stays).
    Archive(usize),
    /// Bring entry `i` back from the archive.
    Unarchive(usize),
    /// Flip whether the viewer shares their hand and deck in entry `i`'s
    /// replays (§18.2; a hosted game).
    Share(usize),
    /// The next page of online games.
    Older,
}

impl ReplaysPress {
    /// What a click on this control does.
    pub(super) fn handle(self, cx: super::press::Cx<'_, '_, '_, '_, '_>) {
        let state = cx.state;
        if state.menu.is_some() {
            state.menu = None;
        }
        match self {
            Self::Source(source) => {
                state.replays.filters.source = source;
                if source == SourceFilter::Archived && state.replays.archived == Online::Unasked {
                    ask_archived(state, cx.mailbox);
                }
            }
            Self::Result(result) => {
                state.replays.filters.result = result.map_or(ResultFilter::Any, ResultFilter::Only);
            }
            Self::CycleSort => {
                state.replays.sort = state.replays.sort.next();
                cx.scrolled.set(List::Replays, 0.0);
            }
            Self::SetView(view) => {
                state.replays.view = view;
            }
            Self::CyclePlayers => {
                state.replays.players_filter = state.replays.players_filter.next();
                cx.scrolled.set(List::Replays, 0.0);
            }
            Self::ClearSearch => {
                state.lobby.set_field(Field::ReplaySearch, "");
            }
            Self::GoPlay => {
                state.hub = Hub::Play;
            }
            Self::Watch(i) => watch(state, i, None),
            Self::WatchFrom(i, perspective) => watch(state, i, Some(perspective)),
            Self::Archive(i) => archive(state, cx.mailbox, i, true),
            Self::Unarchive(i) => archive(state, cx.mailbox, i, false),
            Self::Share(i) => {
                if let Some(entry) = state.replays.library.entries().get(i).cloned()
                    && let (Some(shared), EntrySource::Hosted { game, .. }) =
                        (entry.sharing, &entry.source)
                {
                    state.replays.library.set_sharing(&entry.source, !shared);
                    send_share(state, cx.mailbox, game, !shared);
                }
            }
            Self::Older => {
                let offset = state.replays.library.online_held;
                ask_online(state, cx.mailbox, offset);
            }
        }
    }
}

/// Opens entry `i` (the list's own index) from `perspective`, or the
/// viewer's seat.
fn watch(state: &mut LobbyState, i: usize, perspective: Option<Perspective>) {
    let Some(entry) = state.replays.library.entries().get(i).cloned() else {
        return;
    };
    let seat = baylee_core::ids::PlayerId::new(entry.seat);
    let perspective = perspective
        .filter(|p| entry.can.offers(*p))
        .unwrap_or(Perspective::Seat(seat));
    let source = match &entry.source {
        EntrySource::Local { file } => ReplayRef::Record(file.clone()),
        EntrySource::Hosted { game, .. } => ReplayRef::Hosted { game: game.clone() },
    };
    let verify = entry.check.verify();
    state.replays.watching = Some(entry);
    state.lobby.watch_replay(ReplayHandover {
        source,
        perspective,
        verify,
        from_end_screen: false,
    });
}

/// Archives entry `i`, or brings it back: a local record moves between
/// its folder and the archive here, a hosted game is archived on the
/// gateway (§18.1: its record stays, only this account's list changes).
fn archive(state: &mut LobbyState, mailbox: &Mailbox, i: usize, archived: bool) {
    let Some(entry) = state.replays.library.entries().get(i).cloned() else {
        return;
    };
    let lang = state.lobby.lang();
    match &entry.source {
        EntrySource::Local { file } => {
            if let Err(why) = archive_local(file, archived) {
                state.lobby.say(ReplayPhrase::Failed.fill(lang, &[&why]));
                return;
            }
        }
        EntrySource::Hosted { game, .. } => send_archive(state, mailbox, game, archived),
    }
    state.replays.library.set_archived(&entry.source, archived);
    let note = if archived {
        ReplayPhrase::ArchivedNote
    } else {
        ReplayPhrase::UnarchivedNote
    };
    state.lobby.note_said(note.text(lang));
}

#[cfg(not(target_arch = "wasm32"))]
fn archive_local(file: &str, archived: bool) -> Result<(), String> {
    crate::replay::local::archive(file, archived)
}

#[cfg(target_arch = "wasm32")]
fn archive_local(_file: &str, _archived: bool) -> Result<(), String> {
    Err("no kept records in a browser".to_string())
}

/// The gateway's archive door for a hosted game (§18.1, owner 13.10.2026:
/// archive, not delete): `PUT /replays/{game}/archived {"archived": …}`.
fn send_archive(state: &LobbyState, mailbox: &Mailbox, game: &str, archived: bool) {
    send(
        state,
        mailbox,
        ehttp::Method::PUT,
        &format!("/replays/{game}/archived"),
        &serde_json::json!({ "archived": archived }),
    );
}

/// The viewer's consent for a hosted game (§18.2): `PUT
/// /replays/{game}/share {"shared": …}`.
pub(super) fn send_share(state: &LobbyState, mailbox: &Mailbox, game: &str, shared: bool) {
    send(
        state,
        mailbox,
        ehttp::Method::PUT,
        &format!("/replays/{game}/share"),
        &serde_json::json!({ "shared": shared }),
    );
}

/// Sends a change to the gateway with the session, off the lobby's chain;
/// a refusal is said on the status line in the gateway's words.
fn send(
    state: &LobbyState,
    mailbox: &Mailbox,
    method: ehttp::Method,
    path: &str,
    body: &serde_json::Value,
) {
    let Some(token) = state.lobby.token() else {
        return;
    };
    let url = format!("{}{path}", state.gateway.trim_end_matches('/'));
    let mut request = ehttp::Request::post(url, serde_json::to_vec(body).unwrap_or_default());
    request.method = method;
    request.headers = ehttp::Headers::new(&[
        ("Accept", "application/json"),
        ("Content-Type", "application/json"),
    ]);
    request
        .headers
        .insert("Authorization", format!("Bearer {token}"));
    let box_ = Arc::clone(&mailbox.0);
    let epoch = state.gateway_epoch;
    crate::transport::fetch(request, move |answer| {
        let refused = match answer {
            Ok(response) if response.ok => return,
            Ok(response) => {
                #[derive(serde::Deserialize)]
                struct Refused {
                    error: String,
                }
                serde_json::from_slice::<Refused>(&response.bytes).map_or_else(
                    |_| format!("{} {}", response.status, response.status_text),
                    |r| r.error,
                )
            }
            Err(why) => why,
        };
        if let Ok(mut box_) = box_.lock() {
            box_.push(Reply::Remote(
                epoch,
                Box::new(Reply::ReplayRefused(refused)),
            ));
        }
    });
}

/// Asks the gateway for the account's archived games.
fn ask_archived(state: &mut LobbyState, mailbox: &Mailbox) {
    let Some(token) = state.lobby.token() else {
        state.replays.archived = Online::Read;
        return;
    };
    if state.lobby.offline() {
        state.replays.archived = Online::Read;
        return;
    }
    state.replays.archived = Online::Asking(0);
    super::http::ask_aside::<ReplayPage>(
        &state.gateway,
        "/replays?archived=true&offset=0&limit=50",
        token,
        state.gateway_epoch,
        mailbox,
        Reply::ReplaysArchived,
    );
}

/// A page of archived online games arrived.
pub(super) fn archived_page(state: &mut LobbyState, page: &ReplayPage) {
    let gateway = state.gateway.clone();
    state.replays.library.add_archived_page(&gateway, 0, page);
    state.replays.archived = Online::Read;
}

/// Asks the gateway for a page of the account's games.
pub(super) fn ask_online(state: &mut LobbyState, mailbox: &Mailbox, offset: u64) {
    let Some(token) = state.lobby.token() else {
        return;
    };
    if state.lobby.offline() {
        return;
    }
    state.replays.online = Online::Asking(offset);
    super::http::ask_aside::<ReplayPage>(
        &state.gateway,
        &format!("/replays?offset={offset}&limit=20"),
        token,
        state.gateway_epoch,
        mailbox,
        Reply::Replays,
    );
}

/// A page of online games arrived.
pub(super) fn online_page(state: &mut LobbyState, page: &ReplayPage) {
    let offset = match state.replays.online {
        Online::Asking(offset) => offset,
        _ => 0,
    };
    let gateway = state.gateway.clone();
    state.replays.library.add_page(&gateway, offset, page);
    state.replays.online = Online::Read;
}

/// While the tab is shown: reads the kept records once a visit, and asks
/// the gateway for the first page.
pub(super) fn visit(mut state: ResMut<LobbyState>, mailbox: Res<Mailbox>) {
    if !SHOWN || state.hub != Hub::Replays || !matches!(state.lobby.screen(), Screen::Table) {
        return;
    }
    if !state.replays.local_read {
        let build = baylee_build::short().to_string();
        let you = ReplayPhrase::You.text(state.lobby.lang()).to_string();
        let local = crate::replay::local::entries(&build, &you);
        state.replays.library.set_local(local);
        state.replays.local_read = true;
    }
    if state.replays.online == Online::Unasked {
        if state.lobby.token().is_some() && !state.lobby.offline() {
            ask_online(&mut state, &mailbox, 0);
        } else {
            state.replays.library.forget_online();
            state.replays.online = Online::Read;
        }
    }
}

/// On `Screen::Replay`: installs the replay's host and opens the table.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn open_replay(
    mut commands: Commands,
    mut state: ResMut<LobbyState>,
    mut opens: MessageWriter<DuelCommand>,
) {
    let Screen::Replay(handover) = state.lobby.screen().clone() else {
        return;
    };
    if state.connected {
        return;
    }
    let lang = state.lobby.lang();
    let gateway_name = state
        .gateway_name()
        .unwrap_or_else(|| state.gateway.clone());
    let entry = state.replays.watching.clone();
    let made = match &handover.source {
        ReplayRef::Record(file) => crate::replay::local::read(file)
            .map(|bytes| {
                let names = record_names(&bytes, lang);
                (
                    crate::replay::ReplaySource::Record {
                        bytes: Arc::new(bytes),
                        names,
                        file: file.clone(),
                        by_hand: false,
                    },
                    Perspectives::LOCAL,
                    ReplayPhrase::ThisComputer.text(lang).to_string(),
                )
            })
            .ok_or_else(|| file.clone()),
        ReplayRef::Memory => state
            .replays
            .memory
            .clone()
            .map(|memory| {
                (
                    crate::replay::ReplaySource::Record {
                        bytes: memory.bytes,
                        names: memory.names,
                        file: memory.file,
                        by_hand: false,
                    },
                    Perspectives::LOCAL,
                    ReplayPhrase::ThisComputer.text(lang).to_string(),
                )
            })
            .ok_or_else(String::new),
        ReplayRef::Hosted { game } => state
            .lobby
            .token()
            .map(|token| {
                (
                    crate::replay::ReplaySource::Hosted(crate::replay::HostedAccess {
                        gateway: state.gateway.clone(),
                        name: gateway_name.clone(),
                        token: token.to_string(),
                        game: game.clone(),
                    }),
                    // From an end sheet there is no entry yet: what any
                    // hosted game offers (§18.3), masked by the gateway.
                    entry.as_ref().map_or(
                        Perspectives {
                            seat: true,
                            public: true,
                            all_hands: true,
                        },
                        |e| e.can,
                    ),
                    gateway_name.clone(),
                )
            })
            .ok_or_else(|| game.clone()),
        ReplayRef::Path(path) => crate::replay::local::open_file(path)
            .map(|(source, offers)| (source, offers, path.display().to_string()))
            .map_err(|_| path.display().to_string()),
    };
    match made {
        Ok((source, offers, place)) => {
            let origin = crate::replay::Origin {
                place,
                started: entry.as_ref().map(|e| e.started).filter(|s| *s > 0),
                // The viewer's seat: the entry's, else (from an end sheet)
                // the seat the replay opens on.
                viewer: entry.as_ref().map_or(handover.perspective.seat(), |e| {
                    baylee_core::ids::PlayerId::new(e.seat)
                }),
                unshared: entry.as_ref().map(unshared_seats).unwrap_or_default(),
            };
            let host = crate::replay::ReplayHost::new(
                source,
                handover.perspective,
                handover.verify,
                offers,
                origin,
            );
            state.connected = true;
            commands.insert_resource(host.into_installed());
            opens.write(DuelCommand::Open);
        }
        Err(what) => {
            state.lobby.leave_replay();
            state.hub = Hub::Replays;
            let why = ReplayPhrase::Failed.fill(lang, &[&what]);
            state.lobby.say(why);
        }
    }
}

/// The browser's build opens no replay.
#[cfg(target_arch = "wasm32")]
pub(super) fn open_replay() {}

/// The name a commander's `CardIndex` goes by, from the compiled pool.
pub(super) fn card_name(index: u32) -> Option<String> {
    baylee_cards::generated_names::NAMES
        .iter()
        .find(|(_, card)| card.get() == index)
        .map(|(name, _)| (*name).to_string())
}

// ------------------------------------------------------------ drawing

/// Spawns one KPI statistic card in the Replays overview banner.
fn spawn_replay_kpi_card(
    commands: &mut Commands,
    kit: Kit,
    icon: char,
    label: &str,
    value: &str,
    sub: &str,
    accent: Color,
) -> Entity {
    let card = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                flex_basis: kit.m.px(195.0),
                min_width: kit.m.px(175.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: kit.m.px(12.0),
                padding: UiRect::axes(kit.m.px(14.0), kit.m.px(10.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(tokens::CONTROL.with_alpha(0.45)),
            BorderColor::all(tokens::BORDER.with_alpha(0.75)),
            Pickable::IGNORE,
        ))
        .id();
    let badge = commands
        .spawn((
            Node {
                width: kit.m.px(38.0),
                height: kit.m.px(38.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                border: UiRect::all(px_fixed(1.0)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(accent.with_alpha(0.14)),
            BorderColor::all(accent.with_alpha(0.38)),
            Pickable::IGNORE,
        ))
        .id();
    let glyph = commands
        .spawn((
            Text::new(icon.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.text * 1.05),
            TextColor(accent),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(badge).add_child(glyph);

    let col = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_shrink: 1.0,
                min_width: px(0),
                row_gap: kit.m.px(2.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let lbl = commands
        .spawn((
            Text::new(label),
            crate::hud::tf(kit.fonts, kit.m.small * 0.90),
            TextColor(tokens::MUTED),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id();
    let val = commands
        .spawn((
            Text::new(value),
            crate::hud::tf_bold(kit.fonts, kit.m.text * 1.08),
            TextColor(tokens::INK),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id();
    let sub_node = commands
        .spawn((
            Text::new(sub),
            crate::hud::tf(kit.fonts, kit.m.small * 0.88),
            TextColor(accent.with_alpha(0.92)),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(col).add_children(&[lbl, val, sub_node]);
    commands.entity(card).add_children(&[badge, col]);
    card
}

/// Computes and spawns the 4-card Replay Analytics & Statistics row.
#[allow(clippy::too_many_lines)]
fn spawn_replays_stats_bar(
    commands: &mut Commands,
    kit: Kit,
    entries: &[Entry],
    lang: Lang,
) -> Entity {
    use baylee_client_core::replay::library::EntryCheck;
    let total = entries.len();
    let local_count = entries.iter().filter(|e| e.is_local()).count();
    let online_count = total.saturating_sub(local_count);

    let wins = entries.iter().filter(|e| e.result == Outcome::Won).count();
    let losses = entries.iter().filter(|e| e.result == Outcome::Lost).count();
    let draws = entries.iter().filter(|e| e.result == Outcome::Draw).count();
    let decided = wins + losses + draws;
    let win_rate = (wins * 100).checked_div(decided).unwrap_or(0);

    // Compute current and best win streak (entries are newest-first)
    let mut cur_streak = 0usize;
    for e in entries {
        if e.result == Outcome::Won {
            cur_streak += 1;
        } else if matches!(e.result, Outcome::Lost | Outcome::Draw) {
            break;
        }
    }
    let mut best_streak = 0usize;
    let mut run = 0usize;
    for e in entries.iter().rev() {
        if e.result == Outcome::Won {
            run += 1;
            best_streak = best_streak.max(run);
        } else if matches!(e.result, Outcome::Lost | Outcome::Draw) {
            run = 0;
        }
    }

    let mut sum_mins = 0u64;
    let mut count_mins = 0u64;
    let mut max_mins = 0u64;
    let mut sum_turns = 0u64;
    let mut count_turns = 0u64;
    let mut max_turns = 0u32;
    for e in entries {
        if let Some(m) = e.minutes() {
            sum_mins += m;
            count_mins += 1;
            max_mins = max_mins.max(m);
        }
        if let Some(t) = e.turns {
            sum_turns += u64::from(t);
            count_turns += 1;
            max_turns = max_turns.max(t);
        }
    }
    let avg_mins = sum_mins.checked_div(count_mins).unwrap_or(0);
    let avg_turns = sum_turns.checked_div(count_turns).unwrap_or(0);

    let verified_count = entries
        .iter()
        .filter(|e| matches!(e.check, EntryCheck::Exact | EntryCheck::Checked))
        .count();
    let all_hands_count = entries
        .iter()
        .filter(|e| {
            let seat = baylee_core::ids::PlayerId::new(e.seat);
            e.can.offers(Perspective::AllHands(seat))
        })
        .count();

    let row = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                column_gap: kit.m.px(10.0),
                row_gap: kit.m.px(8.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();

    let c1 = spawn_replay_kpi_card(
        commands,
        kit,
        '\u{f1da}',
        Phrase::ReplaysKpiRecorded.text(lang),
        &format!("{total}"),
        &Phrase::ReplaysKpiRecordedSub
            .fill(lang, &[&local_count.to_string(), &online_count.to_string()]),
        tokens::ACCENT,
    );
    let c2 = spawn_replay_kpi_card(
        commands,
        kit,
        '\u{f091}',
        Phrase::ReplaysKpiRecord.text(lang),
        &Phrase::ReplaysKpiRecordValue.fill(
            lang,
            &[
                &wins.to_string(),
                &losses.to_string(),
                &win_rate.to_string(),
            ],
        ),
        &Phrase::ReplaysKpiStreaks.fill(lang, &[&best_streak.to_string(), &cur_streak.to_string()]),
        tokens::GOLD,
    );
    let c3 = spawn_replay_kpi_card(
        commands,
        kit,
        '\u{f252}',
        Phrase::ReplaysKpiPace.text(lang),
        &Phrase::ReplaysKpiPaceValue.fill(lang, &[&avg_mins.to_string(), &avg_turns.to_string()]),
        &Phrase::ReplaysKpiRecordTurns.fill(lang, &[&max_turns.to_string(), &max_mins.to_string()]),
        Color::srgb(0.46, 0.78, 0.96),
    );
    let c4 = spawn_replay_kpi_card(
        commands,
        kit,
        '\u{f3ed}',
        Phrase::ReplaysKpiVerification.text(lang),
        &Phrase::ReplaysKpiVerified.fill(lang, &[&verified_count.to_string(), &total.to_string()]),
        &Phrase::ReplaysKpiAllHands.fill(lang, &[&all_hands_count.to_string()]),
        Color::srgb(0.36, 0.84, 0.56),
    );
    commands.entity(row).add_children(&[c1, c2, c3, c4]);
    row
}

/// Interactive pill button with a leading Font Awesome icon for the Replays toolbar.
fn replay_filter_pill(
    commands: &mut Commands,
    kit: Kit,
    icon: char,
    label: &str,
    active: bool,
    press: Press,
) -> Entity {
    let bg = if active {
        tokens::ACCENT.with_alpha(0.20)
    } else {
        tokens::CONTROL.with_alpha(0.55)
    };
    let border = if active {
        tokens::ACCENT
    } else {
        tokens::BORDER
    };
    let ink = if active { tokens::INK } else { tokens::MUTED };
    let pill = commands
        .spawn((
            crate::shellkit::role::Role::Button,
            Node {
                min_height: kit.m.px(32.0),
                padding: UiRect::axes(kit.m.px(11.0), kit.m.px(4.0)),
                column_gap: kit.m.px(6.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(bg),
            BorderColor::all(border),
            UiTransform::default(),
            crate::ambience::Feel::rising_to(bg, tokens::HOVER),
            press,
        ))
        .id();
    let glyph = commands
        .spawn((
            Text::new(icon.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.small * 0.94),
            TextColor(if active { tokens::ACCENT } else { ink }),
            Pickable::IGNORE,
        ))
        .id();
    let txt = commands
        .spawn((
            Text::new(label),
            crate::hud::tf_bold(kit.fonts, kit.m.small * 0.94),
            TextColor(ink),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(pill).add_children(&[glyph, txt]);
    pill
}

/// Rich empty state when no replays exist yet or no replay matches the active filters.
#[allow(clippy::too_many_lines)] // icon disc + headline + prose + 3 feature badges + primary CTA button
fn spawn_replays_empty_state(
    commands: &mut Commands,
    kit: Kit,
    lang: Lang,
    no_records_at_all: bool,
) -> Entity {
    let card = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: kit.m.px(12.0),
                padding: UiRect::axes(kit.m.px(24.0), kit.m.px(32.0)),
                margin: UiRect::top(kit.m.px(6.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
                ..default()
            },
            BackgroundColor(tokens::CONTROL.with_alpha(0.32)),
            BorderColor::all(tokens::BORDER.with_alpha(0.65)),
            Pickable::IGNORE,
        ))
        .id();
    let disc = commands
        .spawn((
            Node {
                width: kit.m.px(56.0),
                height: kit.m.px(56.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                border: UiRect::all(px_fixed(1.5)),
                ..default()
            },
            BackgroundColor(tokens::ACCENT.with_alpha(0.14)),
            BorderColor::all(tokens::ACCENT.with_alpha(0.45)),
            Pickable::IGNORE,
        ))
        .id();
    let icon = commands
        .spawn((
            Text::new('\u{f1da}'.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.head * 1.15),
            TextColor(tokens::ACCENT),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(disc).add_child(icon);
    commands.entity(card).add_child(disc);

    let headline = commands
        .spawn((
            Text::new(if no_records_at_all {
                Phrase::ReplaysNoneYet.text(lang)
            } else {
                ReplayPhrase::NoMatch.text(lang)
            }),
            crate::hud::tf_bold(kit.fonts, kit.m.text * 1.15),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(card).add_child(headline);

    if no_records_at_all {
        let sub =
            crate::shellkit::surfaces::prose(commands, kit, ReplayPhrase::Empty.text(lang), true);
        commands.entity(card).add_child(sub);

        let perks = parts::row(commands, kit, true);
        let p1 = parts::badge_with(
            commands,
            kit,
            '\u{f04b}',
            Phrase::ReplaysPerkTimeline.text(lang),
            tokens::ACCENT,
        );
        let p2 = parts::badge_with(
            commands,
            kit,
            '\u{f06e}',
            Phrase::ReplaysPerkPerspectives.text(lang),
            tokens::GOLD,
        );
        let p3 = parts::badge_with(
            commands,
            kit,
            '\u{f3ed}',
            Phrase::ReplaysPerkDeterministic.text(lang),
            Color::srgb(0.36, 0.84, 0.56),
        );
        commands.entity(perks).add_children(&[p1, p2, p3]);
        commands.entity(card).add_child(perks);

        let cta = controls::button_with_icon(
            commands,
            kit,
            '\u{f04b}',
            Phrase::ReplaysPlayNow.text(lang),
            Weight::Primary,
            Live::Yes,
            None,
            Press::Replays(ReplaysPress::GoPlay),
        );
        commands.entity(card).add_child(cta);
    }
    card
}

/// The Replays screen under the header.
#[allow(clippy::too_many_lines)] // one screen, read top to bottom
pub(super) fn draw(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    metrics: Metrics,
    kit: Kit,
    scrolled: &Scrolled,
) {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let ui = &state.replays;
    let body = commands
        .spawn((
            Node {
                width: percent(100),
                flex_grow: 1.0,
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(px_fixed(kit.m.body)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(body);
    let panel = commands
        .spawn((
            crate::shellkit::role::Role::Panel,
            Node {
                width: percent(100),
                max_width: px_fixed(kit.m.body_max),
                flex_grow: 1.0,
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px_fixed(kit.m.pad)),
                row_gap: px_fixed(kit.m.gap * 0.85),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
                ..default()
            },
            BackgroundColor(tokens::PANEL),
            BorderColor::all(tokens::BORDER),
        ))
        .id();
    commands.entity(body).add_child(panel);

    let mut filters = ui.filters.clone();
    filters.query = lobby.field(Field::ReplaySearch).to_string();
    let names = |index: u32| card_name(index);
    let mut shown: Vec<(usize, &Entry)> = ui
        .library
        .entries()
        .iter()
        .enumerate()
        .filter(|(_, e)| filters.pass(e, &names) && ui.players_filter.matches(e.players.len()))
        .collect();
    match ui.sort {
        ReplaysSort::Newest => {}
        ReplaysSort::Longest => {
            shown.sort_by_key(|(_, e)| std::cmp::Reverse(e.minutes().unwrap_or(0)));
        }
        ReplaysSort::MostTurns => {
            shown.sort_by_key(|(_, e)| std::cmp::Reverse(e.turns.unwrap_or(0)));
        }
    }

    // ---- 1. Heading, live count badge, and search box
    let head = parts::row(commands, kit, true);
    commands
        .entity(head)
        .entry::<Node>()
        .and_modify(|mut n| n.width = percent(100));
    let head_icon = commands
        .spawn((
            Text::new('\u{f1da}'.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.head * 0.92),
            TextColor(tokens::ACCENT),
            Pickable::IGNORE,
        ))
        .id();
    let title = crate::shellkit::surfaces::heading(commands, kit, ReplayPhrase::Replays.text(lang));
    let count_badge = parts::badge(
        commands,
        kit,
        &Phrase::ReplaysCount.fill(
            lang,
            &[
                &shown.len().to_string(),
                &ui.library.entries().len().to_string(),
            ],
        ),
        tokens::ACCENT,
    );
    let gap = parts::grow(commands);
    let hunt = commands
        .spawn((
            Node {
                width: kit.m.px(320.0),
                flex_shrink: 1.0,
                min_width: px(140),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let typing = lobby.focus() == Field::ReplaySearch && lobby.typing_here();
    let typed = !lobby.field(Field::ReplaySearch).is_empty();
    let field = text_field(
        commands,
        kit.fonts,
        metrics,
        "",
        &FieldLook {
            buffer: lobby.buffer(Field::ReplaySearch),
            focused: typing,
            mask: None,
            press: Press::Shared(SharedPress::Focus(Field::ReplaySearch)),
            lead: Some(crate::hud::glyph::MAGNIFIER),
            hint: Some(ReplayPhrase::SearchHint.text(lang)),
            tail: typed.then_some(FieldTail {
                glyph: crate::hud::glyph::CLOSE,
                press: Press::Replays(ReplaysPress::ClearSearch),
                lit: false,
            }),
        },
    );
    commands.entity(hunt).add_child(field);
    commands
        .entity(head)
        .add_children(&[head_icon, title, count_badge, gap, hunt]);
    commands.entity(panel).add_child(head);

    // ---- 2. Analytics & KPI Dashboard Strip
    let stats_bar = spawn_replays_stats_bar(commands, kit, ui.library.entries(), lang);
    commands.entity(panel).add_child(stats_bar);

    // ---- 3. Filter, Sort & View Toolbar
    let bar = parts::row(commands, kit, true);
    commands.entity(bar).entry::<Node>().and_modify(|mut n| {
        n.width = percent(100);
    });
    let sources = [
        (SourceFilter::All, ReplayPhrase::FilterAll),
        (SourceFilter::House, ReplayPhrase::FilterHouse),
        (SourceFilter::Online, ReplayPhrase::FilterOnline),
        (SourceFilter::Archived, ReplayPhrase::Archived),
    ];
    let words: Vec<&str> = sources.iter().map(|(_, p)| p.text(lang)).collect();
    let at = sources
        .iter()
        .position(|(s, _)| *s == ui.filters.source)
        .unwrap_or(0);
    let segments = controls::segmented(commands, kit, &words, at, |i| {
        Press::Replays(ReplaysPress::Source(sources[i].0))
    });
    commands.entity(bar).add_child(segments);
    let results = [
        (None, '\u{f00a}'),
        (Some(Outcome::Won), '\u{f091}'),
        (Some(Outcome::Lost), '\u{f714}'),
        (Some(Outcome::Draw), '\u{f2b5}'),
        (Some(Outcome::Unfinished), '\u{f252}'),
    ];
    for (result, icon) in results {
        let on = match ui.filters.result {
            ResultFilter::Any => result.is_none(),
            ResultFilter::Only(o) => result == Some(o),
        };
        let words = result.map_or(ReplayPhrase::AnyResult, ReplayPhrase::outcome);
        let pill = replay_filter_pill(
            commands,
            kit,
            icon,
            words.text(lang),
            on,
            Press::Replays(ReplaysPress::Result(result)),
        );
        commands.entity(bar).add_child(pill);
    }

    let bar_gap = parts::grow(commands);
    commands.entity(bar).add_child(bar_gap);

    let players_pill = replay_filter_pill(
        commands,
        kit,
        match ui.players_filter {
            ReplayPlayersFilter::All => '\u{f0c0}',
            ReplayPlayersFilter::Duel => '\u{f6de}',
            ReplayPlayersFilter::Multiplayer => '\u{f500}',
        },
        ui.players_filter.label(lang),
        ui.players_filter != ReplayPlayersFilter::All,
        Press::Replays(ReplaysPress::CyclePlayers),
    );
    let sort_pill = replay_filter_pill(
        commands,
        kit,
        '\u{f0dc}',
        ui.sort.label(lang),
        ui.sort != ReplaysSort::Newest,
        Press::Replays(ReplaysPress::CycleSort),
    );
    let view_modes = [ReplaysView::Detailed, ReplaysView::Compact];
    let active_v = view_modes.iter().position(|&v| v == ui.view).unwrap_or(0);
    let view_seg = controls::segmented_icons(
        commands,
        kit,
        &[
            ('\u{f009}', Phrase::ReplaysViewDetailed.text(lang)),
            ('\u{f03a}', Phrase::DecksViewCompact.text(lang)),
        ],
        active_v,
        |i| Press::Replays(ReplaysPress::SetView(view_modes[i])),
    );
    commands
        .entity(bar)
        .add_children(&[players_pill, sort_pill, view_seg]);
    commands.entity(panel).add_child(bar);

    // ---- 4. Scrollable Replay List
    let scroll = commands
        .spawn((
            Node {
                width: percent(100),
                flex_grow: 1.0,
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px_fixed(kit.m.gap * 0.55),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            Scrollable(List::Replays),
            ScrollPosition(Vec2::new(0.0, scrolled.get(List::Replays))),
        ))
        .id();
    commands.entity(panel).add_child(scroll);

    if ui.library.entries().is_empty() {
        let empty = spawn_replays_empty_state(commands, kit, lang, true);
        commands.entity(scroll).add_child(empty);
    } else if shown.is_empty() {
        let none = spawn_replays_empty_state(commands, kit, lang, false);
        commands.entity(scroll).add_child(none);
    }

    let offset = local_offset();
    let today = baylee_client_core::replay::library::day_of(parts::now_secs() * 1000, offset);
    let mut last_day = None;
    for (i, entry) in shown {
        if ui.sort == ReplaysSort::Newest {
            let day = baylee_client_core::replay::library::day_of(entry.started, offset);
            if last_day != Some(day) {
                last_day = Some(day);
                let heading = day_heading(day, today, entry.started, offset, lang);
                let day_row = parts::row(commands, kit, false);
                let cal_icon = commands
                    .spawn((
                        Text::new('\u{f133}'.to_string()),
                        crate::hud::icon_tf(kit.fonts, kit.m.small * 0.92),
                        TextColor(tokens::ACCENT),
                        Pickable::IGNORE,
                    ))
                    .id();
                let line = parts::line(commands, kit, &heading, kit.m.small, tokens::MUTED);
                commands.entity(day_row).add_children(&[cal_icon, line]);
                commands.entity(scroll).add_child(day_row);
            }
        }
        let row = entry_row(commands, root, kit, state, i, entry, offset, lang);
        commands.entity(scroll).add_child(row);
    }
    if ui.library.more_online() && ui.filters.source != SourceFilter::House {
        let older = controls::button_with_icon(
            commands,
            kit,
            '\u{f017}',
            ReplayPhrase::OlderOnline.text(lang),
            Weight::Secondary,
            if matches!(ui.online, Online::Asking(_)) {
                Live::No("\u{2026}")
            } else {
                Live::Yes
            },
            None,
            Press::Replays(ReplaysPress::Older),
        );
        commands.entity(scroll).add_child(older);
    }

    let foot = parts::line(
        commands,
        kit,
        ReplayPhrase::FootLine.text(lang),
        kit.m.small,
        tokens::MUTED,
    );
    commands.entity(panel).add_child(foot);
}

/// The local offset from UTC, in seconds, now.
fn local_offset() -> i32 {
    let now = i64::try_from(parts::now_secs()).unwrap_or(0);
    time::OffsetDateTime::from_unix_timestamp(now)
        .ok()
        .and_then(|utc| time::UtcOffset::local_offset_at(utc).ok())
        .map_or(0, time::UtcOffset::whole_seconds)
}

/// A day's heading: Today, Yesterday, else the date.
fn day_heading(day: i64, today: i64, ms: u64, offset: i32, lang: Lang) -> String {
    match today - day {
        0 => ReplayPhrase::Today.text(lang).to_uppercase(),
        1 => ReplayPhrase::Yesterday.text(lang).to_uppercase(),
        _ => {
            let at = local_time(ms, offset);
            at.map_or_else(String::new, |at| match lang {
                Lang::De => format!("{}. {}", at.day(), month(lang, at.month())),
                Lang::En => format!("{} {}", at.day(), month(lang, at.month())),
            })
            .to_uppercase()
        }
    }
}

fn month(lang: Lang, month: time::Month) -> &'static str {
    let at = u8::from(month).saturating_sub(1) as usize;
    let en = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let de = [
        "Jan.", "Feb.", "März", "Apr.", "Mai", "Juni", "Juli", "Aug.", "Sep.", "Okt.", "Nov.",
        "Dez.",
    ];
    match lang {
        Lang::De => de[at.min(11)],
        Lang::En => en[at.min(11)],
    }
}

fn local_time(ms: u64, offset: i32) -> Option<time::OffsetDateTime> {
    let utc = time::OffsetDateTime::from_unix_timestamp(i64::try_from(ms / 1000).ok()?).ok()?;
    Some(utc.to_offset(time::UtcOffset::from_whole_seconds(offset).ok()?))
}

/// What a row says of its players: "You (G) · Bob#0012 (Atraxa) · …".
pub(super) fn players_line(entry: &Entry, lang: Lang) -> String {
    entry
        .players
        .iter()
        .map(|p| {
            let name = if p.you {
                ReplayPhrase::You.text(lang).to_string()
            } else if let Some(level) = &p.house {
                if level.is_empty() {
                    ReplayPhrase::House.text(lang).to_string()
                } else {
                    ReplayPhrase::HouseLevel.fill(lang, &[level])
                }
            } else if p.model {
                ReplayPhrase::LanguageModel.text(lang).to_string()
            } else if p.former {
                ReplayPhrase::FormerPlayer.text(lang).to_string()
            } else {
                p.name.clone()
            };
            let mark = entry.decks.iter().find(|d| d.seat == p.seat).and_then(|d| {
                d.commander.and_then(card_name).or_else(|| {
                    d.colours
                        .as_ref()
                        .filter(|c| !c.is_empty() && (p.you || entry.is_local()))
                        .cloned()
                })
            });
            match mark {
                Some(mark) => format!("{name} ({mark})"),
                None => name,
            }
        })
        .collect::<Vec<_>>()
        .join(" \u{b7} ")
}

/// Returns `(icon, accent_color)` for a replay `Outcome`.
fn outcome_style(outcome: Outcome) -> (char, Color) {
    match outcome {
        Outcome::Won => ('\u{f091}', Color::srgb(0.32, 0.84, 0.52)),
        Outcome::Lost => ('\u{f714}', tokens::DANGER),
        Outcome::Draw => ('\u{f2b5}', tokens::GOLD),
        Outcome::Unfinished | Outcome::Unknown => ('\u{f252}', tokens::MUTED),
    }
}

/// Spawns a compact 28×28 vertical `⋮` menu button for a replay card.
fn spawn_replay_more(commands: &mut Commands, kit: Kit, lang: Lang, press: Press) -> Entity {
    let tip = Phrase::MoreActions.text(lang);
    let btn = commands
        .spawn((
            crate::shellkit::role::Role::Button,
            Node {
                width: kit.m.px(28.0),
                height: kit.m.px(28.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(Color::NONE),
            UiTransform::default(),
            crate::ambience::Feel::rising_to(Color::NONE, tokens::HOVER),
            super::hint::HoverHint(tip.to_string()),
            press,
        ))
        .id();
    let icon = commands
        .spawn((
            Text::new(parts::ELLIPSIS.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.text * 0.92),
            TextColor(tokens::MUTED.with_alpha(0.85)),
            UiTransform {
                rotation: Rot2::radians(std::f32::consts::FRAC_PI_2),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(btn).add_child(icon);
    btn
}

/// Spawns a seat/player pill showing the seat icon, player name, commander name,
/// and mana colour identity discs.
#[allow(clippy::too_many_lines)] // seat icon + name + commander badge + mana discs + shared eye
fn spawn_seat_pill(
    commands: &mut Commands,
    kit: Kit,
    player: &baylee_client_core::replay::library::Player,
    entry: &Entry,
    lang: Lang,
) -> Entity {
    let name = if player.you {
        ReplayPhrase::You.text(lang).to_string()
    } else if let Some(level) = &player.house {
        if level.is_empty() {
            ReplayPhrase::House.text(lang).to_string()
        } else {
            ReplayPhrase::HouseLevel.fill(lang, &[level])
        }
    } else if player.model {
        ReplayPhrase::LanguageModel.text(lang).to_string()
    } else if player.former {
        ReplayPhrase::FormerPlayer.text(lang).to_string()
    } else {
        player.name.clone()
    };
    let seat_icon = if player.you {
        '\u{f007}'
    } else if player.house.is_some() || player.model {
        '\u{f544}'
    } else {
        '\u{f007}'
    };
    let deck_mark = entry.decks.iter().find(|d| d.seat == player.seat);
    let commander_name = deck_mark.and_then(|d| d.commander).and_then(card_name);
    let colours = deck_mark
        .and_then(|d| d.colours.as_deref())
        .filter(|c| !c.is_empty() && (player.you || entry.is_local()));

    let pill = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: kit.m.px(6.0),
                padding: UiRect::axes(kit.m.px(9.0), kit.m.px(4.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(if player.you {
                tokens::ACCENT.with_alpha(0.14)
            } else {
                tokens::CONTROL.with_alpha(0.55)
            }),
            BorderColor::all(if player.you {
                tokens::ACCENT.with_alpha(0.50)
            } else {
                tokens::BORDER.with_alpha(0.75)
            }),
            Pickable::IGNORE,
        ))
        .id();
    let icon_node = commands
        .spawn((
            Text::new(seat_icon.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.small * 0.88),
            TextColor(if player.you {
                tokens::ACCENT
            } else {
                tokens::MUTED
            }),
            Pickable::IGNORE,
        ))
        .id();
    let name_node = commands
        .spawn((
            Text::new(name),
            crate::hud::tf_bold(kit.fonts, kit.m.small * 0.92),
            TextColor(if player.you {
                tokens::INK
            } else {
                tokens::INK.with_alpha(0.88)
            }),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(pill).add_children(&[icon_node, name_node]);

    if let Some(cmd) = commander_name {
        let cmd_node = commands
            .spawn((
                Text::new(format!("\u{f521} {cmd}")),
                crate::hud::tf(kit.fonts, kit.m.small * 0.88),
                TextColor(tokens::GOLD),
                TextLayout::no_wrap(),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(pill).add_child(cmd_node);
    }
    if let Some(cols) = colours {
        let discs = parts::identity_discs(commands, kit, cols);
        commands.entity(pill).add_child(discs);
    }
    if player.shared {
        let shared_icon = commands
            .spawn((
                Text::new('\u{f06e}'.to_string()),
                crate::hud::icon_tf(kit.fonts, kit.m.small * 0.82),
                TextColor(tokens::MUTED.with_alpha(0.75)),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(pill).add_child(shared_icon);
    }
    pill
}

/// One row: when, where, who, how it went, how long; Watch and `⋮`.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn entry_row(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    state: &LobbyState,
    i: usize,
    entry: &Entry,
    offset: i32,
    lang: Lang,
) -> Entity {
    use baylee_client_core::replay::library::EntryCheck;
    let clock = local_time(entry.started, offset)
        .map(|at| format!("{:02}:{:02}", at.hour(), at.minute()))
        .unwrap_or_default();
    let (source_icon, place) = match &entry.source {
        EntrySource::Local { .. } => {
            let house = entry
                .house
                .as_deref()
                .and_then(|level| baylee_client_core::i18n::ai_name(lang, level))
                .or(entry.house.as_deref())
                .unwrap_or_else(|| ReplayPhrase::House.text(lang));
            (
                '\u{f108}',
                format!(
                    "{} \u{b7} {}",
                    ReplayPhrase::ThisComputer.text(lang),
                    ReplayPhrase::VsHouse.fill(lang, &[house])
                ),
            )
        }
        EntrySource::Hosted { .. } => (
            '\u{f0ac}',
            state
                .gateway_name()
                .unwrap_or_else(|| state.gateway.clone()),
        ),
    };
    let (outcome_icon, outcome_color) = outcome_style(entry.result);
    let outcome_word = ReplayPhrase::outcome(entry.result).text(lang);

    let watch = controls::button_compact(
        commands,
        kit,
        Some('\u{f04b}'),
        ReplayPhrase::Watch.text(lang),
        Weight::Primary,
        Live::Yes,
        Press::Replays(ReplaysPress::Watch(i)),
    );
    let more = spawn_replay_more(
        commands,
        kit,
        lang,
        Press::Shared(SharedPress::OpenMenu(ShellMenu::Replay(i))),
    );
    if state.menu == Some(ShellMenu::Replay(i)) {
        let items = row_menu(i, entry, lang);
        menus::draw(
            commands,
            root,
            kit,
            more,
            items.iter().map(|(text, press, destructive, keys)| {
                crate::shellkit::surfaces::MenuItem {
                    text,
                    keys: *keys,
                    destructive: *destructive,
                    action: *press,
                }
            }),
        );
    }

    if state.replays.view == ReplaysView::Compact {
        let title = format!("{clock}  {place}");
        let mut meta = vec![players_line(entry, lang), outcome_word.to_string()];
        if let Some(turn) = entry.turns {
            meta.push(ReplayPhrase::TurnShort.fill(lang, &[&turn.to_string()]));
        }
        if let Some(minutes) = entry.minutes() {
            meta.push(ReplayPhrase::Minutes.fill(lang, &[&minutes.to_string()]));
        }
        return crate::shellkit::surfaces::list_row(
            commands,
            kit,
            &title,
            &meta.join(" \u{b7} "),
            &[watch, more],
            (
                Press::Replays(ReplaysPress::Watch(i)),
                crate::ambience::Feel::tinting_to(Color::NONE, tokens::HOVER),
            ),
        );
    }

    // Detailed card view
    let card = commands
        .spawn((
            crate::shellkit::role::Role::Row,
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(8.0),
                padding: UiRect::axes(kit.m.px(14.0), kit.m.px(11.0)),
                border: UiRect {
                    left: px_fixed(3.0),
                    top: px_fixed(1.0),
                    right: px_fixed(1.0),
                    bottom: px_fixed(1.0),
                },
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
                ..default()
            },
            BackgroundColor(tokens::CONTROL.with_alpha(0.36)),
            BorderColor {
                left: outcome_color,
                top: tokens::BORDER.with_alpha(0.75),
                right: tokens::BORDER.with_alpha(0.75),
                bottom: tokens::BORDER.with_alpha(0.75),
            },
            UiTransform::default(),
            crate::ambience::Feel::tinting_to(tokens::CONTROL.with_alpha(0.36), tokens::HOVER),
            Press::Replays(ReplaysPress::Watch(i)),
        ))
        .id();

    // Top row: outcome badge, clock + source, format/turns/minutes/check badges, and Watch + ⋮
    let top_row = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: kit.m.px(10.0),
                row_gap: kit.m.px(6.0),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let left_badges = parts::row(commands, kit, true);
    commands
        .entity(left_badges)
        .entry::<Node>()
        .and_modify(|mut n| {
            n.flex_grow = 1.0;
            n.flex_shrink = 1.0;
            n.min_width = px(0);
        });
    let out_pill = parts::badge_with(commands, kit, outcome_icon, outcome_word, outcome_color);
    let time_pill = parts::badge_with(commands, kit, '\u{f017}', &clock, tokens::INK);
    let place_pill = parts::badge_with(commands, kit, source_icon, &place, tokens::ACCENT);
    let seats_count = entry.players.len().max(2);
    let table_pill = parts::badge_with(
        commands,
        kit,
        if seats_count <= 2 {
            '\u{f6de}'
        } else {
            '\u{f500}'
        },
        &if seats_count <= 2 {
            Phrase::ReplaysFilterDuel.text(lang).to_string()
        } else {
            Phrase::ReplaysPlayers.fill(lang, &[&seats_count.to_string()])
        },
        tokens::INK.with_alpha(0.85),
    );
    commands
        .entity(left_badges)
        .add_children(&[out_pill, time_pill, place_pill, table_pill]);

    if let Some(turn) = entry.turns {
        let turn_pill = parts::badge_with(
            commands,
            kit,
            '\u{f132}',
            &ReplayPhrase::TurnShort.fill(lang, &[&turn.to_string()]),
            tokens::GOLD,
        );
        commands.entity(left_badges).add_child(turn_pill);
    }
    if let Some(minutes) = entry.minutes() {
        let min_pill = parts::badge_with(
            commands,
            kit,
            '\u{f252}',
            &ReplayPhrase::Minutes.fill(lang, &[&minutes.to_string()]),
            tokens::MUTED,
        );
        commands.entity(left_badges).add_child(min_pill);
    }
    let check_pill = match entry.check {
        EntryCheck::Exact => parts::badge_with(
            commands,
            kit,
            '\u{f058}',
            Phrase::ReplaysExact.text(lang),
            Color::srgb(0.36, 0.84, 0.56),
        ),
        EntryCheck::Checked => parts::badge_with(
            commands,
            kit,
            '\u{f3ed}',
            ReplayPhrase::Checked.text(lang),
            tokens::ACCENT,
        ),
        EntryCheck::Unverified => parts::badge_with(
            commands,
            kit,
            '\u{f071}',
            ReplayPhrase::Unverified.text(lang),
            tokens::GOLD,
        ),
    };
    commands.entity(left_badges).add_child(check_pill);

    let actions_row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: kit.m.px(6.0),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(actions_row).add_children(&[watch, more]);
    commands
        .entity(top_row)
        .add_children(&[left_badges, actions_row]);
    commands.entity(card).add_child(top_row);

    // Bottom row: Player & Commander Roster Pills
    let roster = parts::row(commands, kit, true);
    for player in &entry.players {
        let seat_pill = spawn_seat_pill(commands, kit, player, entry, lang);
        commands.entity(roster).add_child(seat_pill);
    }
    commands.entity(card).add_child(roster);
    card
}

/// A row's `⋯` menu: Watch, Watch from each perspective the entry
/// offers, the viewer's consent (a hosted game), Archive or Unarchive.
/// Each item: its words, its press, whether it is the destructive one, its
/// cap.
fn row_menu(
    i: usize,
    entry: &Entry,
    lang: Lang,
) -> Vec<(String, Press, bool, Option<&'static str>)> {
    let seat = baylee_core::ids::PlayerId::new(entry.seat);
    let mut items = vec![(
        ReplayPhrase::Watch.text(lang).to_string(),
        Press::Replays(ReplaysPress::Watch(i)),
        false,
        None,
    )];
    let froms = [
        (Perspective::Seat(seat), ReplayPhrase::YourSeat),
        (Perspective::Public, ReplayPhrase::TheTable),
        (Perspective::AllHands(seat), ReplayPhrase::AllHands),
    ];
    for (perspective, words) in froms {
        if entry.can.offers(perspective) {
            items.push((
                format!(
                    "{} {}",
                    ReplayPhrase::WatchFrom.text(lang),
                    words.text(lang)
                ),
                Press::Replays(ReplaysPress::WatchFrom(i, perspective)),
                false,
                None,
            ));
        }
    }
    if let Some(shared) = entry.sharing {
        items.push((
            ReplayPhrase::ShareHand.text(lang).to_string(),
            Press::Replays(ReplaysPress::Share(i)),
            false,
            Some(if shared {
                ReplayPhrase::On.text(lang)
            } else {
                ReplayPhrase::Off.text(lang)
            }),
        ));
    }
    if entry.archived {
        items.push((
            ReplayPhrase::Unarchive.text(lang).to_string(),
            Press::Replays(ReplaysPress::Unarchive(i)),
            false,
            None,
        ));
    } else {
        items.push((
            ReplayPhrase::Archive.text(lang).to_string(),
            Press::Replays(ReplaysPress::Archive(i)),
            false,
            None,
        ));
    }
    items
}

#[cfg(not(target_arch = "wasm32"))]
/// What each seat of a kept record is called: as the live table called it
/// (`offline::seat_names`, from the record's own preset).
fn record_names(record: &[u8], lang: Lang) -> Vec<String> {
    crate::replay::local::preset(record)
        .map(|preset| super::offline::seat_names(&preset, lang))
        .unwrap_or_default()
}

/// A way off an end sheet that opens a replay once the table is down
/// (`came_back` opens `ReplaysUi::next`): the game just played (Watch
/// replay), or the replay just watched again or from elsewhere.
pub(super) fn from_the_end(end: EndPress, state: &mut LobbyState, host: Option<&InstalledHost>) {
    let seat_of = |n: u32| baylee_core::ids::PlayerId::new(u8::try_from(n).unwrap_or(0));
    let next = match (end, state.lobby.screen().clone()) {
        (EndPress::Replay, Screen::Seated(handover)) if handover.local => {
            let Some(host) = host else { return };
            let (Some(point), Some(record)) = (host.0.resume_point(), host.0.local_record()) else {
                return;
            };
            let seat = point.seat;
            state.replays.memory = Some(MemoryRecord {
                bytes: Arc::new(record.to_vec()),
                names: point.names,
                file: point.record,
            });
            state.replays.watching = None;
            ReplayHandover {
                source: ReplayRef::Memory,
                perspective: Perspective::Seat(seat),
                verify: Verify::Stop,
                from_end_screen: true,
            }
        }
        (EndPress::Replay, Screen::Seated(handover)) if !handover.watching() => {
            state.replays.watching = None;
            ReplayHandover {
                source: ReplayRef::Hosted {
                    game: handover.game_id.clone(),
                },
                perspective: Perspective::Seat(seat_of(handover.seat)),
                verify: Verify::Stop,
                from_end_screen: true,
            }
        }
        (EndPress::WatchAgain | EndPress::WatchFrom, Screen::Replay(mut handover)) => {
            if let Some(status) = host.and_then(|h| h.0.replay()) {
                handover.perspective = status.perspective;
                handover.verify = status.verify;
                if end == EndPress::WatchFrom {
                    handover.perspective = status
                        .offers
                        .after(status.perspective, status.perspective.seat());
                }
            }
            handover
        }
        _ => return,
    };
    state.replays.next = Some(next);
}

/// The seats of an entry whose players did not share their hands
/// (REPLAY-DESIGN §18): a hosted game's people who did not say yes; a
/// house seat is always shared, and a local record holds every hand.
#[cfg(not(target_arch = "wasm32"))]
fn unshared_seats(entry: &Entry) -> Vec<baylee_core::ids::PlayerId> {
    if entry.is_local() {
        return Vec::new();
    }
    entry
        .players
        .iter()
        .filter(|p| !p.you && p.house.is_none() && !p.shared)
        .map(|p| baylee_core::ids::PlayerId::new(p.seat))
        .collect()
}
