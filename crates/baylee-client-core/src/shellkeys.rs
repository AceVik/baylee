//! The shell's own keymap: the second keymap beside the table's (the shell
//! design, §2.6; `.claude/ux-b6/KEYBOARD.md` is normative for keys).
//!
//! The table's `Keymap` (`prefs.rs`) already holds `C`, `Tab` and most
//! letters as table actions, so the shell cannot share it. This one differs
//! in three ways the shell needs:
//!
//! - **Bound by character or by code** (`KEYBOARD.md` §3.1). `/`, `?`, `+`
//!   and letters are bound by the character the key *produces*, so the
//!   German `Shift+7` is `/`; arrows, digits, F-keys by the physical code.
//! - **`command`, not `ctrl` or `meta`** (§2.6.4): ⌘ on macOS, Ctrl
//!   elsewhere, resolved when matched, so one account-wide map is right on
//!   a Mac and a PC.
//! - **A rebind refuses** a chord another action holds, naming it, and only
//!   a second, explicit request takes it (§5.2); the fixed keys (§5.3) are
//!   refused outright.
//!
//! The resolver here is pure: it is given the key and the context stack and
//! answers which action, if any, the key is. The client builds the stack
//! from what is open and runs what comes back.

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize};

use crate::i18n::Phrase;

/// Where an action lives: the layer of the context stack that owns it
/// (`KEYBOARD.md` §2.1).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum Context {
    /// Every shell screen.
    Global,
    /// The front door.
    Front,
    /// Play.
    Play,
    /// A room.
    Room,
    /// Decks and House decks.
    Decks,
    /// The deck builder.
    Builder,
    /// Settings.
    Settings,
}

/// One thing a shell key does.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum ShellAction {
    /// Go to Play (`1`).
    GoPlay,
    /// Go to Decks (`2`).
    GoDecks,
    /// Go to Settings (`3`).
    GoSettings,
    /// Back one step (`Alt+←`, `⌘[` on macOS).
    Back,
    /// Focus the screen's search (`/`, `Ctrl/Cmd+F`).
    Search,
    /// The shortcuts overlay (`?`, `Ctrl/Cmd+/`).
    Overlay,
    /// The overlay in "go to" mode (`Ctrl/Cmd+K`, native only).
    QuickSwitch,
    /// Settings (`Ctrl/Cmd+,`).
    OpenSettings,
    /// Read this screen's data again (`F5`, `Ctrl/Cmd+R`).
    Refresh,
    /// Text one step larger (`Ctrl/Cmd + =`, also `+`).
    TextLarger,
    /// Text one step smaller (`Ctrl/Cmd + −`).
    TextSmaller,
    /// Text back to step 4 (`Ctrl/Cmd + 0`).
    TextReset,
    /// Return to your game (`r`, while seated).
    ReturnToGame,
    /// Undo the visible toast's deed (`Ctrl/Cmd+Z`).
    Undo,
    /// Create table (`c`, on Play).
    CreateTable,
    /// Start the game (`Ctrl/Cmd+Enter`, in the room).
    StartGame,
    /// New deck (`n`, on Decks).
    NewDeck,
    /// Edit the focused deck tile (`e`, `F2`).
    EditTile,
    /// Save the deck (`Ctrl/Cmd+S`, in the builder).
    SaveDeck,
    /// Import (`Ctrl/Cmd+I`, in the builder).
    ImportDeck,
    /// Export (`Ctrl/Cmd+E`, in the builder).
    ExportDeck,
}

impl ShellAction {
    /// Every action, in the overlay's order.
    pub const ALL: [Self; 21] = [
        Self::GoPlay,
        Self::GoDecks,
        Self::GoSettings,
        Self::Back,
        Self::Search,
        Self::Overlay,
        Self::QuickSwitch,
        Self::OpenSettings,
        Self::Refresh,
        Self::TextLarger,
        Self::TextSmaller,
        Self::TextReset,
        Self::ReturnToGame,
        Self::Undo,
        Self::CreateTable,
        Self::StartGame,
        Self::NewDeck,
        Self::EditTile,
        Self::SaveDeck,
        Self::ImportDeck,
        Self::ExportDeck,
    ];

