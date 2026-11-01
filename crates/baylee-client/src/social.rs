//! Friends, presence and the end-to-end encrypted chat: the client's half
//! (the beta.8 social design §11, the chat window design).
//!
//! Everything that decides lives in [`baylee_client_core::social`]: the
//! [`Social`] machine is handed what arrives (HTTP answers, socket frames,
//! the link, the table, the clock, the player's keys and presses) and
//! answers with [`Effect`]s. This module carries those out and draws what
//! the machine holds. It never builds a frame or an HTTP body.
//!
//! - [`SocialHost`]: the machine, the transport, the secret store and the
//!   chat folder, for as long as the lobby is signed in at a gateway whose
//!   `/info` says it has chat. It persists across the lobby and the table.
//! - [`ChatUi`]: what the window shows (open, which tab, typing, the peek,
//!   the drawer, the sheets), per device.
//! - `window`: the one chat window, at the table in the log's place and in
//!   the lobby docked bottom-right; `drawer` the friends list; `toasts`;
//!   `sheet` the encryption notice, safety numbers and devices.
//!
//! Signed in nowhere, offline (`LocalHost`) or at a gateway with no chat,
//! nothing here is drawn and the table's log panel is today's.

use std::collections::VecDeque;

use baylee_client_core::social::{
    self, ChatCue, ChatTab, ChatToast, ConvId, Effect, Entropy, Intent, Me, Social, SocialLink,
    TableContext, ToastAbout,
};
use bevy::prelude::*;

use crate::lobby::LobbyState;

pub(crate) mod drawer;
pub(crate) mod glass;
pub(crate) mod keys;
pub(crate) mod menu;
pub(crate) mod rows;
pub(crate) mod sheet;
pub(crate) mod toasts;
mod transport;
pub(crate) mod vault_access;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod vault_ui;
pub(crate) mod window;
pub(crate) mod zoom;

use transport::{Arrived, LinkNews, Mailbox, Socket};

/// How long a changed state file waits before it is written (§11.1).
const SAVE_DEBOUNCE: f32 = 2.0;

/// How often the machine's clock is told the time (away, retries, lapsed
/// invites).
const TICK_SECS: f32 = 1.0;

/// The operating system's randomness, for the machine's keys and seals.
pub(crate) struct OsEntropy;

impl Entropy for OsEntropy {
    fn fill(&mut self, out: &mut [u8]) {
        // A machine with no randomness cannot make keys; failing loudly is
        // the only safe answer, and no supported platform does.
        getrandom::fill(out).expect("the operating system's randomness");
    }
}

/// Now, Unix milliseconds (`web_time`: `SystemTime::now` panics in a
/// browser).
#[must_use]
pub(crate) fn now_ms() -> u64 {
    web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// The signed-in account the chat speaks for.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Session {
    gateway: String,
    token: String,
    handle: String,
    tag: String,
    guest: bool,
}

/// Whether the gateway has chat.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum InfoState {
    #[default]
    Unknown,
    Asking,
    /// `Some((version, keep_days))`, or `None`: no chat here.
    Known(Option<(u32, u32)>),
}

/// Something the machine asked of the window, the lobby or the drawer.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum UiAsk {
    /// Show this conversation's tab.
    Open(ConvId),
    /// Join a table (an invite's Join).
    Join {
        game: String,
        invite: Option<String>,
    },
    /// Watch a table.
    Watch { game: String },
    /// Open the friends drawer.
    Drawer,
}

/// The chat's machine and everything that carries its effects out.
#[derive(Resource, Default)]
pub struct SocialHost {
    /// The machine, while a social session exists.
    pub(crate) machine: Option<Social>,
    session: Option<Session>,
    info: InfoState,
    mailbox: Mailbox,
    socket: Socket,
    store: Option<Box<dyn social::SecretStore>>,
    disk: Option<ChatHome>,
    /// The state file, waiting out its debounce.
    save: Option<(f32, Vec<u8>)>,
    /// Since the last clock tick.
    since_tick: f32,
    /// The last table the machine was told about, once it was told (the
    /// outer `Option`: told at all; the inner: at a table or not).
    #[allow(clippy::option_option)] // "never told" and "told: no table" are two states
    table: Option<Option<TableContext>>,
    /// Toasts to show, newest last.
    pub(crate) toasts: VecDeque<ChatToast>,
    /// What standing toasts asked about and is answered or gone since
    /// (`Effect::Withdraw`), for `toasts::take` to take away.
    pub(crate) withdrawn: Vec<ToastAbout>,
    /// Cues to play.
    pub(crate) sounds: Vec<ChatCue>,
    /// What the machine asked of the shell.
    pub(crate) asks: Vec<UiAsk>,
    /// Text for the clipboard.
    pub(crate) copy: Option<String>,
    /// Seconds since the last input, for auto-away.
    idle: f32,
    /// The keep-days the gateway said, for the notice.
    keep_days: u32,
    /// Password vault lifecycle; no OS store is opened on startup.
    vault: vault_access::Access,
}

