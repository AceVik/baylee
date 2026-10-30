//! Friends, presence and end-to-end encrypted chat: the client's brain
//! (the beta.8 social design, §10).
//!
//! [`Social`] is a pure state machine, tested without a renderer the way
//! [`crate::lobby::Lobby`] is. It answers inputs (HTTP answers, socket
//! frames, the link going up or down, the table the player sits at, the
//! clock, the player's intents and keys) with [`Effect`]s the client carries
//! out. It never does I/O, never reads a clock and never draws randomness:
//! `now_ms` and an [`Entropy`] are handed in.
//!
//! The client never builds a frame or an HTTP body: everything that leaves
//! goes out as an [`Effect`]. Every message leaves sealed
//! ([`sealing::seal_for`]), every record on disk is sealed under the
//! device's vault key, and the device's secrets leave only as
//! [`Effect::StoreSecrets`] for the OS credential store.

pub mod commands;
pub mod compose;
pub mod contacts;
pub mod convs;
pub mod disk;
pub mod history;
pub mod keys;
pub mod notify;
pub mod sealing;
pub mod select;
pub mod tabs;
pub mod text;
pub mod vault;
pub mod words;

mod machine;
mod table;
#[cfg(test)]
mod tests;

pub use baylee_seal::Entropy;
pub use commands::{Command, CommandFault, GroupRef, Target};
pub use compose::{
    ComposeContext, Composer, ComposerKey, ComposerOutcome, LinkedCard, Suggest, SuggestItem,
};
pub use contacts::{Contacts, FriendsView, HeldInvite};
pub use convs::{
    Body, BodyLink, BodyMention, BodyPrint, ChatLine, ConvId, ConvKind, Conversation, LineKey,
    LineKind, LineState,
};
pub use disk::{ChatDisk, MemoryDisk};
pub use machine::{ArchiveError, Social};
pub use notify::{ChatCue, Notify, NotifyContext};
pub use select::{DrawnLine, Pos, Selection, copy_text};
pub use table::{RosterSeat, TableContext, tag_of};
pub use tabs::{ChatTab, GroupPick, TabFilter};
pub use vault::{KeyHome, MemorySecrets, SecretSlot, SecretStore};
pub use words::ChatPhrase;

use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use baylee_protocol::social::{SocialSettings, Status, Up};

/// The signed-in account the machine speaks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Me {
    /// The account's tag (`af03`).
    pub tag: String,
    /// The account's handle (`Alice#af03`).
    pub handle: String,
    /// A guest: table chat and whispers at the table only.
    pub guest: bool,
    /// The gateway's base URL.
    pub gateway: String,
}

/// The player's chat settings on this device (§11.8).
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ChatPrefs {
    /// Days of history kept: 0 (off), 7, 30 or 365.
    pub history_days: u16,
    /// Timestamps on lines.
    pub timestamps: bool,
    /// Which kinds play a sound.
    pub sounds: ChatSounds,
    /// Toasts show the message's text.
    pub toast_text: bool,
    /// The encryption notice was shown and answered "Got it".
    pub notice_seen: bool,
    /// What each fixed tab shows, in [`ChatTab::ALL`]'s order (the chat
    /// window design §3.5). A stored list of another length is read as
    /// far as it goes; [`ChatPrefs::filter`] fills the rest.
    pub tabs: Vec<TabFilter>,
}

impl Default for ChatPrefs {
    fn default() -> Self {
        Self {
            history_days: 30,
            timestamps: true,
            sounds: ChatSounds::default(),
            toast_text: true,
            notice_seen: false,
            tabs: TabFilter::standard_set(),
        }
    }
}

impl ChatPrefs {
    /// The history lengths the settings offer.
    pub const HISTORY_DAYS: [u16; 4] = [0, 7, 30, 365];