    /// The stable name the stored map keys it by (kebab-case).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::GoPlay => "go-play",
            Self::GoDecks => "go-decks",
            Self::GoSettings => "go-settings",
            Self::Back => "back",
            Self::Search => "search",
            Self::Overlay => "overlay",
            Self::QuickSwitch => "quick-switch",
            Self::OpenSettings => "open-settings",
            Self::Refresh => "refresh",
            Self::TextLarger => "text-larger",
            Self::TextSmaller => "text-smaller",
            Self::TextReset => "text-reset",
            Self::ReturnToGame => "return-to-game",
            Self::Undo => "undo",
            Self::CreateTable => "create-table",
            Self::StartGame => "start-game",
            Self::NewDeck => "new-deck",
            Self::EditTile => "edit-tile",
            Self::SaveDeck => "save-deck",
            Self::ImportDeck => "import-deck",
            Self::ExportDeck => "export-deck",
        }
    }

    /// The action a stored name is, if this build knows it.
    #[must_use]
    pub fn of_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|a| a.name() == name)
    }

    /// The layer that owns it.
    #[must_use]
    pub const fn context(self) -> Context {
        match self {
            Self::CreateTable => Context::Play,
            Self::StartGame => Context::Room,
            Self::NewDeck | Self::EditTile => Context::Decks,
            Self::SaveDeck | Self::ImportDeck | Self::ExportDeck => Context::Builder,
            _ => Context::Global,
        }
    }

    /// What the overlay and the Controls section call it.
    #[must_use]
    pub const fn label(self) -> Phrase {
        match self {
            Self::GoPlay => Phrase::ShellPlay,
            Self::GoDecks => Phrase::ShellDecks,
            Self::GoSettings | Self::OpenSettings => Phrase::Settings,
            Self::Back => Phrase::Back,
            Self::Search => Phrase::ShellKeySearch,
            Self::Overlay => Phrase::ShellKeyOverlay,
            Self::QuickSwitch => Phrase::ShellKeyQuickSwitch,
            Self::Refresh => Phrase::ShellKeyRefresh,
            Self::TextLarger => Phrase::ShellKeyTextLarger,
            Self::TextSmaller => Phrase::ShellKeyTextSmaller,
            Self::TextReset => Phrase::ShellKeyTextReset,
            Self::ReturnToGame => Phrase::ShellReturnToGame,
            Self::Undo => Phrase::ShellUndo,
            Self::CreateTable => Phrase::CreateTable,
            Self::StartGame => Phrase::ShellKeyStart,
            Self::NewDeck => Phrase::NewDeck,
            Self::EditTile => Phrase::ShellEdit,
            Self::SaveDeck => Phrase::ShellKeySave,
            Self::ImportDeck => Phrase::ImportDeck,
            Self::ExportDeck => Phrase::ShellKeyExport,
        }
    }

    /// Whether it fires while a text field has focus (`KEYBOARD.md` §2.3:
    /// only chords that edit no text pass a field).
    #[must_use]
    pub fn passes_field(self, chord: &ShellChord) -> bool {
        chord.cmd
            || chord
                .key
                .as_deref()
                .is_some_and(|k| k.starts_with('F') && k.len() <= 3)
    }

    /// Whether it fires over a sheet (§2.2: `?`, `Ctrl/Cmd+/` and the text
    /// size pass a modal; every other screen or global key is blocked).
    #[must_use]
    pub const fn passes_modal(self) -> bool {
        matches!(
            self,
            Self::Overlay | Self::TextLarger | Self::TextSmaller | Self::TextReset
        )
    }

    /// The default chords (`KEYBOARD.md` §7.1).
    #[must_use]
    pub fn defaults(self) -> Vec<ShellChord> {
        use ShellChord as C;
        match self {
            Self::GoPlay => vec![C::code("Digit1")],
            Self::GoDecks => vec![C::code("Digit2")],
            Self::GoSettings => vec![C::code("Digit3")],
            Self::Back => vec![C::code("ArrowLeft").alt(), C::code("BracketLeft").mac_cmd()],
            Self::Search => vec![C::ch("/"), C::ch("f").cmd()],
            Self::Overlay => vec![C::ch("?"), C::ch("/").cmd()],
            Self::QuickSwitch => vec![C::ch("k").cmd()],
            Self::OpenSettings => vec![C::ch(",").cmd()],
            Self::Refresh => vec![C::code("F5"), C::ch("r").cmd()],
            Self::TextLarger => vec![C::ch("=").cmd(), C::ch("+").cmd()],
            Self::TextSmaller => vec![C::ch("-").cmd()],
            Self::TextReset => vec![C::code("Digit0").cmd()],
            Self::ReturnToGame => vec![C::ch("r")],
            Self::Undo => vec![C::ch("z").cmd()],
            Self::CreateTable => vec![C::ch("c")],
            Self::StartGame => vec![C::code("Enter").cmd()],
            Self::NewDeck => vec![C::ch("n")],
            Self::EditTile => vec![C::ch("e"), C::code("F2")],
            Self::SaveDeck => vec![C::ch("s").cmd()],
            Self::ImportDeck => vec![C::ch("i").cmd()],
            Self::ExportDeck => vec![C::ch("e").cmd()],
        }
    }
}