impl SocialHost {
    /// The machine, when the chat is up.
    #[must_use]
    pub fn machine(&self) -> Option<&Social> {
        self.machine.as_ref()
    }

    /// Whether a social session exists (the chat is drawn).
    #[must_use]
    pub fn active(&self) -> bool {
        self.machine.is_some()
    }

    /// Hands the machine an intent and carries out what it answers.
    pub(crate) fn act(&mut self, intent: Intent) {
        #[cfg(not(target_arch = "wasm32"))]
        if self.vault.is_central() && matches!(&intent, Intent::RemoveDevice { .. }) {
            // Gateway-only removal cannot revoke Central's archived identity or
            // recipient whitelist. This needs one account-wide retirement flow.
            return;
        }
        // This cache survives a manual lock and seeds the next unlock job.
        // Preference changes must reach it even while no machine is present.
        match &intent {
            Intent::SetPrefs(prefs) => self.vault.prefs.clone_from(prefs),
            Intent::NoticeSeen => self.vault.prefs.notice_seen = true,
            _ => {}
        }
        let Some(machine) = self.machine.as_mut() else {
            return;
        };
        let effects = machine.act(intent, now_ms(), &mut OsEntropy);
        self.carry_out(effects);
    }

    /// Applies account preference updates, including while the vault is locked.
    fn sync_prefs(&mut self, prefs: &social::ChatPrefs) {
        if self.vault.prefs != *prefs {
            self.act(Intent::SetPrefs(prefs.clone()));
        }
    }

    /// Carries out the machine's effects.
    pub(crate) fn carry_out(&mut self, effects: Vec<Effect>) {
        #[cfg(not(target_arch = "wasm32"))]
        let snapshot = self.vault.snapshot_owner();
        #[cfg(not(target_arch = "wasm32"))]
        let _batch = snapshot.as_ref().and_then(|sync| sync.begin_batch());
        for effect in effects {
            self.carry_out_one(effect);
        }
    }

    fn carry_out_one(&mut self, effect: Effect) {
        let Some(session) = self.session.clone() else {
            return;
        };
        match effect {
            Effect::Http {
                tag,
                method,
                path,
                body,
            } => transport::request(
                &self.mailbox,
                &session.gateway,
                &session.token,
                tag,
                method,
                &path,
                body.as_ref(),
            ),
            Effect::Up(up) => {
                #[cfg(not(target_arch = "wasm32"))]
                if let baylee_protocol::social::Up::Ack { upto } = &up
                    && self.vault.defer_ack(*upto)
                {
                    return;
                }
                if let Ok(text) = serde_json::to_string(&up)
                    && !self.socket.send(text)
                    && let Some(machine) = self.machine.as_mut()
                {
                    // No socket: the machine keeps the send pending and
                    // goes again once the link is back.
                    let _ = machine.on_link(SocialLink::Down, now_ms());
                }
            }
            Effect::Dial => self.socket.want(),
            Effect::Hang => self.socket.hang(),
            Effect::StoreSecrets(value) => {
                if let (Some(store), Some(machine)) = (self.store.as_ref(), self.machine.as_ref())
                    && let Err(why) = store.store(&machine.secret_slot(), value.expose())
                {
                    warn!("the chat's keys could not be kept: {why}");
                }
            }
            Effect::ForgetSecrets => {
                if let (Some(store), Some(machine)) = (self.store.take(), self.machine.as_ref()) {
                    let slot = machine.secret_slot();
                    #[cfg(not(target_arch = "wasm32"))]
                    std::thread::spawn(move || {
                        let _ = store.forget(&slot);
                    });
                    #[cfg(target_arch = "wasm32")]
                    let _ = store.forget(&slot);
                }
            }
            Effect::SaveState(bytes) => {
                #[cfg(not(target_arch = "wasm32"))]
                if self.vault.is_central() {
                    // Central's shutdown drain must see the latest conversation map even
                    // when macOS quits by dropping the World without an AppExit message.
                    if let Some(disk) = self.disk.as_mut() {
                        if disk.write(social::disk::STATE_FILE, &bytes).is_err() {
                            self.vault.error = Some(vault_access::Error::Io);
                        }
                        self.save = None;
                    }
                    return;
                }
                self.save = Some((SAVE_DEBOUNCE, bytes));
            }
            effect @ (Effect::AppendHistory { .. }
            | Effect::RewriteHistory { .. }
            | Effect::DeleteAllLocal) => {
                if matches!(effect, Effect::DeleteAllLocal) {
                    self.save = None;
                }
                if let Some(disk) = self.disk.as_mut()
                    && let Some(Err(why)) = social::disk::apply(&effect, &mut **disk)
                {
                    warn!("the chat's history could not be written: {why}");
                }
            }
            Effect::Toast(toast) => {
                self.toasts.push_back(toast);
                while self.toasts.len() > 6 {
                    self.toasts.pop_front();
                }
            }
            Effect::Withdraw(about) => {
                self.toasts
                    .retain(|toast| toast.about.as_ref() != Some(&about));
                self.withdrawn.push(about);
            }
            Effect::Sound(cue) => self.sounds.push(cue),
            Effect::OpenConversation(conv) => self.asks.push(UiAsk::Open(conv)),
            Effect::JoinTable { game, invite } => self.asks.push(UiAsk::Join { game, invite }),
            Effect::WatchTable { game } => self.asks.push(UiAsk::Watch { game }),
            Effect::OpenDrawer => self.asks.push(UiAsk::Drawer),
            Effect::Copy(text) => self.copy = Some(text),
        }
    }