    /// What `tab` shows: the stored filter, or the tab's standard one.
    #[must_use]
    pub fn filter(&self, tab: ChatTab) -> TabFilter {
        self.tabs
            .get(tab.index())
            .cloned()
            .unwrap_or_else(|| TabFilter::standard(tab))
    }

    /// Sets what `tab` shows.
    pub fn set_filter(&mut self, tab: ChatTab, filter: TabFilter) {
        while self.tabs.len() <= tab.index() {
            let next = ChatTab::ALL[self.tabs.len()];
            self.tabs.push(TabFilter::standard(next));
        }
        self.tabs[tab.index()] = filter;
    }
}

/// Which kinds of line play a sound.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ChatSounds {
    /// A whisper.
    pub whisper: bool,
    /// A mention of me.
    pub mention: bool,
    /// A group line while the window has no focus.
    pub group: bool,
    /// A table or team line.
    pub table: bool,
    /// A friend coming online.
    pub friend_online: bool,
}

impl Default for ChatSounds {
    fn default() -> Self {
        Self {
            whisper: true,
            mention: true,
            group: true,
            table: false,
            friend_online: false,
        }
    }
}

/// The social socket's state, as the transport reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SocialLink {
    /// The socket opened.
    Up,
    /// It closed or failed; the transport retries on its schedule.
    Down,
    /// The ticket was refused: the gateway has no chat (or the session is
    /// gone). The transport stops dialling.
    Refused,
}

/// An HTTP method.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    /// GET.
    Get,
    /// POST.
    Post,
    /// PUT.
    Put,
    /// PATCH.
    Patch,
    /// DELETE.
    Delete,
}

/// What an HTTP request was for: handed back with its answer to
/// [`Social::on_http`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HttpTag {
    /// `POST /social/devices`.
    RegisterDevice,
    /// `GET /social/devices`.
    OwnDevices,
    /// `GET /social/keys?tag=…`.
    Keys(Vec<String>),
    /// `GET /social/players/{handle}`, then a friend request.
    Lookup(String),
    /// `POST /social/friends/requests`.
    AddFriend(String),
    /// A friend or block change for a tag (accept, decline, cancel,
    /// unfriend, block, unblock).
    Friendship(String),
    /// `PUT /social/settings`.
    Settings,
    /// A group change.
    Group,
    /// `POST /social/invites`.
    TableInvite(String),
    /// `DELETE /social/invites/{id}`.
    AnswerInvite(String),
    /// `DELETE /social/devices/{id}`.
    RemoveDevice(String),
    /// `POST /social/reports`.
    Report(LineKey),
}

/// A secret on its way to the credential store. `Debug` prints nothing of
/// it; it is wiped when dropped.
#[derive(Clone, PartialEq, Eq)]
pub struct SecretValue(Zeroizing<String>);

impl SecretValue {
    /// The value, for the store and nothing else.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretValue(..)")
    }
}

/// What the client does for the machine.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    /// An HTTP request to the gateway, with the session; answer it with
    /// [`Social::on_http`] and the same tag.
    Http {
        /// What it is for.
        tag: HttpTag,
        /// The method.
        method: Method,
        /// The path, from the gateway's base.
        path: String,
        /// The JSON body.
        body: Option<serde_json::Value>,
    },
    /// A frame for the social socket.
    Up(Up),
    /// Open the social socket (a fresh ticket each dial).
    Dial,
    /// Close it.
    Hang,
    /// Keep the device's secrets in the [`SecretStore`] slot
    /// [`Social::secret_slot`] names.
    StoreSecrets(SecretValue),
    /// Remove them.
    ForgetSecrets,
    /// Write the state file ([`disk::STATE_FILE`]), already sealed;
    /// debounce it.
    SaveState(Vec<u8>),
    /// Add a sealed record to a history log.
    AppendHistory {
        /// The log's path in the chat folder.
        file: String,
        /// The framed, sealed record.
        record: Vec<u8>,
    },
    /// Replace a history log (pruning); empty bytes remove it.
    RewriteHistory {
        /// The log's path in the chat folder.
        file: String,
        /// Its new contents, sealed.
        bytes: Vec<u8>,
    },
    /// Remove the chat folder: the device was removed, or the guest left.
    DeleteAllLocal,
    /// Show a toast.
    Toast(ChatToast),
    /// Take away a toast that asked about this: it was answered (in the
    /// drawer, on another device) or taken back.
    Withdraw(ToastAbout),
    /// Play a cue.
    Sound(ChatCue),
    /// Show this conversation's tab.
    OpenConversation(ConvId),
    /// Join a table (the lobby takes it from here).
    JoinTable {
        /// The lobby game.
        game: String,
        /// The invite that stands in for its password.
        invite: Option<String>,
    },
    /// Watch a table.
    WatchTable {
        /// The lobby game.
        game: String,
    },
    /// Open the friends drawer.
    OpenDrawer,
    /// Put this on the clipboard.
    Copy(String),
}