/// One key combination, bound by the character it produces or by the
/// physical key (`KEYBOARD.md` §3.1, §5.4).
///
/// Stored in the table's `Chord` shape plus two optional fields old readers
/// ignore: `"char"` in place of `"key"` for a character binding, and
/// `"cmd"` for the platform's command modifier.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Serialize, Deserialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the modifiers are independent, as in the table's Chord"
)]
pub struct ShellChord {
    /// The physical key (`KeyCode` name), for a code binding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// The produced character, for a character binding.
    #[serde(default, rename = "char", skip_serializing_if = "Option::is_none")]
    pub ch: Option<String>,
    /// ⌘ on macOS, Ctrl elsewhere.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cmd: bool,
    /// Shift (code bindings only; a character binding ignores it).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub shift: bool,
    /// Alt / Option (code bindings only).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub alt: bool,
    /// Only on macOS (the `⌘[` alias of Back).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub mac: bool,
}

impl ShellChord {
    /// A physical key with no modifier.
    #[must_use]
    pub fn code(key: &str) -> Self {
        Self {
            key: Some(key.to_string()),
            ch: None,
            cmd: false,
            shift: false,
            alt: false,
            mac: false,
        }
    }

    /// A produced character with no modifier.
    #[must_use]
    pub fn ch(ch: &str) -> Self {
        Self {
            key: None,
            ch: Some(ch.to_lowercase()),
            cmd: false,
            shift: false,
            alt: false,
            mac: false,
        }
    }

    /// The same with the command modifier.
    #[must_use]
    pub fn cmd(mut self) -> Self {
        self.cmd = true;
        self
    }

    /// The same with Alt / Option.
    #[must_use]
    pub fn alt(mut self) -> Self {
        self.alt = true;
        self
    }

    /// The same with Shift.
    #[must_use]
    pub fn shift(mut self) -> Self {
        self.shift = true;
        self
    }

    /// ⌘ and only on macOS.
    #[must_use]
    pub fn mac_cmd(mut self) -> Self {
        self.cmd = true;
        self.mac = true;
        self
    }

    /// Whether `press` is this chord on this platform (`KEYBOARD.md` §3.2).
    ///
    /// A code chord's modifiers must match exactly (`W` and `⇧W` are two
    /// chords). A character chord ignores Shift — `?` is Shift+/ on US and
    /// Shift+ß on German — and ignores `AltGr` when `AltGr` produced the
    /// character (it arrives as Ctrl and Alt held together); `command`, when
    /// the chord has it, must be held. A non-ASCII character under a command
    /// chord falls back to the letter's US code (Ctrl+F on Cyrillic).
    #[must_use]
    pub fn matches(&self, press: &KeyPress, mac: bool) -> bool {
        if self.mac && !mac {
            return false;
        }
        let command = if mac { press.meta } else { press.ctrl };
        if let Some(key) = &self.key {
            let other = if mac { press.ctrl } else { press.meta };
            return press.code == *key
                && command == self.cmd
                && !other
                && press.shift == self.shift
                && press.alt == self.alt;
        }
        let Some(want) = &self.ch else {
            return false;
        };
        let alt_gr = press.ctrl && press.alt && !mac;
        let produced = press.ch.as_deref().map(str::to_lowercase);
        let same = match produced.as_deref() {
            Some(got) if got == want => true,
            // Ctrl+F on a Cyrillic layout produces `ф`: the letter's US key.
            Some(got) if self.cmd && !got.is_ascii() => {
                want.len() == 1
                    && want.chars().all(|c| c.is_ascii_alphabetic())
                    && press.code == format!("Key{}", want.to_uppercase())
            }
            _ => false,
        };
        if !same {
            return false;
        }
        if self.cmd {
            command && (!press.alt || alt_gr)
        } else {
            // No command held, or AltGr (Ctrl+Alt) produced the character.
            (!press.ctrl && !press.meta) || alt_gr
        }
    }

    /// Whether this chord produces a printable character and nothing more:
    /// what a focused text field owns (`KEYBOARD.md` §2.3).
    #[must_use]
    pub fn is_printable(&self) -> bool {
        if self.cmd || self.alt {
            return false;
        }
        match (&self.key, &self.ch) {
            (_, Some(_)) => true,
            (Some(key), None) => key.starts_with("Digit") || key.starts_with("Key"),
            (None, None) => false,
        }
    }

    /// Whether this chord is one of the fixed keys no shell action may take
    /// (`KEYBOARD.md` §5.3).
    #[must_use]
    pub fn is_fixed(&self) -> bool {
        if self.ch.as_deref() == Some("/") && self.cmd {
            return true;
        }
        let Some(key) = &self.key else {
            return false;
        };
        if self.cmd {
            return false;
        }
        matches!(
            key.as_str(),
            "Escape"
                | "Tab"
                | "Enter"
                | "NumpadEnter"
                | "Space"
                | "ArrowUp"
                | "ArrowDown"
                | "ArrowLeft"
                | "ArrowRight"
                | "Home"
                | "End"
                | "PageUp"
                | "PageDown"
                | "Backspace"
                | "Delete"
                | "ContextMenu"
        ) && !(key == "ArrowLeft" && self.alt)
            || (key == "F10" && self.shift)
    }

