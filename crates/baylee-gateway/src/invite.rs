//! Closed-beta keys (#317): who may make an account, or a new guest, on a
//! gateway with `BAYLEE_REGISTRATION=invite`, and the command its operator
//! makes them with (`baylee-gateway invite …`, see [`cli`]).
//!
//! A key is `BAYLEE-` and sixteen characters of Crockford's base32 in four
//! groups, eighty bits from the operating system's generator: typed off a
//! message, read out over the phone, or pasted. What a player types is read
//! the way Crockford's alphabet asks, so case, spaces, dashes, `O` for `0`
//! and `I` or `L` for `1` do not matter ([`canonical`]). Only the SHA-256 of
//! the sixteen characters is stored ([`digest`]); the key is printed once,
//! when it is made, and never logged.
//!
//! Every refusal of a key reads the same ([`KEY_INVALID`]), whether it was
//! mistyped, never made, used up, expired or revoked: which of those it was
//! would tell a guesser something. A request that sends no key at all is
//! told so in its own words ([`KEY_NEEDED`]), which name the update a client
//! from before keys needs; that says nothing `GET /info` does not.

use baylee_db::invites::{self, Invite, NewInvite};
use sha2::{Digest as _, Sha256};
use std::fmt::Write as _;
use time::OffsetDateTime;

/// Who may make an account here (`BAYLEE_REGISTRATION`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Registration {
    /// Anybody.
    Open,
    /// Somebody with a closed-beta key; a new guest needs one too.
    Invite,
    /// Nobody.
    Off,
}

impl Registration {
    /// `BAYLEE_REGISTRATION` read: `invite` is [`Self::Invite`], the three
    /// spellings `switched_on` reads as off are [`Self::Off`], and anything
    /// else, unset included, leaves registration open as it always was.
    ///
    /// Exactly `invite` and nothing like it, for the reason `switched_on`
    /// gives: widening what a switch accepts and refusing what it does not
    /// know are decisions of their own. An operator who types `Invite` gets
    /// an open gateway, and `GET /info` says `open` to their face.
    #[must_use]
    pub fn from_env(raw: Option<&str>) -> Self {
        match raw {
            Some("invite") => Self::Invite,
            Some("off" | "0" | "false") => Self::Off,
            _ => Self::Open,
        }
    }

    /// How `GET /info` and `GET /auth/config` spell it.
    #[must_use]
    pub fn wire(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Invite => "invite",
            Self::Off => "off",
        }
    }

    /// Whether a stranger may register at all, key or not: what
    /// `registration_enabled` has always said. `true` for [`Self::Invite`],
    /// so a client from before keys still offers the form and is told by
    /// the refusal to update.
    #[must_use]
    pub fn takes_sign_ups(self) -> bool {
        self != Self::Off
    }
}

/// The refusal of a request that brought no key to a gateway that wants one.
///
/// Written for a client from before keys, which shows a refusal it does not
/// know word for word and has no field to type a key into.
pub const KEY_NEEDED: &str = "this gateway is a closed beta: a new account or guest needs a \
     closed beta key. If there is no field for one, update Baylee";

/// The one refusal of a key that does not admit, whatever the reason.
pub const KEY_INVALID: &str = "this closed beta key is not valid";

/// Crockford's base32: the ten digits and the letters but `I`, `L`, `O`
/// and `U`.
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// What every key starts with, so that it reads as what it is.
const PREFIX: &str = "BAYLEE";

/// How many characters of the alphabet a key carries: eighty bits.
const KEY_CHARS: usize = 16;

/// How many random bytes a key is made from: eighty bits.
const KEY_BYTES: usize = KEY_CHARS * 5 / 8;

/// A new key, as it is printed: `BAYLEE-XXXX-XXXX-XXXX-XXXX`.
///
/// # Panics
///
/// When the operating system has no random numbers to give.
#[must_use]
pub fn generate() -> String {
    let mut bytes = [0u8; KEY_BYTES];
    getrandom::fill(&mut bytes).expect("OS RNG available");
    shown(&encode(&bytes))
}