    /// Ends the session: the socket closes, the state file is written.
    fn end(&mut self) {
        self.flush_save();
        self.socket.hang();
        self.socket = Socket::default();
        self.machine = None;
        self.store = None;
        self.disk = None;
        self.vault = vault_access::Access::default();
        self.copy = None;
        self.sounds.clear();
        self.session = None;
        self.info = InfoState::Unknown;
        self.mailbox = Mailbox::default();
        self.table = None;
        self.toasts.clear();
        self.asks.clear();
    }

    fn flush_save(&mut self) {
        if let (Some((_, bytes)), Some(disk)) = (self.save.take(), self.disk.as_mut())
            && let Err(why) = disk.write(social::disk::STATE_FILE, &bytes)
        {
            warn!("the chat's state could not be written: {why}");
        }
    }

    /// Starts only the memory-only guest/browser session here. Native account
    /// vaults are prepared off the render thread by `vault_access`.
    fn start(&mut self, keep_days: u32, prefs: social::ChatPrefs) {
        self.keep_days = keep_days;
        self.vault.prefs = prefs;
        let Some(session) = self.session.clone() else {
            return;
        };
        if cfg!(not(target_arch = "wasm32")) && !session.guest {
            return;
        }
        let disk: ChatHome = Box::new(social::disk::MemoryDisk::new());
        let store = Box::new(social::vault::MemorySecrets::new());
        let me = Me {
            tag: session.tag,
            handle: session.handle,
            guest: session.guest,
            gateway: session.gateway,
        };
        let (mut machine, effects) = Social::start(
            me,
            None,
            None,
            self.vault.prefs.clone(),
            now_ms(),
            &mut OsEntropy,
        );
        machine.set_key_home(social::KeyHome::Memory);
        machine.set_keep_days(keep_days);
        self.store = Some(store);
        self.disk = Some(disk);
        self.machine = Some(machine);
        self.carry_out(effects);
    }
}

/// The chat's disk, behind the one interface every platform's has.
type ChatHome = Box<dyn social::disk::ChatDisk + Send + Sync>;

/// The client's config directory, where the chat folder lives; `None` in a
/// process that never opened the store (a test).
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn config_dir() -> Option<std::path::PathBuf> {
    use baylee_client_core::userdirs::{Kind, Os, real_env, user_dir};
    if !crate::settings::store_is_open() {
        return None;
    }
    user_dir(Kind::Config, Os::current(), &real_env)
}