    /// The chord as a key cap reads it (`KEYBOARD.md` §3.3): the produced
    /// character, never a code name the layout does not print, after the
    /// modifiers as **words** — `Cmd+` on macOS, `Ctrl+` elsewhere, `Alt+`,
    /// `Shift+` — as the table's keymap writes them (`Chord::display`): the
    /// interface face carries none of `⌘ ⌥ ⇧ ↵`, and each drew an empty
    /// advance. A code chord shows what `learnt` has seen the key produce,
    /// when it has (`Cmd+Ü` for `⌘[` on a German Mac).
    #[must_use]
    pub fn display(&self, mac: bool, learnt: &Learnt) -> String {
        let mut out = String::new();
        if self.cmd {
            out.push_str(if mac { "Cmd+" } else { "Ctrl+" });
        }
        if self.alt {
            out.push_str("Alt+");
        }
        if self.shift {
            out.push_str("Shift+");
        }
        let main = match (&self.ch, &self.key) {
            (Some(ch), _) => ch.to_uppercase(),
            (None, Some(key)) => learnt
                .produced(key)
                .map_or_else(|| pretty_code(key), str::to_uppercase),
            (None, None) => String::new(),
        };
        out.push_str(&main);
        out
    }
}

/// A code name as a key cap prints it.
fn pretty_code(key: &str) -> String {
    match key {
        "ArrowLeft" => "←".into(),
        "ArrowRight" => "→".into(),
        "ArrowUp" => "↑".into(),
        "ArrowDown" => "↓".into(),
        "Enter" => "Enter".into(),
        "Escape" => "Esc".into(),
        "BracketLeft" => "[".into(),
        "BracketRight" => "]".into(),
        other => other
            .strip_prefix("Digit")
            .or_else(|| other.strip_prefix("Key"))
            .unwrap_or(other)
            .to_string(),
    }
}

/// A key event as the resolver reads it.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the modifiers held, each independent"
)]
pub struct KeyPress {
    /// The physical key's `KeyCode` name (`"Digit7"`).
    pub code: String,
    /// The character it produced, if any (`"/"`).
    pub ch: Option<String>,
    /// Shift held.
    pub shift: bool,
    /// Control held.
    pub ctrl: bool,
    /// Alt / Option held (`AltGr` arrives as Ctrl and Alt).
    pub alt: bool,
    /// ⌘ / the Windows key held.
    pub meta: bool,
    /// A key repeat.
    pub repeat: bool,
}

/// What the session has seen keys produce, for the hints (`KEYBOARD.md`
/// §3.3): `BracketLeft` reads `Ü` on a German Mac once it has been seen.
/// Winit offers no layout lookup, so the pairs are learnt as they are typed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Learnt(BTreeMap<String, String>);

impl Learnt {
    /// Notes what an unshifted key produced.
    pub fn learn(&mut self, press: &KeyPress) {
        if press.shift || press.alt {
            return;
        }
        if let Some(ch) = &press.ch
            && ch.chars().count() == 1
            && !ch.chars().all(char::is_control)
        {
            self.0.insert(press.code.clone(), ch.clone());
        }
    }

    /// What `code` produces unshifted, if it has been seen.
    #[must_use]
    pub fn produced(&self, code: &str) -> Option<&str> {
        self.0.get(code).map(String::as_str)
    }
}

/// The shell's keymap: every action's chords, stored with the account
/// (`Preferences.shell_keys`, `KEYBOARD.md` §5.4).
///
/// Stored as one object keyed by each action's kebab-case name — the
/// table's keymap's shape — not as a struct around it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellKeymap {
    binds: BTreeMap<ShellAction, Vec<ShellChord>>,
}

impl Serialize for ShellKeymap {
    fn serialize<S: serde::Serializer>(&self, out: S) -> Result<S::Ok, S::Error> {
        by_name(&self.binds, out)
    }
}

impl Default for ShellKeymap {
    fn default() -> Self {
        Self::standard()
    }
}

fn by_name<S: serde::Serializer>(
    binds: &BTreeMap<ShellAction, Vec<ShellChord>>,
    out: S,
) -> Result<S::Ok, S::Error> {
    use serde::ser::SerializeMap;
    let mut map = out.serialize_map(Some(binds.len()))?;
    for (action, chords) in binds {
        map.serialize_entry(action.name(), chords)?;
    }
    map.end()
}

