//! What a new line does (§10.6): a badge, a toast, a sound, a peek line at
//! the table, given the player's settings, do not disturb, focus, mute and
//! ignore.

use super::ChatPrefs;
use super::convs::ConvKind;

/// A synthesised cue (§11.11).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChatCue {
    /// Two soft rising notes.
    Whisper,
    /// A brighter ping.
    Mention,
    /// A chime.
    FriendRequest,
    /// One quiet note.
    GroupLine,
}

/// Where the player is when a line arrives.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NotifyContext {
    /// The chat panel is open on this line's conversation.
    pub reading: bool,
    /// The player's status is do not disturb.
    pub dnd: bool,
    /// The window has the focus.
    pub focused: bool,
    /// A question is up at the table (a line still shows, without a sound
    /// unless it is a whisper or a mention).
    pub question_up: bool,
    /// The line mentions this player.
    pub mention: bool,
    /// The player is at a table (a peek line can be shown).
    pub at_table: bool,
}

/// What a line does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Notify {
    /// Counts on the tab, panel, pill and door.
    pub badge: bool,
    /// Shows a toast.
    pub toast: bool,
    /// Plays a cue.
    pub sound: Option<ChatCue>,
    /// Shows in the table's peek.
    pub peek: bool,
}

/// What a line of `kind` does. An ignored sender's line does nothing (it is
/// hidden); a muted conversation counts only dimly (no badge, toast or
/// sound); do not disturb leaves only the badge (no peek either); a conversation the player
/// is reading does nothing at all.
#[must_use]
pub fn on_line(
    kind: ConvKind,
    muted: bool,
    ignored: bool,
    prefs: &ChatPrefs,
    ctx: NotifyContext,
) -> Notify {
    if ignored || (ctx.reading && ctx.focused) {
        return Notify::default();
    }
    if muted {
        return Notify {
            peek: ctx.at_table && matches!(kind, ConvKind::Table | ConvKind::Team),
            ..Notify::default()
        };
    }
    let peek = ctx.at_table;
    if ctx.dnd {
        // Do not disturb: badges only (§10.6), not even a peek line.
        return Notify {
            badge: true,
            ..Notify::default()
        };
    }
    let urgent = ctx.mention || kind == ConvKind::Whisper;
    let quiet = ctx.question_up && !urgent;
    let (toast, sound) = if ctx.mention {
        (true, prefs.sounds.mention.then_some(ChatCue::Mention))
    } else {
        match kind {
            ConvKind::Whisper => (true, prefs.sounds.whisper.then_some(ChatCue::Whisper)),
            ConvKind::Group => (
                false,
                (prefs.sounds.group && !ctx.focused).then_some(ChatCue::GroupLine),
            ),
            ConvKind::Table | ConvKind::Team => {
                (false, prefs.sounds.table.then_some(ChatCue::GroupLine))
            }
        }
    };
    Notify {
        badge: true,
        // At a table the toast lane isn't drawn: a toast becomes a peek line.
        toast: toast && !ctx.at_table,
        sound: if quiet { None } else { sound },
        peek,
    }
}