/// What the chat window shows, per device and per session.
#[derive(Resource, Debug)]
#[allow(clippy::struct_excessive_bools)] // independent switches of one window
pub struct ChatUi {
    /// Whether the panel stands (the table: state 2 or 3; the lobby: not
    /// hidden to the pill).
    pub open: bool,
    /// Native password-vault sheet holds input independently of chat.
    pub(crate) vault_open: bool,
    /// The tab shown.
    pub tab: ChatTab,
    /// The last chat tab used (where the chat door lands).
    pub last_chat_tab: Option<ChatTab>,
    /// The composer has the caret: every key is the chat's (state 3).
    pub typing: bool,
    /// The frame the composer let go of the keys: they are swallowed so the
    /// `Esc` or `Enter` that closed it does not reach the table.
    pub swallow: bool,
    /// The lobby panel is minimised to its tab row.
    pub minimised: bool,
    /// The group the Group tab shows, by id.
    pub shown_group: Option<String>,
    /// The friends drawer stands.
    pub drawer: bool,
    /// The sheet over everything, when one is up.
    pub sheet: Option<sheet::Sheet>,
    /// The filter dialog for this tab, when up.
    pub filters: Option<ChatTab>,
    /// The group dropdown is open.
    pub group_menu: bool,
    /// The target chip's menu is open.
    pub chip_menu: bool,
    /// Peek lines at the table: the line's text and when it arrived.
    pub peek: Vec<(String, f32)>,
    /// A line's row menu, by its key.
    pub row_menu: Option<social::LineKey>,
    /// The drawer's open `⋯` menu: a friend's tag, or `grp:<id>`.
    pub drawer_menu: Option<String>,
    /// Forget this device was pressed once: the next press confirms.
    pub forget_armed: bool,
    /// A hint the window shows for a moment (a private card refused).
    pub hint: Option<(social::ChatPhrase, f32)>,
    /// The drawer's search/add box.
    pub find: baylee_client_core::textbuf::TextBuffer,
    /// A small text box elsewhere (new group's name, a nickname) and what
    /// it is for.
    pub entry: Option<(Entry, baylee_client_core::textbuf::TextBuffer)>,
    /// Which text box has the keys: the composer, the drawer's search or
    /// the entry.
    pub field: Field,
    /// The list's scroll is asked to follow its end.
    pub follow: bool,
    /// A game is open (the table's frame, the Log tab).
    pub at_table: bool,
    /// The selection in the history, for copy.
    pub selection: Option<social::Selection>,
    /// The first line that was unread when the shown tab was last opened:
    /// the history draws its "New" divider above it until the tab goes.
    pub marker: Option<(ChatTab, social::LineKey)>,
    /// The All tab's arrival stamps: chat lines by key, log lines by the
    /// book's length when they arrived.
    pub(crate) arrivals: Arrivals,
}

impl Default for ChatUi {
    fn default() -> Self {
        Self {
            open: false,
            vault_open: false,
            tab: ChatTab::Log,
            last_chat_tab: None,
            typing: false,
            swallow: false,
            minimised: false,
            shown_group: None,
            drawer: false,
            sheet: None,
            filters: None,
            group_menu: false,
            chip_menu: false,
            peek: Vec::new(),
            row_menu: None,
            drawer_menu: None,
            forget_armed: false,
            hint: None,
            find: baylee_client_core::textbuf::TextBuffer::default(),
            entry: None,
            field: Field::Composer,
            follow: true,
            at_table: false,
            selection: None,
            marker: None,
            arrivals: Arrivals::default(),
        }
    }
}

impl ChatUi {
    /// Whether the chat holds every key this frame (the composer or one of
    /// the chat's boxes has the caret, or let go of it this frame).
    #[must_use]
    pub fn holds_keyboard(&self) -> bool {
        self.typing || self.swallow || self.vault_open
    }
}

/// Which of the chat's text boxes has the keys while [`ChatUi::typing`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Field {
    /// The composer.
    #[default]
    Composer,
    /// The friends drawer's search and add box.
    Find,
    /// [`ChatUi::entry`].
    Entry,
}

/// What a small entry box is for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    /// A new group's name.
    NewGroup,
    /// A group's new name.
    Rename(String),
    /// A nickname for a tag.
    Nickname(String),
    /// A friend to invite to a group, by handle.
    InviteToGroup(String),
}

/// A count in the lobby header's chat buttons (the chat window design
/// §4.3), written by the chat without rebuilding the header.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeaderCount {
    /// The chat button's unread count: `· 3`.
    Chat,
    /// The friends button: the friends online, and `•` while something waits.
    Friends,
}

/// The header counts as last written, so a header the lobby draws again
/// starts with them rather than empty for a frame (the flicker of the
/// chat and friends buttons whenever the lobby redrew its header).
static HEADER_COUNTS: std::sync::RwLock<(String, String)> =
    std::sync::RwLock::new((String::new(), String::new()));

/// What a header count says now.
pub(crate) fn header_count(count: HeaderCount) -> String {
    let counts = HEADER_COUNTS
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    match count {
        HeaderCount::Chat => counts.0.clone(),
        HeaderCount::Friends => counts.1.clone(),
    }
}

/// What a header button (or the settings row) asked of the chat.
pub(crate) fn header_ask(host: &mut SocialHost, ui: &mut ChatUi, ask: crate::lobby::ShellAsk) {
    use crate::lobby::ShellAsk;
    if !host.active() {
        return;
    }
    match ask {
        ShellAsk::Chat => {
            if ui.open && !ui.minimised {
                ui.open = false;
                ui.typing = false;
            } else {
                window::open_on_chat(host, ui, None);
                ui.typing = true;
                ui.field = Field::Composer;
            }
        }
        ShellAsk::Friends => ui.drawer = !ui.drawer,
        ShellAsk::ChatPrivacy => ui.sheet = Some(sheet::Sheet::Privacy),
        ShellAsk::Shortcuts | ShellAsk::Report | ShellAsk::DismissToast(_) | ShellAsk::Quit => {}
    }
}