/// A toast (§11.11).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatToast {
    /// What it says.
    pub phrase: ChatPhrase,
    /// Its values.
    pub args: Vec<String>,
    /// The conversation it opens, when pressed.
    pub conv: Option<ConvId>,
    /// Its buttons, each an intent.
    pub actions: Vec<(ToastButton, Intent)>,
    /// Stays until answered (requests, invites) rather than 6 s.
    pub until_answered: bool,
    /// What it asks about, for a toast that waits for an answer the player
    /// can also give elsewhere (the drawer, another device) or that the
    /// asker can take back: it goes once that is no longer open
    /// ([`Social::still_asks`]).
    pub about: Option<ToastAbout>,
}

/// What a toast that waits for an answer asks about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToastAbout {
    /// A friend request from this tag.
    FriendRequest(String),
    /// An invite to the group with this id.
    GroupInvite(String),
    /// The table invite with this id.
    TableInvite(String),
}

/// A toast's button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToastButton {
    /// Accept.
    Accept,
    /// Decline.
    Decline,
    /// Join.
    Join,
    /// Watch.
    Watch,
    /// Not now.
    NotNow,
}

/// What the player asked for (§10.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// Send a friend request to a handle.
    AddFriend {
        /// `Name#tag`.
        handle: String,
    },
    /// Accept a request.
    Accept {
        /// From whom.
        tag: String,
    },
    /// Decline a request.
    Decline {
        /// From whom.
        tag: String,
    },
    /// Withdraw a request.
    Cancel {
        /// To whom.
        tag: String,
    },
    /// Remove a friend.
    Unfriend {
        /// Whom.
        tag: String,
    },
    /// Block on the server.
    Block {
        /// Whom.
        tag: String,
    },
    /// Unblock.
    Unblock {
        /// Whom.
        tag: String,
    },
    /// Hide a player's lines on this device.
    Ignore {
        /// Whom.
        tag: String,
    },
    /// Show them again.
    Unignore {
        /// Whom.
        tag: String,
    },
    /// Mute a conversation's notifications.
    MuteConv(ConvId),
    /// Unmute it.
    UnmuteConv(ConvId),
    /// Mark a conversation read.
    MarkRead(ConvId),
    /// Close its tab.
    CloseConv(ConvId),
    /// Show a conversation's tab (and make it the composer's target).
    OpenConv(ConvId),
    /// Start a whisper with a tag.
    Whisper {
        /// With whom.
        tag: String,
    },
    /// Make a group.
    NewGroup {
        /// Its name.
        name: String,
    },
    /// Rename a group.
    RenameGroup {
        /// Which.
        id: String,
        /// Its new name.
        name: String,
    },
    /// Invite a friend to a group.
    InviteToGroup {
        /// Which.
        id: String,
        /// Whom.
        tag: String,
    },
    /// Accept a group invite.
    JoinGroup {
        /// Which.
        id: String,
    },
    /// Decline a group invite.
    DeclineGroup {
        /// Which.
        id: String,
    },
    /// Leave a group.
    LeaveGroup {
        /// Which.
        id: String,
    },
    /// Remove a member (owner).
    Kick {
        /// Which group.
        id: String,
        /// Whom.
        tag: String,
    },
    /// Hand a group on (owner).
    HandOver {
        /// Which group.
        id: String,
        /// To whom.
        tag: String,
    },
    /// Delete a group (owner).
    DeleteGroup {
        /// Which.
        id: String,
    },
    /// Invite a friend to this player's table.
    InviteToTable {
        /// Whom.
        tag: String,
    },
    /// Answer a table invite.
    AnswerInvite {
        /// Which.
        id: String,
        /// Join (or watch) rather than decline.
        accept: bool,
    },
    /// Watch a friend's game.
    Watch {
        /// Whom.
        tag: String,
    },
    /// Set this player's status.
    SetStatus(Status),
    /// Change the server-side settings.
    SetSettings(SocialSettings),
    /// Change this device's chat settings.
    SetPrefs(ChatPrefs),
    /// Mark a contact verified (its safety number matched).
    Verify {
        /// Whom.
        tag: String,
        /// The exact sorted identity/agreement-key snapshot displayed for confirmation.
        expected: Vec<([u8; 32], [u8; 32])>,
    },
    /// Drop the verified mark.
    Unverify {
        /// Whom.
        tag: String,
    },
    /// Remove one of this account's devices (this one included).
    RemoveDevice {
        /// Which.
        id: String,
    },
    /// Set or clear a local nickname.
    Nickname {
        /// Whose.
        tag: String,
        /// The nickname; `None` clears it.
        name: Option<String>,
    },
    /// Report a received message.
    Report {
        /// Which.
        line: LineKey,
        /// What the reporter says.
        note: String,
    },
    /// Enter in the composer: a command or a message.
    Submit,
    /// Send a failed line again.
    Retry {
        /// Which.
        line: LineKey,
    },
    /// Remove a failed line.
    Forget {
        /// Which.
        line: LineKey,
    },
    /// The encryption notice was read ("Got it").
    NoticeSeen,
    /// Open or close the chat panel, and which tab it shows; `None` closes.
    Panel(Option<ConvId>),
}

