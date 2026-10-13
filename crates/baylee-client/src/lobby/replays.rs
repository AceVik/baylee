//! The Replays tab (`Hub::Replays`, `.claude/b8/REPLAY-DESIGN.md` §7.2) and
//! the door from the lobby into a replay (`Screen::Replay`).
//!
//! One list, grouped by day, newest first: this computer's kept records of
//! games against the house, and the hosted games the account sat in, read
//! from the gateway's `GET /replays` a page at a time. Desktop only: the web
//! and mobile builds draw no tab.

use std::sync::Arc;

use baylee_client_core::lobby::{ReplayHandover, ReplayRef};
use baylee_client_core::replay::library::{
    Entry, EntrySource, Filters, Library, Outcome, ReplayPage, ResultFilter, SourceFilter,
};
use baylee_client_core::replay::words::ReplayPhrase;
use baylee_client_core::replay::{Perspective, Perspectives, Verify};

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

/// The Replays tab's state, and what a replay being opened needs.
#[derive(Default)]
pub(crate) struct ReplaysUi {
    /// Every entry.
    pub(crate) library: Library,
    /// Which source and result are shown (the query is the search field's).
    pub(crate) filters: Filters,
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
    /// The local entry whose deletion is being asked, by index.
    pub(crate) delete: Option<usize>,
}

impl ReplaysUi {
    /// The list is read again on the next visit.
    pub(crate) fn forget_visit(&mut self) {
        self.local_read = false;
        self.online = Online::Unasked;
    }
}