/// The lobby's side of the chat: whether the header draws its buttons, the
/// counts on them, and the shell's `f` and `Enter`.
#[allow(clippy::too_many_arguments)] // a Bevy system: what it reads and writes
fn lobby_side(
    mut lobby: ResMut<LobbyState>,
    mut host: ResMut<SocialHost>,
    mut ui: ResMut<ChatUi>,
    mut counts: Query<(&HeaderCount, &mut Text, Option<&ChildOf>)>,
    mut badges: Query<&mut Visibility, With<crate::shellkit::role::Role>>,
    mut fired: MessageReader<crate::shellkit::keys::ShellFired>,
    mut typed: MessageReader<bevy::input::keyboard::KeyboardInput>,
    stack: Option<Res<crate::shellkit::keys::ShellStack>>,
    focus: Option<Res<bevy::input_focus::InputFocusVisible>>,
    phase: Option<Res<State<crate::DuelPhase>>>,
) {
    use baylee_client_core::shellkeys::ShellAction;
    let header = host.machine().map(|m| !m.me().guest);
    if lobby.chat_header != header {
        // The friends list stands open from the start of a session (owner,
        // 13.10.); a guest has none.
        if lobby.chat_header.is_none() && header == Some(true) {
            ui.drawer = true;
        }
        lobby.chat_header = header;
    }
    let at_table = phase.is_some_and(|p| {
        matches!(
            *p.get(),
            crate::DuelPhase::Playing | crate::DuelPhase::Finished
        )
    });
    let asked: Vec<ShellAction> = fired.read().map(|f| f.0).collect();
    let entered = typed.read().any(|k| {
        k.state.is_pressed() && !k.repeat && k.logical_key == bevy::input::keyboard::Key::Enter
    });
    if !at_table && !ui.holds_keyboard() {
        for action in asked {
            if action == ShellAction::Friends && header == Some(true) {
                ui.drawer = !ui.drawer;
            }
        }
        // Enter opens the chat (§12.1) where nothing else would take it: no
        // field has the caret, no sheet or menu stands, no focus ring shows
        // (the walker's Enter is that control's), on a shell screen.
        let free = stack.as_ref().is_some_and(|s| {
            s.live && !s.stack.field && !s.stack.modal && !s.stack.menu && !s.stack.typeahead
        });
        if entered && free && header.is_some() && !focus.as_ref().is_some_and(|f| f.0) {
            header_ask(&mut host, &mut ui, crate::lobby::ShellAsk::Chat);
            ui.swallow = true;
        }
    }
    let Some(machine) = host.machine() else {
        return;
    };
    let (unread, _) = machine.unread_where(&social::TabFilter::standard(ChatTab::All));
    let chat = crate::shellkit::header::format_badge_count(unread as usize);
    let online = machine.contacts().online_count();
    let friends = if machine.contacts().waiting() {
        format!("{online} \u{2022}")
    } else {
        online.to_string()
    };
    {
        let mut stored = HEADER_COUNTS
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if stored.0 != chat || stored.1 != friends {
            *stored = (chat.clone(), friends.clone());
        }
    }
    for (count, mut text, parent) in &mut counts {
        let want = match count {
            HeaderCount::Chat => &chat,
            HeaderCount::Friends => &friends,
        };
        if text.0 != *want {
            text.0.clone_from(want);
        }
        if matches!(count, HeaderCount::Chat)
            && let Some(parent) = parent
            && let Ok(mut vis) = badges.get_mut(parent.parent())
        {
            vis.set_if_neq(if want.is_empty() {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            });
        }
    }
}

/// The order things arrived in, for the All tab's merge (§5): never sorted
/// by time, which the host's and the gateway's clocks would disagree on.
#[derive(Debug, Default)]
pub(crate) struct Arrivals {
    next: u64,
    /// Chat lines by key.
    pub(crate) chat: std::collections::HashMap<social::LineKey, u64>,
    /// `(first line index, stamp)`: every log line from that index on (to
    /// the next mark) arrived with that stamp.
    pub(crate) log: Vec<(usize, u64)>,
    /// How long the book was when last looked at.
    log_len: usize,
}

impl Arrivals {
    /// Notes the log's length; new lines get a stamp.
    pub(crate) fn log_grew(&mut self, len: usize) {
        if len < self.log_len {
            // Rewritten (a reconnect replays it): one stamp for all of it.
            self.log.clear();
            self.log_len = 0;
        }
        if len > self.log_len {
            self.next += 1;
            self.log.push((self.log_len, self.next));
            self.log_len = len;
        }
    }