/// A contact's safety number and devices (§4.4, §11.8).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SafetyView {
    /// The exact displayed key snapshot to include in a verification intent.
    pub expected: Vec<([u8; 32], [u8; 32])>,
    /// The 60 digits, as 12 groups of 5.
    pub groups: Vec<String>,
    /// Whether the player verified exactly these devices.
    pub verified: bool,
    /// The contact's devices: label and when added.
    pub devices: Vec<(String, u64)>,
}

/// One of this account's devices, as Settings lists them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceRow {
    /// Its id, shown in four groups (it is its fingerprint).
    pub id: String,
    /// Its OS family.
    pub label: String,
    /// When it was added, Unix milliseconds.
    pub added_at: u64,
    /// When it was last seen.
    pub seen_at: u64,
    /// This device.
    pub this: bool,
}

/// What the encryption notice should say about this device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoteState {
    /// Whether the first-run notice was answered.
    pub seen: bool,
    /// Where the keys are kept.
    pub home: KeyHome,
    /// The days undelivered messages are kept (`/info.social.keep_days`;
    /// 1 for a guest).
    pub keep_days: u32,
}

/// This device's OS family, the label its registration carries (§8.3).
#[must_use]
pub fn device_label() -> &'static str {
    if cfg!(target_os = "macos") {
        "macOS"
    } else if cfg!(target_os = "windows") {
        "Windows"
    } else if cfg!(target_os = "linux") {
        "Linux"
    } else if cfg!(target_os = "android") {
        "Android"
    } else if cfg!(target_os = "ios") {
        "iOS"
    } else if cfg!(target_arch = "wasm32") {
        "Web"
    } else {
        "Other"
    }
}