impl<'de> Deserialize<'de> for ShellKeymap {
    /// Tolerant: a row this build does not know is dropped, a row it cannot
    /// read keeps its default, and an action the stored map lacks takes its
    /// default only where no other action holds that chord (a stored map
    /// from an older client keeps its rebinds and gains the new actions).
    fn deserialize<D: Deserializer<'de>>(from: D) -> Result<Self, D::Error> {
        let raw: BTreeMap<String, serde_json::Value> = BTreeMap::deserialize(from)?;
        let mut binds = BTreeMap::new();
        for (name, value) in raw {
            let Some(action) = ShellAction::of_name(&name) else {
                continue;
            };
            if let Ok(chords) = serde_json::from_value::<Vec<ShellChord>>(value) {
                binds.insert(action, chords);
            }
        }
        let mut map = Self { binds };
        for action in ShellAction::ALL {
            if map.binds.contains_key(&action) {
                continue;
            }
            let free: Vec<ShellChord> = action
                .defaults()
                .into_iter()
                .filter(|chord| map.holder_of(action.context(), chord).is_none())
                .collect();
            map.binds.insert(action, free);
        }
        Ok(map)
    }
}

/// Why a rebind did not happen (`KEYBOARD.md` §5.2).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Refused {
    /// A fixed key (§5.3): never bindable.
    Fixed,
    /// Another action that can be live at the same time holds it; a second,
    /// explicit request ([`ShellKeymap::take`]) moves it.
    Held(ShellAction),
}

impl ShellKeymap {
    /// The defaults.
    #[must_use]
    pub fn standard() -> Self {
        Self {
            binds: ShellAction::ALL
                .into_iter()
                .map(|a| (a, a.defaults()))
                .collect(),
        }
    }

    /// Whether this is exactly the standard map (what a player who never
    /// rebound anything has; the stored preferences leave it out).
    #[must_use]
    pub fn is_standard(&self) -> bool {
        *self == Self::standard()
    }

    /// An action's chords.
    #[must_use]
    pub fn chords(&self, action: ShellAction) -> &[ShellChord] {
        self.binds.get(&action).map_or(&[], Vec::as_slice)
    }

    /// The action, live together with `context`, that holds `chord`: the
    /// co-live set is the global layer plus one screen (`KEYBOARD.md`
    /// §2.6.1), so `c` may mean one thing on Play and another on Decks.
    #[must_use]
    pub fn holder_of(&self, context: Context, chord: &ShellChord) -> Option<ShellAction> {
        self.binds.iter().find_map(|(action, chords)| {
            let co_live = context == Context::Global
                || action.context() == Context::Global
                || action.context() == context;
            (co_live && chords.contains(chord)).then_some(*action)
        })
    }

    /// Binds `chord` to `action` — or refuses: a fixed key always, a chord
    /// another co-live action holds until [`Self::take`] is asked for.
    ///
    /// # Errors
    /// [`Refused`] says why, naming the holder.
    pub fn bind(&mut self, action: ShellAction, chord: ShellChord) -> Result<(), Refused> {
        if chord.is_fixed() {
            return Err(Refused::Fixed);
        }
        if let Some(holder) = self.holder_of(action.context(), &chord)
            && holder != action
        {
            return Err(Refused::Held(holder));
        }
        self.binds.insert(action, vec![chord]);
        Ok(())
    }

    /// The second, explicit request: take `chord` from whoever holds it.
    ///
    /// # Errors
    /// [`Refused::Fixed`] still: a fixed key is never taken.
    pub fn take(&mut self, action: ShellAction, chord: ShellChord) -> Result<(), Refused> {
        if chord.is_fixed() {
            return Err(Refused::Fixed);
        }
        while let Some(holder) = self.holder_of(action.context(), &chord) {
            if holder == action {
                break;
            }
            if let Some(chords) = self.binds.get_mut(&holder) {
                chords.retain(|c| *c != chord);
            }
        }
        self.binds.insert(action, vec![chord]);
        Ok(())
    }

    /// The key cap an action shows (`KEYBOARD.md` §3.3, §4.2): its first
    /// chord that works on this platform — except that a macOS-only code
    /// chord whose key the session has seen type a character wins, so Back
    /// reads `Alt+←` until `BracketLeft` has typed `ü`, then `Cmd+Ü`. `None` for
    /// an action with no binding here.
    #[must_use]
    pub fn hint(&self, action: ShellAction, mac: bool, learnt: &Learnt) -> Option<String> {
        let here: Vec<&ShellChord> = self
            .chords(action)
            .iter()
            .filter(|c| !c.mac || mac)
            .collect();
        let learnt_alias = here.iter().find(|c| {
            c.mac
                && c.key
                    .as_deref()
                    .is_some_and(|k| learnt.produced(k).is_some())
        });
        let shown = learnt_alias
            .or_else(|| here.iter().find(|c| !c.mac))
            .or_else(|| here.first())?;
        Some(shown.display(mac, learnt))
    }