/// `bytes` in the alphabet, five bits to a character, most significant
/// first.
fn encode(bytes: &[u8; KEY_BYTES]) -> String {
    let bits = bytes
        .iter()
        .fold(0u128, |acc, &byte| (acc << 8) | u128::from(byte));
    (0..KEY_CHARS)
        .rev()
        .map(|at| char::from(ALPHABET[usize::try_from((bits >> (at * 5)) & 31).unwrap_or(0)]))
        .collect()
}

/// A canonical key as it is printed.
fn shown(canonical: &str) -> String {
    let mut out = String::from(PREFIX);
    for (at, c) in canonical.chars().enumerate() {
        if at % 4 == 0 {
            out.push('-');
        }
        out.push(c);
    }
    out
}

/// The sixteen characters a typed key names, or `None` when it names none.
///
/// Spaces and dashes are dropped, case is folded, and the prefix is taken
/// off before anything else is read, since it holds an `L` of its own. Then
/// `O` reads as `0` and `I` and `L` as `1`, as Crockford asks. Any other
/// character outside the alphabet (`U` among them), or a length other than
/// sixteen, is no key.
#[must_use]
pub fn canonical(typed: &str) -> Option<String> {
    let folded: String = typed
        .chars()
        .filter(|c| !c.is_whitespace() && !matches!(c, '-' | '_' | '\u{2010}'..='\u{2015}'))
        .flat_map(char::to_uppercase)
        .collect();
    let body = match folded.strip_prefix(PREFIX) {
        Some(rest) if rest.chars().count() == KEY_CHARS => rest,
        _ => folded.as_str(),
    };
    let key: String = body
        .chars()
        .map(|c| match c {
            'O' => '0',
            'I' | 'L' => '1',
            other => other,
        })
        .collect();
    (key.len() == KEY_CHARS && key.bytes().all(|b| ALPHABET.contains(&b))).then_some(key)
}

/// What is stored of a key: the SHA-256 of its canonical characters.
#[must_use]
pub fn digest(canonical: &str) -> Vec<u8> {
    Sha256::digest(canonical.as_bytes()).to_vec()
}

// ----------------------------------------------------------------- command

/// What `baylee-gateway invite …` was asked.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Command {
    /// Make `count` keys, each admitting `uses` accounts.
    Create {
        uses: i32,
        expires: Option<time::Duration>,
        note: Option<String>,
        count: u32,
    },
    /// Show every key but the keys.
    List,
    /// Revoke one.
    Revoke(uuid::Uuid),
    /// Say how.
    Help,
}

/// How to use the command.
const USAGE: &str = "\
usage: baylee-gateway invite <command>

  create [--uses N] [--expires 30d|12h] [--note TEXT] [--count K]
          make K keys (1), each admitting N accounts (1), printed once
  list    every key: id, note, uses left, expiry, revocation, accounts admitted
  revoke <id>
          the key admits nobody from now on

Needs DATABASE_URL, not a running gateway.";

/// The most keys one `create` makes.
const MAX_COUNT: u32 = 100;

/// The most accounts one key admits.
const MAX_USES: i32 = 1000;

/// The longest note, in characters; the table holds the same bound.
const MAX_NOTE_CHARS: usize = 100;

/// The command line after `invite`, read.
fn parse(args: &[String]) -> Result<Command, String> {
    let Some((verb, rest)) = args.split_first() else {
        return Ok(Command::Help);
    };
    match verb.as_str() {
        "create" => parse_create(rest),
        "list" if rest.is_empty() => Ok(Command::List),
        "revoke" => match rest {
            [id] => uuid::Uuid::parse_str(id)
                .map(Command::Revoke)
                .map_err(|_| format!("{id:?} is not a key's id; `invite list` shows them")),
            _ => Err("revoke takes one id".into()),
        },
        "help" | "--help" | "-h" if rest.is_empty() => Ok(Command::Help),
        other => Err(format!("unknown: {other:?}")),
    }
}

