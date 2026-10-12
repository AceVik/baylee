//! The slash commands (§10.3, §12.2), WoW-style: a line that starts with
//! `/` is a command, `//text` sends `/text`, and a command's name is matched
//! case aside. [`parse`] is pure and resolves nothing: a name stays the
//! string typed, which the machine resolves against friends, recent
//! whisper partners and the table's roster.

use baylee_protocol::social::Status;

use super::words::ChatPhrase;

/// Where a line goes.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum Target {
    /// The table this player sits at.
    Table,
    /// This player's team at the table.
    Team,
    /// A group.
    Group(GroupRef),
    /// A whisper to a name, as typed.
    Whisper(String),
    /// Whatever the composer's chip shows.
    #[default]
    Sticky,
}

/// Which group.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GroupRef {
    /// The open tab's group, else the last group written to (`/g`).
    Current,
    /// The Nth group in the drawer's order (`/1`–`/9`).
    Number(u8),
}

/// A parsed line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// A message (`/t`, `/team`, `/g`, `/1`, `/me`, or plain text). An empty
    /// text switches the composer's target instead of sending.
    Say {
        /// Where.
        to: Target,
        /// What (may be empty: switch the target).
        text: String,
        /// `/me`.
        emote: bool,
    },
    /// `/r text`.
    Reply(String),
    /// `/w name text`; an empty text switches the target to them.
    Whisper {
        /// The name, as typed.
        who: String,
        /// What.
        text: String,
    },
    /// `/friend name`.
    Friend(String),
    /// `/unfriend name`.
    Unfriend(String),
    /// `/block name`.
    Block(String),
    /// `/unblock name`.
    Unblock(String),
    /// `/ignore name` (or `/mute name`).
    Ignore(String),
    /// `/unignore name` (or `/unmute name`).
    Unignore(String),
    /// `/invite name`.
    Invite(String),
    /// `/watch name`.
    Watch(String),
    /// `/online`, `/away`, `/dnd`, `/invisible`.
    Status(Status),
    /// `/clear`.
    Clear,
    /// `/mute` with no name.
    Mute,
    /// `/unmute` with no name.
    Unmute,
    /// `/who`.
    Who,
    /// `/help [command]`.
    Help(Option<String>),
    /// `/leave`.
    Leave,
    /// `/friends`.
    Friends,
    /// `/nick name [nickname]`.
    Nick {
        /// The name, as typed.
        who: String,
        /// The nickname; `None` clears it.
        name: Option<String>,
    },
}

/// Why a line isn't a command that can run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandFault {
    /// No such command (its name, without the `/`).
    Unknown(String),
    /// The command needs a name (its name).
    MissingName(String),
    /// The command needs a message (its name).
    MissingText(String),
    /// The command has nowhere to go here (`/t` away from a table).
    NotHere(Target),
}

impl CommandFault {
    /// What the notice line says, and its values.
    #[must_use]
    pub fn said(&self) -> (ChatPhrase, Vec<String>) {
        match self {
            Self::Unknown(name) => (ChatPhrase::UnknownCommand, vec![name.clone()]),
            Self::MissingName(name) => (ChatPhrase::MissingName, vec![name.clone()]),
            Self::MissingText(name) => (ChatPhrase::MissingText, vec![name.clone()]),
            Self::NotHere(Target::Team) => (ChatPhrase::NoTeam, Vec::new()),
            Self::NotHere(Target::Group(GroupRef::Number(n))) => {
                (ChatPhrase::NoGroupNumber, vec![n.to_string()])
            }
            Self::NotHere(Target::Group(GroupRef::Current)) => (ChatPhrase::NoGroup, Vec::new()),
            Self::NotHere(Target::Whisper(_)) => (ChatPhrase::NoReply, Vec::new()),
            Self::NotHere(Target::Table | Target::Sticky) => (ChatPhrase::NoTable, Vec::new()),
        }
    }
}

/// One command, its aliases and its help line: what `/help` lists and the
/// composer completes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandInfo {
    /// The name and its aliases, the name first.
    pub names: &'static [&'static str],
    /// Its help line.
    pub help: ChatPhrase,
}