    /// Back to the defaults for one action.
    pub fn reset(&mut self, action: ShellAction) {
        self.binds.insert(action, action.defaults());
    }
}

/// What has the keys right now, innermost first (`KEYBOARD.md` §2.1).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent facts about what is open"
)]
pub struct Stack {
    /// The screen on show, if it is a shell screen with bindings.
    pub screen: Option<Context>,
    /// A sheet (or the overlay) is open: it blocks screen and global keys
    /// but those that pass a modal.
    pub modal: bool,
    /// A menu or popover is open: it blocks everything but Esc and Tab,
    /// which are the composite's own.
    pub menu: bool,
    /// A text field has focus: it owns every printable key and the
    /// editing chords.
    pub field: bool,
    /// A composite with type-ahead has focus: it owns printable keys but
    /// digits and `?` (§1.5).
    pub typeahead: bool,
}

/// The action `press` is, given what is open (`KEYBOARD.md` §2.2), or
/// `None`: the key belongs to the field, the menu, the composite or nobody.
#[must_use]
pub fn resolve(
    keymap: &ShellKeymap,
    press: &KeyPress,
    stack: Stack,
    mac: bool,
) -> Option<ShellAction> {
    if press.repeat || stack.menu {
        return None;
    }
    let layers = [stack.screen, Some(Context::Global)];
    for context in layers.into_iter().flatten() {
        for action in ShellAction::ALL {
            if action.context() != context {
                continue;
            }
            if stack.modal && !action.passes_modal() {
                continue;
            }
            let Some(chord) = keymap
                .chords(action)
                .iter()
                .find(|chord| chord.matches(press, mac))
            else {
                continue;
            };
            if stack.field && !action.passes_field(chord) {
                continue;
            }
            if stack.typeahead && chord.is_printable() && !digit_or_question(press) {
                continue;
            }
            return Some(action);
        }
    }
    None
}