    /// Notes a chat line; a new one gets a stamp.
    pub(crate) fn chat_line(&mut self, key: &social::LineKey) -> u64 {
        if let Some(stamp) = self.chat.get(key) {
            return *stamp;
        }
        self.next += 1;
        self.chat.insert(key.clone(), self.next);
        self.next
    }

    /// The stamp of log line `index`.
    pub(crate) fn log_stamp(&self, index: usize) -> u64 {
        self.log
            .iter()
            .rev()
            .find(|(from, _)| *from <= index)
            .map_or(0, |(_, stamp)| *stamp)
    }
}

/// Installs the chat: the machine's driver, the window, the drawer, the
/// toasts and the sheets.
pub(crate) fn install(app: &mut App) {
    if app.world().contains_resource::<SocialHost>() {
        return;
    }
    app.init_resource::<SocialHost>()
        .init_resource::<ChatUi>()
        .init_resource::<zoom::Zoom>()
        .add_systems(Update, follow_the_zoom)
        .add_systems(
            Update,
            (
                drive.run_if(resource_exists::<LobbyState>),
                table_context.run_if(resource_exists::<LobbyState>),
                lobby_side.run_if(resource_exists::<LobbyState>),
            )
                .chain(),
        );
    window::install(app);
    glass::install(app);
    keys::install(app);
    menu::install(app);
    drawer::install(app);
    toasts::install(app);
    sheet::install(app);
    #[cfg(not(target_arch = "wasm32"))]
    vault_ui::install(app);
}

/// The lobby's chat grows with the shell past 1920 × 1080 and with the text
/// step; at the table the HUD's `UiScale` grows it, so the zoom is 1 there.
/// A change takes the lobby's chat surfaces down to be drawn again at it.
#[allow(clippy::type_complexity)] // the surfaces drawn at the zoom
fn follow_the_zoom(
    mut commands: Commands,
    mut zoom: ResMut<zoom::Zoom>,
    (windows, ui): (
        Query<&Window, With<bevy::window::PrimaryWindow>>,
        Option<Res<UiScale>>,
    ),
    settings: Option<Res<crate::settings::ClientSettings>>,
    phase: Option<Res<State<crate::DuelPhase>>>,
    roots: Query<
        Entity,
        Or<(
            With<window::LobbyChat>,
            With<drawer::Drawer>,
            With<drawer::FriendsDoor>,
            With<toasts::ToastColumn>,
            With<sheet::SheetRoot>,
            With<menu::RowMenu>,
            With<window::FilterDialog>,
        )>,
    >,
) {
    let at_table = phase.is_some_and(|p| *p.get() != crate::DuelPhase::Closed);
    let now = if at_table {
        1.0
    } else {
        let step = settings.as_ref().map(|s| s.text_size).unwrap_or_default();
        windows.single().map_or(1.0, |w| {
            crate::shellkit::metrics::screen_scale(
                crate::shellkit::Viewport::desktop(
                    crate::hud::scale::desktop_space(w, ui.as_deref()).x,
                    crate::hud::scale::desktop_space(w, ui.as_deref()).y,
                ),
                step,
            ) * step.factor()
        })
    };
    if (zoom.0 - now).abs() > f32::EPSILON {
        zoom.0 = now;
        for root in &roots {
            commands.entity(root).despawn();
        }
    }
}

/// The session the lobby is signed in to, if any.
fn wanted_session(lobby: &LobbyState) -> Option<Session> {
    let token = lobby.lobby.token()?;
    if lobby.lobby.offline() {
        return None;
    }
    let me = lobby.lobby.me()?;
    let tag = social::tag_of(&me.handle)?.to_string();
    Some(Session {
        gateway: lobby.gateway.trim_end_matches('/').to_string(),
        token: token.to_string(),
        handle: me.handle.clone(),
        tag,
        guest: me.guest,
    })
}