/// Every command, in `/help`'s order.
pub const COMMANDS: &[CommandInfo] = &[
    CommandInfo {
        names: &["t", "s", "say", "table"],
        help: ChatPhrase::HelpTable,
    },
    CommandInfo {
        names: &["team", "p", "party"],
        help: ChatPhrase::HelpTeam,
    },
    CommandInfo {
        names: &["w", "whisper", "tell", "msg"],
        help: ChatPhrase::HelpWhisper,
    },
    CommandInfo {
        names: &["r", "reply"],
        help: ChatPhrase::HelpReply,
    },
    CommandInfo {
        names: &["g", "group", "1", "2", "3", "4", "5", "6", "7", "8", "9"],
        help: ChatPhrase::HelpGroup,
    },
    CommandInfo {
        names: &["me", "emote", "e"],
        help: ChatPhrase::HelpEmote,
    },
    CommandInfo {
        names: &["friend", "f", "add", "unfriend", "remove"],
        help: ChatPhrase::HelpFriend,
    },
    CommandInfo {
        names: &["block", "unblock"],
        help: ChatPhrase::HelpBlock,
    },
    CommandInfo {
        names: &["ignore", "unignore"],
        help: ChatPhrase::HelpIgnore,
    },
    CommandInfo {
        names: &["invite", "inv"],
        help: ChatPhrase::HelpInvite,
    },
    CommandInfo {
        names: &["watch", "spectate"],
        help: ChatPhrase::HelpWatch,
    },
    CommandInfo {
        names: &[
            "online",
            "away",
            "afk",
            "dnd",
            "busy",
            "invisible",
            "offline",
        ],
        help: ChatPhrase::HelpStatus,
    },
    CommandInfo {
        names: &["friends", "social"],
        help: ChatPhrase::HelpFriends,
    },
    CommandInfo {
        names: &["who"],
        help: ChatPhrase::HelpWho,
    },
    CommandInfo {
        names: &["nick"],
        help: ChatPhrase::HelpNick,
    },
    CommandInfo {
        names: &["leave"],
        help: ChatPhrase::HelpLeave,
    },
    CommandInfo {
        names: &["clear"],
        help: ChatPhrase::HelpClear,
    },
    CommandInfo {
        names: &["mute", "unmute"],
        help: ChatPhrase::HelpMute,
    },
    CommandInfo {
        names: &["help", "?", "commands"],
        help: ChatPhrase::HelpHelp,
    },
];

/// The help line for the command `name` (any alias, case aside).
#[must_use]
pub fn help_for(name: &str) -> Option<ChatPhrase> {
    let name = name.trim_start_matches('/').to_lowercase();
    COMMANDS
        .iter()
        .find(|c| c.names.contains(&name.as_str()))
        .map(|c| c.help)
}

/// Every command name and alias starting with `prefix` (case aside), for
/// the composer's completion.
#[must_use]
pub fn names_starting(prefix: &str) -> Vec<&'static str> {
    let prefix = prefix.to_lowercase();
    COMMANDS
        .iter()
        .flat_map(|c| c.names.iter().copied())
        .filter(|n| n.starts_with(&prefix))
        .collect()
}