/// A control of the Replays tab.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ReplaysPress {
    /// Show this source.
    Source(SourceFilter),
    /// Show this result (`None`: any).
    Result(Option<Outcome>),
    /// Watch entry `i` from its default perspective.
    Watch(usize),
    /// Watch entry `i` from this perspective.
    WatchFrom(usize, Perspective),
    /// Ask whether to delete entry `i` (a local record).
    Delete(usize),
    /// Delete it.
    ConfirmDelete,
    /// Keep it.
    CancelDelete,
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
            Self::Source(source) => state.replays.filters.source = source,
            Self::Result(result) => {
                state.replays.filters.result = result.map_or(ResultFilter::Any, ResultFilter::Only);
            }
            Self::Watch(i) => watch(state, i, None),
            Self::WatchFrom(i, perspective) => watch(state, i, Some(perspective)),
            Self::Delete(i) => state.replays.delete = Some(i),
            Self::CancelDelete => state.replays.delete = None,
            Self::ConfirmDelete => {
                if let Some(i) = state.replays.delete.take()
                    && let Some(entry) = state.replays.library.entries().get(i).cloned()
                    && entry.deletable(None)
                    && let EntrySource::Local { file } = &entry.source
                {
                    crate::replay::local::delete(file);
                    state.replays.library.remove(&entry.source);
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
                    entry.as_ref().map_or(
                        Perspectives {
                            seat: true,
                            public: true,
                            all_hands: false,
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
                row_gap: px_fixed(kit.m.gap),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
                ..default()
            },
            BackgroundColor(tokens::PANEL),
            BorderColor::all(tokens::BORDER),
        ))
        .id();
    commands.entity(body).add_child(panel);

    // ---- the heading and the search
    let head = parts::row(commands, kit, true);
    commands
        .entity(head)
        .entry::<Node>()
        .and_modify(|mut n| n.width = percent(100));
    let title = crate::shellkit::surfaces::heading(commands, kit, ReplayPhrase::Replays.text(lang));
    let gap = parts::grow(commands);
    let hunt = commands
        .spawn((
            Node {
                width: kit.m.px(300.0),
                flex_shrink: 1.0,
                min_width: px(120),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let typing = lobby.focus() == Field::ReplaySearch && lobby.typing_here();
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
            tail: None,
        },
    );
    commands.entity(hunt).add_child(field);
    commands.entity(head).add_children(&[title, gap, hunt]);
    commands.entity(panel).add_child(head);

    // ---- the filters
    let bar = parts::row(commands, kit, true);
    let sources = [
        (SourceFilter::All, ReplayPhrase::FilterAll),
        (SourceFilter::House, ReplayPhrase::FilterHouse),
        (SourceFilter::Online, ReplayPhrase::FilterOnline),
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
        None,
        Some(Outcome::Won),
        Some(Outcome::Lost),
        Some(Outcome::Draw),
        Some(Outcome::Unfinished),
    ];
    for result in results {
        let on = match ui.filters.result {
            ResultFilter::Any => result.is_none(),
            ResultFilter::Only(o) => result == Some(o),
        };
        let words = result.map_or(ReplayPhrase::AnyResult, ReplayPhrase::outcome);
        let chip = controls::chip(
            commands,
            kit,
            words.text(lang),
            on,
            None,
            false,
            Press::Replays(ReplaysPress::Result(result)),
        );
        commands.entity(bar).add_child(chip);
    }
    commands.entity(panel).add_child(bar);

    // ---- a deletion asked
    if let Some(i) = ui.delete {
        let ask = parts::row(commands, kit, true);
        let words = parts::line(
            commands,
            kit,
            ReplayPhrase::DeleteAsk.text(lang),
            kit.m.text,
            tokens::INK,
        );
        let yes = controls::button(
            commands,
            kit,
            ReplayPhrase::DeleteYes.text(lang),
            Weight::Danger,
            Live::Yes,
            None,
            Press::Replays(ReplaysPress::ConfirmDelete),
        );
        let no = controls::button(
            commands,
            kit,
            Phrase::SheetCancel.text(lang),
            Weight::Secondary,
            Live::Yes,
            None,
            Press::Replays(ReplaysPress::CancelDelete),
        );
        commands.entity(ask).add_children(&[words, yes, no]);
        commands.entity(panel).add_child(ask);
        let _ = i;
    }

    // ---- the list
    let scroll = commands
        .spawn((
            Node {
                width: percent(100),
                flex_grow: 1.0,
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px_fixed(kit.m.gap * 0.5),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            Scrollable(List::Replays),
            ScrollPosition(Vec2::new(0.0, scrolled.get(List::Replays))),
        ))
        .id();
    commands.entity(panel).add_child(scroll);
    let mut filters = ui.filters.clone();
    filters.query = lobby.field(Field::ReplaySearch).to_string();
    let names = |index: u32| card_name(index);
    let shown: Vec<(usize, &Entry)> = ui
        .library
        .entries()
        .iter()
        .enumerate()
        .filter(|(_, e)| filters.pass(e, &names))
        .collect();
    if ui.library.entries().is_empty() {
        let empty =
            crate::shellkit::surfaces::prose(commands, kit, ReplayPhrase::Empty.text(lang), true);
        commands.entity(scroll).add_child(empty);
    } else if shown.is_empty() {
        let none = parts::line(
            commands,
            kit,
            ReplayPhrase::NoMatch.text(lang),
            kit.m.text,
            tokens::MUTED,
        );
        commands.entity(scroll).add_child(none);
    }
    let offset = local_offset();
    let today = baylee_client_core::replay::library::day_of(parts::now_secs() * 1000, offset);
    let mut last_day = None;
    for (i, entry) in shown {
        let day = baylee_client_core::replay::library::day_of(entry.started, offset);
        if last_day != Some(day) {
            last_day = Some(day);
            let heading = day_heading(day, today, entry.started, offset, lang);
            let line = parts::line(commands, kit, &heading, kit.m.small, tokens::MUTED);
            commands.entity(scroll).add_child(line);
        }
        let row = entry_row(commands, root, kit, state, i, entry, offset, lang);
        commands.entity(scroll).add_child(row);
    }
    if ui.library.more_online() && ui.filters.source != SourceFilter::House {
        let older = controls::button(
            commands,
            kit,
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

/// One row: when, where, who, how it went, how long; Watch and `⋯`.
#[allow(clippy::too_many_arguments)]
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
    let place = match &entry.source {
        EntrySource::Local { .. } => {
            let house = entry
                .house
                .as_deref()
                .and_then(|level| baylee_client_core::i18n::ai_name(lang, level))
                .or(entry.house.as_deref())
                .unwrap_or_else(|| ReplayPhrase::House.text(lang));
            format!(
                "{} \u{b7} {}",
                ReplayPhrase::ThisComputer.text(lang),
                ReplayPhrase::VsHouse.fill(lang, &[house])
            )
        }
        EntrySource::Hosted { .. } => state
            .gateway_name()
            .unwrap_or_else(|| state.gateway.clone()),
    };
    let title = format!("{clock}  {place}");
    let mut meta = vec![players_line(entry, lang)];
    meta.push(ReplayPhrase::outcome(entry.result).text(lang).to_string());
    if let Some(turn) = entry.turns {
        meta.push(ReplayPhrase::TurnShort.fill(lang, &[&turn.to_string()]));
    }
    if let Some(minutes) = entry.minutes() {
        meta.push(ReplayPhrase::Minutes.fill(lang, &[&minutes.to_string()]));
    }
    match entry.check {
        EntryCheck::Exact => {}
        EntryCheck::Checked => meta.push(format!("\u{25d0} {}", ReplayPhrase::Checked.text(lang))),
        EntryCheck::Unverified => {
            meta.push(format!("\u{25cc} {}", ReplayPhrase::Unverified.text(lang)));
        }
    }
    let watch = controls::button(
        commands,
        kit,
        ReplayPhrase::Watch.text(lang),
        Weight::Secondary,
        Live::Yes,
        None,
        Press::Replays(ReplaysPress::Watch(i)),
    );
    let more = parts::icon_button(
        commands,
        kit,
        parts::ELLIPSIS,
        Press::Shared(SharedPress::OpenMenu(ShellMenu::Replay(i))),
    );
    if state.menu == Some(ShellMenu::Replay(i)) {
        let items = row_menu(i, entry, lang);
        menus::draw(
            commands,
            root,
            kit,
            more,
            items.iter().map(
                |(text, press, destructive)| crate::shellkit::surfaces::MenuItem {
                    text,
                    keys: None,
                    destructive: *destructive,
                    action: *press,
                },
            ),
        );
    }
    crate::shellkit::surfaces::list_row(
        commands,
        kit,
        &title,
        &meta.join(" \u{b7} "),
        &[watch, more],
        (
            Press::Replays(ReplaysPress::Watch(i)),
            crate::ambience::Feel::tinting_to(Color::NONE, tokens::HOVER),
        ),
    )
}

/// A row's `⋯` menu: Watch, Watch from ▸ each perspective the entry
/// offers, Delete… for a local record.
fn row_menu(i: usize, entry: &Entry, lang: Lang) -> Vec<(String, Press, bool)> {
    let seat = baylee_core::ids::PlayerId::new(entry.seat);
    let mut items = vec![(
        ReplayPhrase::Watch.text(lang).to_string(),
        Press::Replays(ReplaysPress::Watch(i)),
        false,
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
            ));
        }
    }
    if entry.deletable(None) {
        items.push((
            ReplayPhrase::Delete.text(lang).to_string(),
            Press::Replays(ReplaysPress::Delete(i)),
            true,
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