/// Digits and `?` keep their bare meanings on a type-ahead composite.
fn digit_or_question(press: &KeyPress) -> bool {
    press.code.starts_with("Digit") || press.ch.as_deref() == Some("?")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: &str, ch: Option<&str>) -> KeyPress {
        KeyPress {
            code: code.to_string(),
            ch: ch.map(str::to_string),
            ..KeyPress::default()
        }
    }

    fn shifted(code: &str, ch: &str) -> KeyPress {
        KeyPress {
            shift: true,
            ..key(code, Some(ch))
        }
    }

    fn quiet(screen: Context) -> Stack {
        Stack {
            screen: Some(screen),
            ..Stack::default()
        }
    }

    /// `KEYBOARD.md` §9.4: the German layout, as a unit test.
    #[test]
    fn a_character_binding_follows_the_layout() {
        let map = ShellKeymap::standard();
        let play = quiet(Context::Play);
        // German: `/` is Shift+7.
        assert_eq!(
            resolve(&map, &shifted("Digit7", "/"), play, false),
            Some(ShellAction::Search)
        );
        assert_eq!(resolve(&map, &key("Digit7", Some("7")), play, false), None);
        // US: its own key.
        assert_eq!(
            resolve(&map, &key("Slash", Some("/")), play, false),
            Some(ShellAction::Search)
        );
        // German `?` is Shift+ß, the `Minus` code.
        assert_eq!(
            resolve(&map, &shifted("Minus", "?"), play, false),
            Some(ShellAction::Overlay)
        );
        // AltGr+8 on German Windows produces `[` as Ctrl+Alt held.
        let alt_gr = KeyPress {
            ctrl: true,
            alt: true,
            ..key("Digit8", Some("["))
        };
        assert!(ShellChord::ch("[").matches(&alt_gr, false));
        // Find: ⌘F on macOS, Ctrl+F elsewhere, and not the other way round.
        let cmd_f = KeyPress {
            meta: true,
            ..key("KeyF", Some("f"))
        };
        let ctrl_f = KeyPress {
            ctrl: true,
            ..key("KeyF", Some("f"))
        };
        assert_eq!(resolve(&map, &cmd_f, play, true), Some(ShellAction::Search));
        assert_eq!(
            resolve(&map, &ctrl_f, play, false),
            Some(ShellAction::Search)
        );
        assert_eq!(resolve(&map, &ctrl_f, play, true), None);
        // Cyrillic: Ctrl+F produces `ф`, and still finds.
        let cyrillic = KeyPress {
            ctrl: true,
            ..key("KeyF", Some("ф"))
        };
        assert_eq!(
            resolve(&map, &cyrillic, play, false),
            Some(ShellAction::Search)
        );
    }

    /// The back hint reads `Alt+←` until the session has seen what the
    /// bracket key produces, then `Cmd+Ü` on a German Mac (§3.3; the design's
    /// `⌥←` and `⌘Ü` in words, which the interface face can draw).
    #[test]
    fn a_code_chord_shows_the_character_the_session_has_seen() {
        let mut learnt = Learnt::default();
        let alias = ShellChord::code("BracketLeft").mac_cmd();
        assert_eq!(alias.display(true, &learnt), "Cmd+[");
        assert_eq!(
            ShellChord::code("ArrowLeft").alt().display(true, &learnt),
            "Alt+←"
        );
        learnt.learn(&key("BracketLeft", Some("ü")));
        assert_eq!(alias.display(true, &learnt), "Cmd+Ü");
        assert_eq!(ShellChord::ch("s").cmd().display(false, &learnt), "Ctrl+S");
        // The action's hint: `Alt+←` until the bracket is learnt, then `Cmd+Ü`;
        // elsewhere Alt+← whatever was learnt.
        let map = ShellKeymap::standard();
        assert_eq!(
            map.hint(ShellAction::Back, true, &Learnt::default())
                .as_deref(),
            Some("Alt+←")
        );
        assert_eq!(
            map.hint(ShellAction::Back, true, &learnt).as_deref(),
            Some("Cmd+Ü")
        );
        assert_eq!(
            map.hint(ShellAction::Back, false, &learnt).as_deref(),
            Some("Alt+←")
        );
        assert_eq!(
            map.hint(ShellAction::CreateTable, false, &learnt)
                .as_deref(),
            Some("C")
        );
        assert_eq!(ShellChord::ch("/").display(false, &learnt), "/");
    }

    /// §9.2: no printable key fires while a field has focus — every bound
    /// character, and AltGr+Q (`@` on German Windows) — while the chords
    /// that edit no text still pass.
    #[test]
    fn no_printable_key_fires_while_a_field_has_focus() {
        let map = ShellKeymap::standard();
        for screen in [
            Context::Play,
            Context::Decks,
            Context::Room,
            Context::Builder,
        ] {
            let typing = Stack {
                field: true,
                ..quiet(screen)
            };
            for (code, ch) in [
                ("Slash", "/"),
                ("Digit7", "/"),
                ("Minus", "?"),
                ("KeyC", "c"),
                ("KeyN", "n"),
                ("KeyR", "r"),
                ("KeyE", "e"),
                ("Equal", "+"),
                ("Minus", "-"),
                ("Digit1", "1"),
                ("Digit2", "2"),
                ("Digit3", "3"),
            ] {
                for shift in [false, true] {
                    let press = KeyPress {
                        shift,
                        ..key(code, Some(ch))
                    };
                    assert_eq!(resolve(&map, &press, typing, false), None, "{code} {ch}");
                }
            }
            let alt_gr_q = KeyPress {
                ctrl: true,
                alt: true,
                ..key("KeyQ", Some("@"))
            };
            assert_eq!(resolve(&map, &alt_gr_q, typing, false), None);
            let save = KeyPress {
                ctrl: true,
                ..key("KeyS", Some("s"))
            };
            let want = (screen == Context::Builder).then_some(ShellAction::SaveDeck);
            assert_eq!(resolve(&map, &save, typing, false), want);
            let larger = KeyPress {
                ctrl: true,
                ..key("Equal", Some("="))
            };
            assert_eq!(
                resolve(&map, &larger, typing, false),
                Some(ShellAction::TextLarger)
            );
            assert_eq!(
                resolve(&map, &key("F5", None), typing, false),
                Some(ShellAction::Refresh)
            );
        }
    }

    /// A sheet blocks its screen and the global keys, but the overlay and
    /// the text size; a menu blocks everything here.
    #[test]
    fn a_sheet_blocks_its_screen_and_a_menu_blocks_all() {
        let map = ShellKeymap::standard();
        let sheet = Stack {
            modal: true,
            ..quiet(Context::Play)
        };
        assert_eq!(resolve(&map, &key("KeyC", Some("c")), sheet, false), None);
        assert_eq!(resolve(&map, &key("Digit1", Some("1")), sheet, false), None);
        assert_eq!(
            resolve(&map, &shifted("Slash", "?"), sheet, false),
            Some(ShellAction::Overlay)
        );
        let menu = Stack {
            menu: true,
            ..quiet(Context::Play)
        };
        assert_eq!(resolve(&map, &shifted("Slash", "?"), menu, false), None);
        // `c` is Create table on Play only.
        assert_eq!(
            resolve(&map, &key("KeyC", Some("c")), quiet(Context::Play), false),
            Some(ShellAction::CreateTable)
        );
        assert_eq!(
            resolve(&map, &key("KeyC", Some("c")), quiet(Context::Decks), false),
            None
        );
    }

    /// Digits and `?` keep their meanings on a type-ahead composite; other
    /// letters belong to it (§1.5).
    #[test]
    fn a_type_ahead_composite_keeps_letters_but_not_digits_or_the_question_mark() {
        let map = ShellKeymap::standard();
        let nav = Stack {
            typeahead: true,
            ..quiet(Context::Settings)
        };
        assert_eq!(
            resolve(&map, &key("Digit1", Some("1")), nav, false),
            Some(ShellAction::GoPlay)
        );
        assert_eq!(
            resolve(&map, &shifted("Slash", "?"), nav, false),
            Some(ShellAction::Overlay)
        );
        assert_eq!(resolve(&map, &key("Slash", Some("/")), nav, false), None);
        assert_eq!(resolve(&map, &key("KeyR", Some("r")), nav, false), None);
    }

    /// §9.11: a taken chord is refused naming its holder, a second request
    /// moves it, a fixed key is refused outright.
    #[test]
    fn a_rebind_refuses_then_takes_and_never_takes_a_fixed_key() {
        let mut map = ShellKeymap::standard();
        let slash = ShellChord::ch("/");
        assert_eq!(
            map.bind(ShellAction::CreateTable, slash.clone()),
            Err(Refused::Held(ShellAction::Search))
        );
        assert_eq!(map, ShellKeymap::standard(), "a refusal changes nothing");
        assert_eq!(map.take(ShellAction::CreateTable, slash.clone()), Ok(()));
        assert_eq!(
            map.chords(ShellAction::CreateTable),
            std::slice::from_ref(&slash)
        );
        assert!(!map.chords(ShellAction::Search).contains(&slash));
        for fixed in [
            ShellChord::code("Tab"),
            ShellChord::code("Escape"),
            ShellChord::code("Enter"),
            ShellChord::ch("/").cmd(),
            ShellChord::code("F10").shift(),
        ] {
            assert_eq!(
                map.bind(ShellAction::NewDeck, fixed.clone()),
                Err(Refused::Fixed)
            );
            assert_eq!(map.take(ShellAction::NewDeck, fixed), Err(Refused::Fixed));
        }
        // Not co-live: Play's `c` and a Decks action may share a chord.
        assert_eq!(map.bind(ShellAction::NewDeck, ShellChord::ch("c")), Ok(()));
    }

    /// The stored map is the table's shape plus `char` and `cmd`; an unknown
    /// row is dropped, a missing action gets its default where it is free.
    #[test]
    fn the_stored_map_reads_tolerantly_and_round_trips() {
        let map = ShellKeymap::standard();
        let json = serde_json::to_string(&map).expect("serialises");
        assert!(
            json.contains(r#""search":[{"char":"/"},{"char":"f","cmd":true}]"#),
            "{json}"
        );
        assert!(
            json.starts_with(r#"{"go-play":[{"key":"Digit1"}],"#),
            "the map is not one object keyed by action: {json}"
        );
        let back: ShellKeymap = serde_json::from_str(&json).expect("reads");
        assert_eq!(back, map);
        // A rebind survives the trip (the standard map alone could not tell
        // a lost map from a kept one).
        let mut rebound = ShellKeymap::standard();
        rebound
            .take(ShellAction::CreateTable, ShellChord::ch("t"))
            .expect("t is free");
        let json = serde_json::to_string(&rebound).expect("serialises");
        let back: ShellKeymap = serde_json::from_str(&json).expect("reads");
        assert_eq!(back, rebound);
        assert_eq!(
            back.chords(ShellAction::CreateTable),
            &[ShellChord::ch("t")]
        );
        let old =
            r#"{"search":[{"char":"s"}],"from-the-future":[{"key":"F12"}],"go-play":"nonsense"}"#;
        let read: ShellKeymap = serde_json::from_str(old).expect("tolerant");
        assert_eq!(read.chords(ShellAction::Search), &[ShellChord::ch("s")]);
        assert_eq!(
            read.chords(ShellAction::GoPlay),
            ShellAction::GoPlay.defaults().as_slice()
        );
        assert_eq!(read.chords(ShellAction::NewDeck), &[ShellChord::ch("n")]);
    }

    #[test]
    fn every_default_is_unclashed_and_none_is_fixed() {
        let map = ShellKeymap::standard();
        for action in ShellAction::ALL {
            assert_eq!(ShellAction::of_name(action.name()), Some(action));
            for chord in action.defaults() {
                assert!(
                    !chord.is_fixed() || action == ShellAction::Overlay,
                    "{action:?} {chord:?}"
                );
                assert_eq!(
                    map.holder_of(action.context(), &chord),
                    Some(action),
                    "{chord:?}"
                );
            }
        }
    }
}