/// Reads a line the player sent.
///
/// # Errors
///
/// [`CommandFault::Unknown`], [`CommandFault::MissingName`] or
/// [`CommandFault::MissingText`]. ([`CommandFault::NotHere`] is the
/// machine's, which knows where the player is.)
pub fn parse(line: &str) -> Result<Command, CommandFault> {
    let Some(rest) = line.strip_prefix('/') else {
        return Ok(say(Target::Sticky, line));
    };
    if rest.starts_with('/') {
        return Ok(say(Target::Sticky, rest));
    }
    let (name, args) = rest
        .split_once(char::is_whitespace)
        .map_or((rest, ""), |(name, args)| (name, args.trim_start()));
    let lower = name.to_lowercase();
    let text = args.trim_end().to_string();
    let needs_name = |make: fn(String) -> Command| -> Result<Command, CommandFault> {
        let (who, _) = split_name(args);
        if who.is_empty() {
            Err(CommandFault::MissingName(lower.clone()))
        } else {
            Ok(make(who))
        }
    };
    Ok(match lower.as_str() {
        "t" | "s" | "say" | "table" => say(Target::Table, &text),
        "team" | "p" | "party" => say(Target::Team, &text),
        "g" | "group" => say(Target::Group(GroupRef::Current), &text),
        "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
            let n = lower.parse().unwrap_or(1);
            say(Target::Group(GroupRef::Number(n)), &text)
        }
        "me" | "emote" | "e" => {
            if text.is_empty() {
                return Err(CommandFault::MissingText(lower));
            }
            Command::Say {
                to: Target::Sticky,
                text,
                emote: true,
            }
        }
        "w" | "whisper" | "tell" | "msg" => {
            let (who, text) = split_name(args);
            if who.is_empty() {
                return Err(CommandFault::MissingName(lower));
            }
            Command::Whisper { who, text }
        }
        "r" | "reply" => Command::Reply(text),
        "friend" | "f" | "add" => needs_name(Command::Friend)?,
        "unfriend" | "remove" => needs_name(Command::Unfriend)?,
        "block" => needs_name(Command::Block)?,
        "unblock" => needs_name(Command::Unblock)?,
        "mute" if text.is_empty() => Command::Mute,
        "unmute" if text.is_empty() => Command::Unmute,
        "ignore" | "mute" => needs_name(Command::Ignore)?,
        "unignore" | "unmute" => needs_name(Command::Unignore)?,
        "invite" | "inv" => needs_name(Command::Invite)?,
        "watch" | "spectate" => needs_name(Command::Watch)?,
        "online" => Command::Status(Status::Online),
        "away" | "afk" => Command::Status(Status::Away),
        "dnd" | "busy" => Command::Status(Status::Dnd),
        "invisible" | "offline" => Command::Status(Status::Invisible),
        "friends" | "social" => Command::Friends,
        "who" => Command::Who,
        "leave" => Command::Leave,
        "clear" => Command::Clear,
        "help" | "?" | "commands" => {
            Command::Help((!text.is_empty()).then(|| text.trim_start_matches('/').to_lowercase()))
        }
        "nick" => {
            let (who, name) = split_name(args);
            if who.is_empty() {
                return Err(CommandFault::MissingName(lower));
            }
            Command::Nick {
                who,
                name: (!name.is_empty()).then_some(name),
            }
        }
        _ => return Err(CommandFault::Unknown(name.to_string())),
    })
}

fn say(to: Target, text: &str) -> Command {
    Command::Say {
        to,
        text: text.trim_end().to_string(),
        emote: false,
    }
}

/// Splits a name off the front of `args`: a handle with a space in its
/// display name (`Bob Smith#0012`) runs to its first word that has a `#`
/// tag in it, if one is among the first four (a word that *starts* with
/// `#` counts only as the first); else the name is the first word. Returns
/// the name and the rest, trimmed.
#[must_use]
pub fn split_name(args: &str) -> (String, String) {
    let args = args.trim();
    if args.is_empty() {
        return (String::new(), String::new());
    }
    let words: Vec<&str> = args.split_whitespace().collect();
    // A bare `#tag` names a player only as the first word: later, `#1234`
    // is an issue number or a hex word in the message (`/w Bob see #1234`).
    let tagged = words
        .iter()
        .take(4)
        .enumerate()
        .position(|(i, w)| has_tag(w) && (i == 0 || !w.starts_with('#')));
    let take = tagged.map_or(1, |i| i + 1);
    let mut rest = args;
    for _ in 0..take {
        rest = rest.trim_start();
        let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        rest = &rest[end..];
    }
    let name_len = args.len() - rest.len();
    (args[..name_len].trim().to_string(), rest.trim().to_string())
}

/// Whether `word` ends in a `#tag` (`Bob#0012`, `#0012`).
fn has_tag(word: &str) -> bool {
    word.rsplit_once('#')
        .is_some_and(|(_, tag)| baylee_protocol::social::valid_tag(tag))
}