/// `create`'s options.
fn parse_create(mut rest: &[String]) -> Result<Command, String> {
    let (mut uses, mut expires, mut note, mut count) = (1, None, None, 1);
    while let [flag, value, tail @ ..] = rest {
        match flag.as_str() {
            "--uses" => {
                uses = value
                    .parse()
                    .ok()
                    .filter(|n| (1..=MAX_USES).contains(n))
                    .ok_or_else(|| format!("--uses takes 1 to {MAX_USES}, not {value:?}"))?;
            }
            "--expires" => expires = Some(expiry(value)?),
            "--note" => note = Some(checked_note(value)?),
            "--count" => {
                count = value
                    .parse()
                    .ok()
                    .filter(|n| (1..=MAX_COUNT).contains(n))
                    .ok_or_else(|| format!("--count takes 1 to {MAX_COUNT}, not {value:?}"))?;
            }
            other => return Err(format!("create does not take {other:?}")),
        }
        rest = tail;
    }
    if let [dangling] = rest {
        return Err(format!("{dangling:?} needs a value"));
    }
    Ok(Command::Create {
        uses,
        expires,
        note,
        count,
    })
}

/// `30d` or `12h`: how long a key admits anybody.
fn expiry(raw: &str) -> Result<time::Duration, String> {
    let wrong = || format!("--expires takes days or hours, like 30d or 12h, not {raw:?}");
    let (count, unit) = raw.split_at(raw.len().saturating_sub(1));
    let count: i64 = count.parse().map_err(|_| wrong())?;
    if !(1..=3650).contains(&count) {
        return Err(wrong());
    }
    match unit {
        "d" => Ok(time::Duration::days(count)),
        "h" => Ok(time::Duration::hours(count)),
        _ => Err(wrong()),
    }
}

/// A note as it is kept: trimmed, at most a hundred characters, and none
/// that would break or reorder the line `invite list` prints it on.
fn checked_note(raw: &str) -> Result<String, String> {
    let note = raw.trim();
    if note.chars().count() > MAX_NOTE_CHARS {
        return Err(format!("a note has {MAX_NOTE_CHARS} characters at most"));
    }
    if note
        .chars()
        .any(|c| c.is_control() || crate::is_bidi_control(c))
    {
        return Err("a note is one line of plain text".into());
    }
    Ok(note.to_owned())
}

/// A moment as `invite list` shows it.
fn when(at: OffsetDateTime) -> String {
    let at = at.to_offset(time::UtcOffset::UTC);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        at.year(),
        u8::from(at.month()),
        at.day(),
        at.hour(),
        at.minute()
    )
}

/// The table `invite list` prints. Nothing in [`Invite`] is a key, so
/// nothing here can print one.
fn table(rows: &[Invite]) -> String {
    let mut out = format!(
        "{:<36}  {:<16}  {:>9}  {:<16}  {:<16}  {:>8}  note\n",
        "id", "created (UTC)", "uses left", "expires", "revoked", "admitted"
    );
    for row in rows {
        let _ = writeln!(
            out,
            "{:<36}  {:<16}  {:>9}  {:<16}  {:<16}  {:>8}  {}",
            row.id,
            when(row.created_at),
            row.uses_left,
            row.expires_at.map_or_else(|| "never".into(), when),
            row.revoked_at.map_or_else(|| "-".into(), when),
            row.admitted,
            row.note.as_deref().unwrap_or("")
        );
    }
    if rows.is_empty() {
        out.push_str("(no keys)\n");
    }
    out
}