/// Drives the machine: the session, the gateway's answers, the socket, the
/// clock, the debounced state file.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // a Bevy system: each input in turn
fn drive(
    time: Res<Time>,
    mut lobby: ResMut<LobbyState>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut host: ResMut<SocialHost>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    windows: Query<&Window>,
) {
    let host = &mut *host;
    let wanted = wanted_session(&lobby);
    if host.session != wanted {
        if host.session.is_some() {
            host.end();
        }
        host.session = wanted;
    }
    if let Some(prefs) = prefs.as_deref() {
        host.sync_prefs(&prefs.all().chat);
    }
    // Vault credentials are an invisible one-shot handoff. Mutating only that
    // field must not flag visible lobby state and rebuild its UI every frame.
    vault_access::discard_stale_login(lobby.bypass_change_detection());
    let Some(session) = host.session.clone() else {
        return;
    };
    if host.info == InfoState::Unknown {
        host.info = InfoState::Asking;
        transport::ask_info(&host.mailbox, &session.gateway);
    }
    let dt = time.delta_secs();
    let now = now_ms();
    let mut news = Vec::new();
    for arrived in host.mailbox.drain() {
        match arrived {
            Arrived::Info(social) => {
                host.info = InfoState::Known(social);
                if let Some((version, keep)) = social
                    && version == baylee_protocol::social::SOCIAL_VERSION
                {
                    let chat = prefs
                        .as_deref()
                        .map(|p| p.all().chat.clone())
                        .unwrap_or_default();
                    host.start(keep, chat);
                }
            }
            Arrived::Http { tag, status, body } => {
                if let Some(machine) = host.machine.as_mut() {
                    let effects = machine.on_http(tag, status, &body, now, &mut OsEntropy);
                    host.carry_out(effects);
                }
            }
            Arrived::Ticket(status, body) => {
                host.socket.ticket(
                    status,
                    &body,
                    &session.gateway,
                    &session.token,
                    &host.mailbox,
                    &mut news,
                );
            }
        }
    }
    vault_access::follow(host, lobby.bypass_change_detection());
    if host.machine.is_none() {
        return;
    }
    host.socket.pump(
        dt,
        &session.gateway,
        &session.token,
        &host.mailbox,
        &mut news,
    );
    for item in news {
        let Some(machine) = host.machine.as_mut() else {
            break;
        };
        let effects = match item {
            LinkNews::Up => machine.on_link(SocialLink::Up, now),
            LinkNews::Down => machine.on_link(SocialLink::Down, now),
            LinkNews::Refused | LinkNews::Unauthorized => machine.on_link(SocialLink::Refused, now),
            LinkNews::Frame(text) => {
                if let Ok(frame) = serde_json::from_str::<baylee_protocol::social::Down>(&text) {
                    machine.on_frame(frame, now, &mut OsEntropy)
                } else {
                    debug!("the social socket sent something unreadable");
                    Vec::new()
                }
            }
        };
        host.carry_out(effects);
    }
    // Input resets the idle clock (auto-away after ten minutes, §6).
    if keys.get_just_pressed().next().is_some()
        || mouse.is_some_and(|m| m.get_just_pressed().next().is_some())
    {
        host.idle = 0.0;
    } else {
        host.idle += dt;
    }
    host.since_tick += dt;
    if host.since_tick >= TICK_SECS {
        host.since_tick = 0.0;
        let idle_ms = std::time::Duration::from_secs_f32(host.idle.max(0.0))
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX);
        let focused = windows.iter().any(|w| w.focused);
        if let Some(machine) = host.machine.as_mut() {
            machine.set_focus(focused);
            let effects = machine.tick(now, idle_ms);
            host.carry_out(effects);
        }
    }
    if let Some(machine) = host.machine.as_mut()
        && machine.wants_entropy()
    {
        let effects = machine.flush(now, &mut OsEntropy);
        host.carry_out(effects);
    }
    if let Some((left, _)) = host.save.as_mut() {
        *left -= dt;
        if *left <= 0.0 {
            host.flush_save();
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(upto) = host.vault.ready_ack()
        && let Ok(text) = serde_json::to_string(&baylee_protocol::social::Up::Ack { upto })
        && host.socket.send(text)
    {
        host.vault.sent_ack(upto);
    }
}

/// Tells the machine where the player sits: a waiting room, a table being
/// played or watched (from the lobby's seat and its listing), or nowhere.
/// Told only when it changes.
fn table_context(
    lobby: Res<LobbyState>,
    duel: Option<Res<crate::Duel>>,
    phase: Option<Res<State<crate::DuelPhase>>>,
    mut host: ResMut<SocialHost>,
) {
    if host.machine.is_none() {
        return;
    }
    let at_table = phase.is_some_and(|p| *p.get() != crate::DuelPhase::Closed);
    let next = context_of(&lobby, duel.as_deref().filter(|_| at_table), at_table);
    if host.table.as_ref() == Some(&next) {
        return;
    }
    host.table = Some(next.clone());
    if let Some(machine) = host.machine.as_mut() {
        let effects = machine.on_table(next);
        host.carry_out(effects);
    }
}

/// The table the lobby says the player is at.
fn context_of(
    lobby: &LobbyState,
    duel: Option<&crate::Duel>,
    at_table: bool,
) -> Option<TableContext> {
    use baylee_client_core::lobby::{Screen, SeatHandover};
    let (game_id, watching) = match lobby.lobby.screen() {
        Screen::Seated(handover) if !handover.local => (
            handover.game_id.clone(),
            handover.seat == SeatHandover::WATCH,
        ),
        _ => {
            let room = baylee_client_core::lobby::strips::my_waiting_table(&lobby.lobby)?;
            (room.id.clone(), false)
        }
    };
    let summary = lobby.lobby.games().iter().find(|g| g.id == game_id);
    let mut roster: Vec<social::RosterSeat> = summary
        .map(|g| {
            g.seats
                .iter()
                .filter_map(|s| {
                    Some(social::RosterSeat {
                        seat: u8::try_from(s.seat).ok()?,
                        handle: s.player.clone()?,
                        team: s.team,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    // At the table the roster is the game's own, which a listing that stopped
    // updating when the seat was taken may lag.
    if let Some(statics) = duel.and_then(|d| d.statics.as_ref()) {
        roster = statics
            .seats
            .iter()
            .map(|s| social::RosterSeat {
                seat: s.player.get(),
                handle: s.display_name.clone(),
                team: s.team,
            })
            .collect();
    }
    let me_seat = if watching {
        None
    } else {
        duel.and_then(|d| d.view.as_ref())
            .map(|v| v.seat.get())
            .or_else(|| {
                summary
                    .and_then(baylee_client_core::lobby::GameSummary::my_seat)
                    .and_then(|s| u8::try_from(s).ok())
            })
    };
    let my_team =
        me_seat.and_then(|seat| roster.iter().find(|s| s.seat == seat).and_then(|s| s.team));
    Some(TableContext {
        game: game_id,
        playing: at_table || summary.is_some_and(|g| g.state == "playing"),
        roster,
        me_seat,
        my_team,
        watching,
    })
}

/// What `/state.chat` says (the chat window design §13).
#[cfg(all(feature = "dev-control", not(target_arch = "wasm32")))]
pub(crate) fn state_json(host: &SocialHost, ui: &ChatUi) -> serde_json::Value {
    let Some(machine) = host.machine.as_ref() else {
        return serde_json::json!({ "active": false, "info": format!("{:?}", host.info) });
    };
    let prefs = machine.prefs();
    let tabs: Vec<serde_json::Value> = window::drawn_tabs(machine, ui.at_table)
        .into_iter()
        .map(|tab| {
            let (unread, urgent) = if tab.is_chat() {
                machine.unread_where(&prefs.filter(tab))
            } else {
                (0, false)
            };
            serde_json::json!({ "name": tab.name(), "unread": unread, "urgent": urgent })
        })
        .collect();
    let composer = machine.composer();
    let friends = machine.friends();
    serde_json::json!({
        "active": true,
        "online": machine.online(),
        "me": machine.me().handle,
        "device": machine.device_id(),
        "key_home": format!("{:?}", machine.encryption_note().home),
        "open": ui.open,
        "tab": ui.tab.name(),
        "tabs": tabs,
        "typing": ui.typing,
        "field": format!("{:?}", ui.field),
        "drawer": ui.drawer,
        "sheet": ui.sheet.as_ref().map(|s| format!("{s:?}")),
        "group_shown": ui.shown_group,
        "composer": {
            "target": format!("{:?}", composer.target()),
            "text": composer.text(),
            "suggesting": composer.suggestions().map(|s| s.items.len()),
            "cards": composer.candidates().cards.len(),
            "players": composer.candidates().players.len(),
        },
        "peek": ui.peek.iter().map(|(t, _)| t.clone()).collect::<Vec<_>>(),
        "friends": friends.online.iter().chain(friends.offline.iter()).map(|f| serde_json::json!({
            "handle": f.person.handle,
            "status": format!("{:?}", f.presence.status),
            "place": f.presence.place.as_ref().map(|p| format!("{p:?}")),
        })).collect::<Vec<_>>(),
        "incoming": friends.incoming.iter().map(|p| p.handle.clone()).collect::<Vec<_>>(),
        "outgoing": friends.outgoing.iter().map(|p| p.handle.clone()).collect::<Vec<_>>(),
        "groups": machine.contacts().groups.iter().map(|g| serde_json::json!({"id": g.id, "name": g.name, "members": g.members.len()})).collect::<Vec<_>>(),
        "invites": machine.contacts().invites.iter().map(|i| serde_json::json!({"id": i.invite.id, "from": i.invite.from.handle, "game": i.invite.game})).collect::<Vec<_>>(),
        "conversations": machine.conversations().iter().map(|c| serde_json::json!({
            "id": c.id.to_string(),
            "title": c.title,
            "unread": c.unread,
            "lines": c.lines.iter().rev().take(20).rev().map(|l| rows::plain_text(machine, &c.id, l)).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "toasts": host.toasts.len(),
        "table": machine.table().map(|t| serde_json::json!({"game": t.game, "playing": t.playing, "watching": t.watching, "me_seat": t.me_seat})),
        "key_home_file": matches!(machine.encryption_note().home, social::KeyHome::File),
    })
}

#[cfg(test)]
mod tests;