/// `baylee-gateway invite …`: runs the command and answers the exit code.
///
/// Its own entrance, taken before the gateway sets up logging, binds a port
/// or reads any other setting: the keys go to standard output, and a log
/// line there would be read as one. It needs `DATABASE_URL` and nothing
/// else, and migrates the database on connecting as the gateway does.
pub async fn cli(args: &[String]) -> i32 {
    let command = match parse(args) {
        Ok(command) => command,
        Err(why) => {
            eprintln!("baylee-gateway invite: {why}\n\n{USAGE}");
            return 2;
        }
    };
    if command == Command::Help {
        println!("{USAGE}");
        return 0;
    }
    let Some(url) = std::env::var("DATABASE_URL").ok().filter(|u| !u.is_empty()) else {
        eprintln!("baylee-gateway invite: DATABASE_URL is not set");
        return 1;
    };
    let db = match baylee_db::connect(&url, 1).await {
        Ok(db) => db,
        Err(e) => {
            eprintln!("baylee-gateway invite: {e:#}");
            return 1;
        }
    };
    match run(&db, command, OffsetDateTime::now_utc()).await {
        Ok(()) => 0,
        Err(why) => {
            eprintln!("baylee-gateway invite: {why}");
            1
        }
    }
}

/// Carries out one command against `db`.
async fn run(
    db: &sea_orm::DatabaseConnection,
    command: Command,
    now: OffsetDateTime,
) -> Result<(), String> {
    match command {
        Command::Create {
            uses,
            expires,
            note,
            count,
        } => {
            let expires_at = expires.map(|d| now + d);
            for _ in 0..count {
                let key = generate();
                let canonical = canonical(&key).ok_or("a key this command made does not read")?;
                invites::create(
                    db,
                    &NewInvite {
                        key_hash: digest(&canonical),
                        note: note.clone().filter(|n| !n.is_empty()),
                        uses,
                        expires_at,
                    },
                )
                .await
                .map_err(|e| format!("storing a key: {e}"))?;
                println!("{key}");
            }
            eprintln!(
                "{count} key{}, {uses} account{} each, {}{}. Shown only now: \
                 only a hash of each is kept.",
                if count == 1 { "" } else { "s" },
                if uses == 1 { "" } else { "s" },
                expires_at.map_or_else(
                    || "never expiring".into(),
                    |at| format!("until {} UTC", when(at))
                ),
                note.map_or_else(String::new, |n| format!(", for {n}"))
            );
            Ok(())
        }
        Command::List => {
            let rows = invites::list(db)
                .await
                .map_err(|e| format!("reading the keys: {e}"))?;
            print!("{}", table(&rows));
            Ok(())
        }
        Command::Revoke(id) => {
            if invites::revoke(db, id, now)
                .await
                .map_err(|e| format!("revoking: {e}"))?
            {
                println!("revoked {id}");
                Ok(())
            } else {
                Err(format!("no key {id} that is not revoked already"))
            }
        }
        Command::Help => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(line: &str) -> Vec<String> {
        line.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn a_key_is_the_prefix_and_four_groups_of_the_alphabet() {
        for _ in 0..200 {
            let key = generate();
            assert_eq!(key.len(), "BAYLEE-XXXX-XXXX-XXXX-XXXX".len(), "{key}");
            let groups: Vec<&str> = key.split('-').collect();
            assert_eq!(groups[0], "BAYLEE", "{key}");
            assert_eq!(groups.len(), 5, "{key}");
            for group in &groups[1..] {
                assert_eq!(group.len(), 4, "{key}");
                assert!(group.bytes().all(|b| ALPHABET.contains(&b)), "{key}");
            }
            assert!(
                !groups[1..].concat().contains(['I', 'L', 'O', 'U']),
                "{key}"
            );
            assert_eq!(canonical(&key).as_deref(), Some(&groups[1..].concat()[..]));
        }
    }

    /// Eighty bits: sixteen characters of five bits, every one of the
    /// thirty-two reachable in every position, and no two keys alike.
    #[test]
    fn a_key_carries_eighty_bits() {
        assert_eq!(KEY_BYTES * 8, 80);
        assert_eq!(KEY_CHARS * 5, 80);
        assert_eq!(encode(&[0; KEY_BYTES]), "0000000000000000");
        assert_eq!(encode(&[0xff; KEY_BYTES]), "ZZZZZZZZZZZZZZZZ");
        // One bit at the bottom and one at the top: no bit is dropped.
        let mut low = [0; KEY_BYTES];
        low[KEY_BYTES - 1] = 1;
        assert_eq!(encode(&low), "0000000000000001");
        let mut high = [0; KEY_BYTES];
        high[0] = 0x80;
        assert_eq!(encode(&high), "G000000000000000");

        let keys: std::collections::BTreeSet<String> = (0..2000).map(|_| generate()).collect();
        assert_eq!(keys.len(), 2000, "two keys alike");
        let mut seen = [[false; 32]; KEY_CHARS];
        for key in &keys {
            for (at, b) in canonical(key).unwrap().bytes().enumerate() {
                let symbol = ALPHABET.iter().position(|&a| a == b).unwrap();
                seen[at][symbol] = true;
            }
        }
        for (at, symbols) in seen.iter().enumerate() {
            assert!(symbols.iter().all(|&s| s), "position {at}: {symbols:?}");
        }
    }

    #[test]
    fn a_key_reads_however_it_was_typed() {
        let want = Some("0123456789ABCDEF".to_owned());
        for typed in [
            "BAYLEE-0123-4567-89AB-CDEF",
            "baylee-0123-4567-89ab-cdef",
            "  BAYLEE 0123 4567 89AB CDEF  ",
            "Baylee0123456789abcdef",
            "0123-4567-89AB-CDEF",
            "0123456789abcdef",
            "O123 4567 89AB CDEF",
            "baylee–0123–4567–89ab–cdef",
            "BAYLEE_0123_4567_89AB_CDEF",
            "\tBAYLEE-0123-4567-89AB-CDEF\n",
        ] {
            assert_eq!(canonical(typed), want, "{typed:?}");
        }
        // Crockford's confusables: O is 0, I and L are 1, either case.
        assert_eq!(
            canonical("BAYLEE-oOiI-lL00-1111-ZZZZ").as_deref(),
            Some("001111001111ZZZZ")
        );
        // The prefix holds an L of its own, and is taken off first.
        assert_eq!(
            canonical("BAYLEE-LLLL-LLLL-LLLL-LLLL").as_deref(),
            Some("1111111111111111")
        );
        assert_eq!(
            canonical("BAYLEE-0123-4567-89AB-CDEF"),
            canonical("B A Y L E E 0123456789ABCDEF")
        );
    }

    #[test]
    fn what_is_not_a_key_is_refused() {
        for typed in [
            "",
            "BAYLEE",
            "BAYLEE-",
            "0123-4567-89AB-CDE",
            "0123-4567-89AB-CDEF0",
            "BAYLEE-0123-4567-89AB-CDEU",
            "BAYLEE-0123-4567-89AB-CDE!",
            "BAYLEE-0123-4567-89AB-CDÉF",
            "XAYLEE-0123-4567-89AB-CDEF",
            "BAYLEE-BAYLEE-0123-4567-89AB-CDEF",
        ] {
            assert_eq!(canonical(typed), None, "{typed:?}");
        }
    }

    #[test]
    fn a_key_is_kept_as_its_hash() {
        let key = canonical("BAYLEE-0123-4567-89AB-CDEF").unwrap();
        let hash = digest(&key);
        assert_eq!(hash.len(), 32);
        assert_eq!(hash, digest("0123456789ABCDEF"));
        assert_ne!(
            hash,
            digest("BAYLEE-0123-4567-89AB-CDEF"),
            "the canonical form, not as typed"
        );
        assert_ne!(hash, digest("0123456789ABCDEG"));
    }

    #[test]
    fn registration_reads_three_ways() {
        assert_eq!(Registration::from_env(Some("invite")), Registration::Invite);
        for off in ["off", "0", "false"] {
            assert_eq!(Registration::from_env(Some(off)), Registration::Off);
        }
        // What `switched_on` reads as on stays open, however near it is.
        for open in [
            None,
            Some(""),
            Some("on"),
            Some("open"),
            Some("Invite"),
            Some("OFF"),
            Some("no"),
        ] {
            assert_eq!(Registration::from_env(open), Registration::Open, "{open:?}");
        }
        assert_eq!(
            [Registration::Open, Registration::Invite, Registration::Off].map(Registration::wire),
            ["open", "invite", "off"]
        );
        assert!(
            Registration::Invite.takes_sign_ups(),
            "old clients still offer the form"
        );
        assert!(Registration::Open.takes_sign_ups());
        assert!(!Registration::Off.takes_sign_ups());
    }

    #[test]
    fn the_command_line_reads() {
        assert_eq!(parse(&args("")), Ok(Command::Help));
        assert_eq!(parse(&args("help")), Ok(Command::Help));
        assert_eq!(parse(&args("list")), Ok(Command::List));
        assert_eq!(
            parse(&args("create")),
            Ok(Command::Create {
                uses: 1,
                expires: None,
                note: None,
                count: 1
            })
        );
        assert_eq!(
            parse(&[
                "create".into(),
                "--note".into(),
                "  Max & Moritz ".into(),
                "--uses".into(),
                "3".into(),
                "--expires".into(),
                "30d".into(),
                "--count".into(),
                "5".into()
            ]),
            Ok(Command::Create {
                uses: 3,
                expires: Some(time::Duration::days(30)),
                note: Some("Max & Moritz".into()),
                count: 5
            })
        );
        let id = uuid::Uuid::now_v7();
        assert_eq!(
            parse(&args(&format!("revoke {id}"))),
            Ok(Command::Revoke(id))
        );
        for wrong in [
            "create --uses 0",
            "create --uses 1001",
            "create --uses x",
            "create --count 0",
            "create --count 101",
            "create --expires 30",
            "create --expires 30w",
            "create --expires 0d",
            "create --expires -1d",
            "create --note",
            "create --colour red",
            "revoke",
            "revoke not-a-uuid",
            "list extra",
            "delete",
        ] {
            assert!(parse(&args(wrong)).is_err(), "{wrong:?}");
        }
        assert_eq!(expiry("12h"), Ok(time::Duration::hours(12)));
    }

    #[test]
    fn a_note_is_one_short_line() {
        assert_eq!(checked_note(&"ü".repeat(100)), Ok("ü".repeat(100)));
        assert!(checked_note(&"x".repeat(101)).is_err());
        assert!(checked_note("two\nlines").is_err());
        assert!(checked_note("turned\u{202e}round").is_err());
    }

    #[test]
    fn the_list_names_everything_but_a_key() {
        let at = OffsetDateTime::from_unix_timestamp(1_790_000_000).unwrap();
        let rows = [
            Invite {
                id: uuid::Uuid::nil(),
                created_at: at,
                note: Some("for Max".into()),
                uses_left: 2,
                expires_at: None,
                revoked_at: Some(at),
                admitted: 1,
            },
            Invite {
                id: uuid::Uuid::max(),
                created_at: at,
                note: None,
                uses_left: 0,
                expires_at: Some(at),
                revoked_at: None,
                admitted: 3,
            },
        ];
        let shown = table(&rows);
        let lines: Vec<&str> = shown.lines().collect();
        assert_eq!(lines.len(), 3, "{shown}");
        assert!(
            lines[1].starts_with(&uuid::Uuid::nil().to_string()),
            "{shown}"
        );
        assert!(
            lines[1].contains("never") && lines[1].ends_with("for Max"),
            "{shown}"
        );
        assert!(lines[2].contains(&when(at)), "{shown}");
        assert_eq!(when(at), "2026-09-21 14:13");
        assert!(table(&[]).contains("(no keys)"));
    }
}
