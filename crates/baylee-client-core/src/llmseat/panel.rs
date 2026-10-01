//! The settings panel's model: the language-model seat's file as a player
//! edits it on the client's settings screen (`docs/llm-seat.md`,
//! `docs/client.md` §"Settings, and what belongs to whom").
//!
//! Every box holds text as it was typed until the file is written. A number
//! that does not read yet stays on the screen with its fault beside it,
//! rather than snapping back to what it was. What the boxes say is read
//! into [`SeatSettings`], and the one predicate the bridge refuses a file
//! with, [`SeatSettings::faults`], says what is wrong and where. The panel
//! adds only what text can say and settings cannot hold: a number that is
//! not one, a price given by halves, two profiles of one name. Saving is
//! offered only while nothing is wrong ([`SeatPanel::to_save`]), so nothing
//! `check` refuses, a key above all, is ever handed to
//! [`super::store::save`], which refuses it once more.
//!
//! Profiles are kept in a list rather than by name, so a profile being
//! renamed stays where it is and the default follows it: the default is a
//! place in the list, not a name that the next keystroke would orphan.
//!
//! Pure: no file and no clock. `super::desk` reads and writes the files
//! and tells the panel what it found, and the caller says what time it is
//! ([`Moment`]).

use super::ledger::{Ledger, Moment, Spent};
use super::{
    AnswerMode, CapField, Caps, DEFAULT_ANTHROPIC_MODEL, DEFAULT_CLI_CALLS,
    DEFAULT_CLI_SPEND_TOKENS, Field, GivenPrice, PRICED, Period, Place, Price, Profile, Provider,
    SeatSettings, Why, cli_model, price, shaped_like_a_key,
};
use crate::i18n::{Lang, Phrase};
use crate::textbuf::TextBuffer;
use std::collections::BTreeMap;

/// What the settings file held when it was last read or written.
#[derive(Clone, Debug, PartialEq)]
pub enum Disk {
    /// No file: the bridge plays as it did before there was one.
    Missing,
    /// A file, as it reads.
    Read(SeatSettings),
    /// A file that cannot be used, and why, as `store::load` says it. It is
    /// left as it is: the panel edits nothing until the file reads.
    Refused(String),
}

impl Disk {
    /// What `store::load` answered.
    #[must_use]
    pub fn of(loaded: Result<Option<SeatSettings>, String>) -> Self {
        match loaded {
            Ok(Some(settings)) => Self::Read(settings),
            Ok(None) => Self::Missing,
            Err(why) => Self::Refused(why),
        }
    }
}

/// A box or a choice in a profile's editor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Slot {
    /// The profile's name.
    Name,
    /// Which API: a choice.
    Provider,
    /// The model's id.
    Model,
    /// How hard it thinks.
    Effort,
    /// How it answers: a choice.
    Answer,
    /// The most one reply may take.
    MaxTokens,
    /// The price of input, over this build's.
    PriceIn,
    /// The price of output, over this build's.
    PriceOut,
    /// The most a game may spend in dollars.
    GameUsd,
    /// The most a game may spend in tokens.
    GameTokens,
    /// The most calls a game may make.
    GameCalls,
    /// The longest one answer may take.
    ThinkSecs,
    /// The name of the variable the key is read from.
    KeyEnv,
    /// Where the API is.
    BaseUrl,
    /// The program a CLI profile runs.
    Command,
}

impl Slot {
    /// The boxes typed into, in the order Tab walks them.
    pub const TYPED: [Self; 13] = [
        Self::Name,
        Self::Model,
        Self::Effort,
        Self::MaxTokens,
        Self::PriceIn,
        Self::PriceOut,
        Self::GameUsd,
        Self::GameTokens,
        Self::GameCalls,
        Self::ThinkSecs,
        Self::KeyEnv,
        Self::BaseUrl,
        Self::Command,
    ];

    /// The box or choice that shows `field`.
    #[must_use]
    pub const fn of(field: Field) -> Self {
        match field {
            Field::Provider => Self::Provider,
            Field::Model => Self::Model,
            Field::Effort => Self::Effort,
            Field::Answer => Self::Answer,
            Field::MaxTokens => Self::MaxTokens,
            Field::PriceInput => Self::PriceIn,
            Field::PriceOutput => Self::PriceOut,
            Field::GameUsd => Self::GameUsd,
            Field::GameTokens => Self::GameTokens,
            Field::GameCalls => Self::GameCalls,
            Field::ThinkSecs => Self::ThinkSecs,
            Field::KeyEnv => Self::KeyEnv,
            Field::BaseUrl => Self::BaseUrl,
            Field::Command => Self::Command,
        }
    }

    /// What the box is called above it.
    #[must_use]
    pub const fn label(self) -> Phrase {
        match self {
            Self::Name => Phrase::SeatName,
            Self::Provider => Phrase::SeatProvider,
            Self::Model => Phrase::SeatModel,
            Self::Effort => Phrase::SeatEffort,
            Self::Answer => Phrase::SeatAnswer,
            Self::MaxTokens => Phrase::SeatMaxTokens,
            Self::PriceIn => Phrase::SeatPriceIn,
            Self::PriceOut => Phrase::SeatPriceOut,
            Self::GameUsd => Phrase::SeatGameUsd,
            Self::GameTokens => Phrase::SeatGameTokens,
            Self::GameCalls => Phrase::SeatGameCalls,
            Self::ThinkSecs => Phrase::SeatThinkSecs,
            Self::KeyEnv => Phrase::SeatKeyEnv,
            Self::BaseUrl => Phrase::SeatBaseUrl,
            Self::Command => Phrase::SeatCommand,
        }
    }
}

/// What a cap's box is called above it.
#[must_use]
pub const fn cap_label(cap: CapField) -> Phrase {
    match cap {
        CapField::DayUsd => Phrase::SeatDayUsd,
        CapField::MonthUsd => Phrase::SeatMonthUsd,
        CapField::DayTokens => Phrase::SeatDayTokens,
        CapField::MonthTokens => Phrase::SeatMonthTokens,
    }
}

/// Where on the panel the caret or a fault is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Spot {
    /// A profile's box or choice, by the profile's place in the list.
    Profile(usize, Slot),
    /// A cap's box.
    Cap(CapField),
    /// The choice of the default profile.
    Default,
    /// The file as a whole: a refusal no field of the panel shows.
    File,
}

/// What is wrong, as the panel says it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Problem {
    /// What the settings file refuses ([`SeatSettings::faults`]), or a key
    /// typed where a number goes.
    Refused(Why),
    /// A box for a whole number holds something else.
    NotAWholeNumber,
    /// A box for dollars holds something else.
    NotDollars,
    /// Only one of a price's two amounts is given.
    HalfAPrice,
    /// Another profile has this name.
    NameTaken,
    /// A refusal no field shows, in the file's own sentence.
    Said(String),
}

/// One thing wrong, and where it is shown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PanelFault {
    /// Where.
    pub spot: Spot,
    /// What.
    pub problem: Problem,
}

/// What pressing something on the panel asks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    /// Add a profile, and put the caret in its name.
    Add,
    /// Show this profile's fields.
    Select(usize),
    /// Add a copy of this profile, under a name of its own, below it.
    Duplicate(usize),
    /// Remove this profile.
    Remove(usize),
    /// Make this profile the default, or none the default if it is.
    Default(usize),
    /// Set a profile's provider.
    Provider(usize, Provider),
    /// Set how a profile answers; `None` is the build's way.
    Answer(usize, Option<AnswerMode>),
    /// Put a priced model, by its place in [`PRICED`], in a profile's box.
    Suggest(usize, usize),
    /// Put the caret in a box.
    Focus(Spot),
    /// Write the file (`super::desk`).
    Save,
    /// Drop every edit and show what the file holds now.
    Revert,
}

/// What the last save came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Saved {
    /// The file is written.
    Written,
    /// It was not, and why.
    Failed(String),
}

/// A day's and a month's spend from the book, at a moment.
#[derive(Clone, Debug, PartialEq)]
pub struct SpentView {
    /// The day counted, `YYYY-MM-DD`.
    pub day: String,
    /// The month counted, `YYYY-MM`.
    pub month: String,
    /// What the day's games count.
    pub today: Spent,
    /// What the month's games count.
    pub this_month: Spent,
    /// Whose clock the periods are on: "UTC+02:00", or `None` for UTC.
    pub offset: Option<String>,
}

/// One profile's boxes, as typed.
#[derive(Clone, Debug)]
struct Draft {
    name: TextBuffer,
    provider: Provider,
    answer: Option<AnswerMode>,
    model: TextBuffer,
    effort: TextBuffer,
    max_tokens: TextBuffer,
    price_in: TextBuffer,
    price_out: TextBuffer,
    game_usd: TextBuffer,
    game_tokens: TextBuffer,
    game_calls: TextBuffer,
    think_secs: TextBuffer,
    key_env: TextBuffer,
    base_url: TextBuffer,
    command: TextBuffer,
}

/// A form's texts and choices, to tell whether anything was edited.
type Form = (
    Vec<(Provider, Option<AnswerMode>, Vec<String>)>,
    Option<usize>,
    Vec<String>,
);

impl Draft {
    /// The boxes for `profile`, called `name`.
    fn of(name: &str, profile: &Profile) -> Self {
        let text = |value: Option<&str>| TextBuffer::new(value.unwrap_or_default());
        let number =
            |value: Option<u64>| TextBuffer::new(&value.map(|n| n.to_string()).unwrap_or_default());
        let usd = |value: Option<f64>| TextBuffer::new(&value.map(usd_text).unwrap_or_default());
        Self {
            name: TextBuffer::new(name),
            provider: profile.provider,
            answer: profile.answer,
            model: TextBuffer::new(&profile.model),
            effort: text(profile.effort.as_deref()),
            max_tokens: number(profile.max_tokens.map(u64::from)),
            price_in: usd(profile.price.map(|p| p.input)),
            price_out: usd(profile.price.map(|p| p.output)),
            game_usd: usd(profile.game_usd),
            game_tokens: number(profile.game_tokens),
            game_calls: number(profile.game_calls),
            think_secs: number(profile.think_secs),
            key_env: text(profile.key_env.as_deref()),
            base_url: text(profile.base_url.as_deref()),
            command: text(profile.command.as_deref()),
        }
    }

    fn buffer(&self, slot: Slot) -> Option<&TextBuffer> {
        Some(match slot {
            Slot::Name => &self.name,
            Slot::Model => &self.model,
            Slot::Effort => &self.effort,
            Slot::MaxTokens => &self.max_tokens,
            Slot::PriceIn => &self.price_in,
            Slot::PriceOut => &self.price_out,
            Slot::GameUsd => &self.game_usd,
            Slot::GameTokens => &self.game_tokens,
            Slot::GameCalls => &self.game_calls,
            Slot::ThinkSecs => &self.think_secs,
            Slot::KeyEnv => &self.key_env,
            Slot::BaseUrl => &self.base_url,
            Slot::Command => &self.command,
            Slot::Provider | Slot::Answer => return None,
        })
    }

    fn buffer_mut(&mut self, slot: Slot) -> Option<&mut TextBuffer> {
        Some(match slot {
            Slot::Name => &mut self.name,
            Slot::Model => &mut self.model,
            Slot::Effort => &mut self.effort,
            Slot::MaxTokens => &mut self.max_tokens,
            Slot::PriceIn => &mut self.price_in,
            Slot::PriceOut => &mut self.price_out,
            Slot::GameUsd => &mut self.game_usd,
            Slot::GameTokens => &mut self.game_tokens,
            Slot::GameCalls => &mut self.game_calls,
            Slot::ThinkSecs => &mut self.think_secs,
            Slot::KeyEnv => &mut self.key_env,
            Slot::BaseUrl => &mut self.base_url,
            Slot::Command => &mut self.command,
            Slot::Provider | Slot::Answer => return None,
        })
    }

    /// Its name as the file would hold it.
    fn name(&self) -> String {
        self.name.text().trim().to_string()
    }

    /// What the boxes say, as a profile, and what they say that no profile
    /// can hold.
    fn read(&self) -> (Profile, Vec<(Slot, Problem)>) {
        let mut problems = Vec::new();
        let word = |buffer: &TextBuffer| {
            let text = buffer.text().trim();
            (!text.is_empty()).then(|| text.to_string())
        };
        let max_tokens = noted(
            Slot::MaxTokens,
            whole(&self.max_tokens).and_then(|n| {
                n.map(u32::try_from)
                    .transpose()
                    .map_err(|_| Problem::NotAWholeNumber)
            }),
            &mut problems,
        );
        let price_in = noted(Slot::PriceIn, dollars(&self.price_in), &mut problems);
        let price_out = noted(Slot::PriceOut, dollars(&self.price_out), &mut problems);
        let price = match (price_in, price_out) {
            (Some(input), Some(output)) => Some(GivenPrice { input, output }),
            (Some(_), None) if self.price_out.text().trim().is_empty() => {
                problems.push((Slot::PriceOut, Problem::HalfAPrice));
                None
            }
            (None, Some(_)) if self.price_in.text().trim().is_empty() => {
                problems.push((Slot::PriceIn, Problem::HalfAPrice));
                None
            }
            _ => None,
        };
        let profile = Profile {
            provider: self.provider,
            model: self.model.text().trim().to_string(),
            effort: word(&self.effort),
            answer: self.answer,
            max_tokens,
            price,
            game_usd: noted(Slot::GameUsd, dollars(&self.game_usd), &mut problems),
            game_tokens: noted(Slot::GameTokens, whole(&self.game_tokens), &mut problems),
            game_calls: noted(Slot::GameCalls, whole(&self.game_calls), &mut problems),
            think_secs: noted(Slot::ThinkSecs, whole(&self.think_secs), &mut problems),
            key_env: word(&self.key_env),
            base_url: word(&self.base_url),
            command: word(&self.command),
        };
        (profile, problems)
    }

    fn texts(&self) -> Vec<String> {
        Slot::TYPED
            .iter()
            .filter_map(|slot| self.buffer(*slot))
            .map(|buffer| buffer.text().to_string())
            .collect()
    }
}

/// What a box read as, noting in `problems` a box that read as nothing it
/// may hold.
fn noted<T>(
    slot: Slot,
    read: Result<Option<T>, Problem>,
    problems: &mut Vec<(Slot, Problem)>,
) -> Option<T> {
    read.unwrap_or_else(|problem| {
        problems.push((slot, problem));
        None
    })
}

/// A whole number from `buffer`, digits with `_` between them if the
/// player likes, or `None` for an empty box. A key is refused as a key,
/// not as a number that is not one.
fn whole(buffer: &TextBuffer) -> Result<Option<u64>, Problem> {
    let text = buffer.text().trim();
    if text.is_empty() {
        return Ok(None);
    }
    if shaped_like_a_key(text) {
        return Err(Problem::Refused(Why::KeyShaped));
    }
    let digits: String = text.chars().filter(|c| *c != '_').collect();
    (!digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()))
        .then(|| digits.parse().ok())
        .flatten()
        .map(Some)
        .ok_or(Problem::NotAWholeNumber)
}

/// Dollars from `buffer`: digits with one decimal point or comma, a `$`
/// before or after allowed, or `None` for an empty box. A key is refused
/// as a key, not as an amount that is not one.
fn dollars(buffer: &TextBuffer) -> Result<Option<f64>, Problem> {
    let text = buffer.text().trim();
    if text.is_empty() {
        return Ok(None);
    }
    if shaped_like_a_key(text) {
        return Err(Problem::Refused(Why::KeyShaped));
    }
    let bare = text.trim_start_matches('$').trim_end_matches('$').trim();
    let bare = if bare.contains('.') {
        bare.to_string()
    } else {
        bare.replacen(',', ".", 1)
    };
    let points = bare.chars().filter(|c| *c == '.').count();
    (points <= 1
        && bare.chars().any(|c| c.is_ascii_digit())
        && bare.chars().all(|c| c.is_ascii_digit() || c == '.'))
    .then(|| bare.parse::<f64>().ok())
    .flatten()
    .filter(|usd| usd.is_finite())
    .map(Some)
    .ok_or(Problem::NotDollars)
}

/// Dollars as a box shows them: as short as reads back the same.
fn usd_text(usd: f64) -> String {
    format!("{usd}")
}

/// The language-model seat's settings as the panel shows and edits them.
#[derive(Clone, Debug)]
pub struct SeatPanel {
    /// What the file held when last read or written.
    disk: Disk,
    /// What it holds now, when it changed on disk while there were edits
    /// here: they are kept, and saving writes over it.
    newer: Option<Disk>,
    drafts: Vec<Draft>,
    /// The default profile, by its place in `drafts`.
    default: Option<usize>,
    /// The caps' boxes, in [`CapField::ALL`]'s order.
    caps: Vec<TextBuffer>,
    /// The form as it was read, to tell an edit by.
    pristine: Form,
    /// The profile whose fields are shown.
    selected: Option<usize>,
    /// The box with the caret.
    focus: Option<Spot>,
    /// What the last save came to, until the next edit.
    saved: Option<Saved>,
    /// The spend book as last read, and when.
    book: Option<Result<Ledger, String>>,
    /// The moment the spend is shown at.
    now: Option<Moment>,
}

impl SeatPanel {
    /// The panel over what the file holds.
    #[must_use]
    pub fn new(disk: Disk) -> Self {
        let mut panel = Self {
            disk: Disk::Missing,
            newer: None,
            drafts: Vec::new(),
            default: None,
            caps: Vec::new(),
            pristine: (Vec::new(), None, Vec::new()),
            selected: None,
            focus: None,
            saved: None,
            book: None,
            now: None,
        };
        panel.load(disk);
        panel
    }

    /// Shows `disk`, dropping every edit. The profile shown stays shown if
    /// it is still there, and the caret stays in a box that still is.
    fn load(&mut self, disk: Disk) {
        let shown = self.selected_name();
        // The caret goes with its profile, by name: a profile the file
        // added above it moves the box it is in.
        let caret = self.focus.map(|spot| match spot {
            Spot::Profile(at, _) => (self.drafts.get(at).map(Draft::name), spot),
            other => (None, other),
        });
        let empty = SeatSettings::default();
        let settings = match &disk {
            Disk::Read(settings) => settings,
            Disk::Missing | Disk::Refused(_) => &empty,
        };
        self.drafts = settings
            .profiles
            .iter()
            .map(|(name, profile)| Draft::of(name, profile))
            .collect();
        self.default = settings
            .default
            .as_ref()
            .and_then(|name| settings.profiles.keys().position(|n| n == name));
        let caps = settings.caps;
        self.caps = CapField::ALL
            .iter()
            .map(|cap| {
                TextBuffer::new(
                    &match cap {
                        CapField::DayUsd => caps.day_usd.map(usd_text),
                        CapField::MonthUsd => caps.month_usd.map(usd_text),
                        CapField::DayTokens => caps.day_tokens.map(|n| n.to_string()),
                        CapField::MonthTokens => caps.month_tokens.map(|n| n.to_string()),
                    }
                    .unwrap_or_default(),
                )
            })
            .collect();
        self.selected = shown
            .and_then(|name| self.drafts.iter().position(|d| d.name() == name))
            .or_else(|| (!self.drafts.is_empty()).then_some(0));
        self.focus = caret
            .and_then(|(name, spot)| match (spot, name) {
                (Spot::Profile(_, slot), Some(name)) => self
                    .drafts
                    .iter()
                    .position(|d| d.name() == name)
                    .map(|at| Spot::Profile(at, slot)),
                (Spot::Profile(..), None) => None,
                (other, _) => Some(other),
            })
            .filter(|spot| self.holds(*spot));
        self.disk = disk;
        self.newer = None;
        self.pristine = self.form();
    }

    /// What the file on disk holds now, as the desk found it. With no edits
    /// here the panel shows it; with edits, they are kept and the panel
    /// says the file changed ([`Self::newer_on_disk`]). Whether anything the
    /// panel shows changed.
    pub fn found(&mut self, disk: Disk) -> bool {
        if *self.newer.as_ref().unwrap_or(&self.disk) == disk {
            return false;
        }
        if disk == self.disk {
            self.newer = None;
        } else if self.changed() {
            self.newer = Some(disk);
        } else {
            self.load(disk);
        }
        true
    }

    /// Whether the file changed on disk since it was read, while there
    /// were edits here.
    #[must_use]
    pub fn newer_on_disk(&self) -> bool {
        self.newer.is_some()
    }

    /// What the file held when last read or written.
    #[must_use]
    pub fn disk(&self) -> &Disk {
        &self.disk
    }

    /// Whether the panel may be edited: not while the file on disk cannot
    /// be used, which would be written over with less than it holds.
    #[must_use]
    pub fn editable(&self) -> bool {
        !matches!(self.disk, Disk::Refused(_))
    }

    /// Whether anything was edited since the file was read or written.
    #[must_use]
    pub fn changed(&self) -> bool {
        self.form() != self.pristine
    }

    fn form(&self) -> Form {
        (
            self.drafts
                .iter()
                .map(|d| (d.provider, d.answer, d.texts()))
                .collect(),
            self.default,
            self.caps.iter().map(|b| b.text().to_string()).collect(),
        )
    }

    /// How many profiles there are.
    #[must_use]
    pub fn len(&self) -> usize {
        self.drafts.len()
    }

    /// Whether there are none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.drafts.is_empty()
    }

    /// A profile's name as typed.
    #[must_use]
    pub fn name(&self, at: usize) -> Option<&str> {
        self.drafts.get(at).map(|d| d.name.text())
    }

    /// The profile played when none is named, by its place in the list.
    #[must_use]
    pub fn default(&self) -> Option<usize> {
        self.default
    }

    /// The profile whose fields are shown.
    #[must_use]
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    fn selected_name(&self) -> Option<String> {
        self.selected
            .and_then(|at| self.drafts.get(at))
            .map(Draft::name)
    }

    /// A profile's provider.
    #[must_use]
    pub fn provider(&self, at: usize) -> Option<Provider> {
        self.drafts.get(at).map(|d| d.provider)
    }

    /// How a profile answers; `None` is the build's way.
    #[must_use]
    pub fn answer(&self, at: usize) -> Option<AnswerMode> {
        self.drafts.get(at).and_then(|d| d.answer)
    }

    /// The box at `spot`, if it is one that is typed into.
    #[must_use]
    pub fn buffer(&self, spot: Spot) -> Option<&TextBuffer> {
        match spot {
            Spot::Profile(at, slot) => self.drafts.get(at)?.buffer(slot),
            Spot::Cap(cap) => self.caps.get(cap_index(cap)),
            Spot::Default | Spot::File => None,
        }
    }

    fn buffer_mut(&mut self, spot: Spot) -> Option<&mut TextBuffer> {
        match spot {
            Spot::Profile(at, slot) => self.drafts.get_mut(at)?.buffer_mut(slot),
            Spot::Cap(cap) => self.caps.get_mut(cap_index(cap)),
            Spot::Default | Spot::File => None,
        }
    }

    fn holds(&self, spot: Spot) -> bool {
        self.buffer(spot).is_some()
    }

    /// Whether the box at `spot` is drawn. Every box is, but the ones a
    /// profile's provider does not take: an address is an OpenAI-compatible
    /// endpoint's; a key's variable and the dollars are an API's, which a
    /// CLI on a subscription has neither of; and the program to run is a
    /// CLI's. One the file fills stays in view, and so does the box with
    /// the caret, so nothing is kept or typed unseen.
    #[must_use]
    pub fn shows(&self, spot: Spot) -> bool {
        let Spot::Profile(at, slot) = spot else {
            return self.holds(spot);
        };
        let Some(provider) = self.provider(at) else {
            return false;
        };
        let taken = match slot {
            Slot::BaseUrl => provider == Provider::OpenAi,
            Slot::KeyEnv | Slot::PriceIn | Slot::PriceOut | Slot::GameUsd => {
                provider != Provider::Cli
            }
            Slot::Command => provider == Provider::Cli,
            _ => true,
        };
        self.holds(spot)
            && (taken
                || self.focus == Some(spot)
                || self.buffer(spot).is_some_and(|b| !b.text().is_empty()))
    }

    /// The box with the caret.
    #[must_use]
    pub fn focus(&self) -> Option<Spot> {
        self.focus
    }

    /// Whether a box has the caret, so that keys are typed into it.
    #[must_use]
    pub fn typing(&self) -> bool {
        self.focus.is_some()
    }

    /// Takes the caret out of every box.
    pub fn blur(&mut self) {
        self.focus = None;
    }

    /// Edits the box with the caret, if there is one: what a key handler
    /// types, deletes and moves the caret with.
    pub fn edit(&mut self, change: impl FnOnce(&mut TextBuffer)) {
        let Some(spot) = self.focus else {
            return;
        };
        if let Some(buffer) = self.buffer_mut(spot) {
            change(buffer);
            self.saved = None;
        }
    }

    /// Moves the caret to the next box, or the one before: the shown
    /// profile's, then the caps'.
    pub fn tab(&mut self, back: bool) {
        let mut ring: Vec<Spot> = self
            .selected
            .map(|at| Slot::TYPED.iter().map(|s| Spot::Profile(at, *s)).collect())
            .unwrap_or_default();
        ring.extend(CapField::ALL.iter().map(|cap| Spot::Cap(*cap)));
        ring.retain(|spot| self.shows(*spot));
        let next = match self
            .focus
            .and_then(|spot| ring.iter().position(|s| *s == spot))
        {
            Some(at) if back => (at + ring.len() - 1) % ring.len(),
            Some(at) => (at + 1) % ring.len(),
            None if back => ring.len() - 1,
            None => 0,
        };
        self.focus = ring.get(next).copied();
    }

    /// Does what a press asks, but for [`Act::Save`], which is the desk's.
    pub fn act(&mut self, act: Act) {
        if !self.editable() && !matches!(act, Act::Revert | Act::Select(_)) {
            return;
        }
        match act {
            Act::Add => {
                let profile = Profile::new(Provider::Anthropic, DEFAULT_ANTHROPIC_MODEL);
                let name = self.free_name("sonnet");
                self.drafts.push(Draft::of(&name, &profile));
                self.name_the_new(self.drafts.len() - 1);
            }
            Act::Select(at) if at < self.drafts.len() => {
                self.selected = Some(at);
                if matches!(self.focus, Some(Spot::Profile(other, _)) if other != at) {
                    self.focus = None;
                }
            }
            Act::Duplicate(at) if at < self.drafts.len() => {
                let mut copy = self.drafts[at].clone();
                copy.name = TextBuffer::new(&self.free_name(&self.drafts[at].name()));
                self.drafts.insert(at + 1, copy);
                self.default = self.default.map(|d| if d > at { d + 1 } else { d });
                self.name_the_new(at + 1);
            }
            Act::Remove(at) if at < self.drafts.len() => self.remove(at),
            Act::Default(at) if at < self.drafts.len() => {
                self.default = if self.default == Some(at) {
                    None
                } else {
                    Some(at)
                };
            }
            Act::Provider(at, provider) if at < self.drafts.len() => {
                self.drafts[at].provider = provider;
            }
            Act::Answer(at, answer) if at < self.drafts.len() => {
                self.drafts[at].answer = answer;
            }
            Act::Suggest(at, model) if at < self.drafts.len() => {
                if let Some((_, id, _)) = PRICED.get(model) {
                    self.drafts[at].model = TextBuffer::new(id);
                }
            }
            Act::Focus(spot) if self.holds(spot) => {
                if let Spot::Profile(at, _) = spot {
                    self.selected = Some(at);
                }
                self.focus = Some(spot);
                return;
            }
            Act::Revert => {
                let disk = self.newer.take().unwrap_or_else(|| self.disk.clone());
                self.load(disk);
            }
            _ => return,
        }
        self.saved = None;
    }

    /// Shows the profile at `at`, just added, with its name selected for
    /// the name the player will type over it.
    fn name_the_new(&mut self, at: usize) {
        self.selected = Some(at);
        self.focus = Some(Spot::Profile(at, Slot::Name));
        self.drafts[at].name.select_all();
    }

    fn remove(&mut self, at: usize) {
        self.drafts.remove(at);
        // The default is the profile, not the place: it goes with it, and
        // the one after it keeps being the default one place up. No other
        // profile becomes the default in its stead, since a default nobody
        // chose would be a model spending money nobody asked it to.
        self.default = match self.default {
            Some(d) if d == at => None,
            Some(d) if d > at => Some(d - 1),
            other => other,
        };
        self.selected = match self.selected {
            _ if self.drafts.is_empty() => None,
            Some(s) if s > at || (s == at && s == self.drafts.len()) => Some(s - 1),
            other => other,
        };
        self.focus = match self.focus {
            Some(Spot::Profile(f, _)) if f == at => None,
            Some(Spot::Profile(f, slot)) if f > at => Some(Spot::Profile(f - 1, slot)),
            other => other,
        };
    }

    /// `base`, or `base-2`, `base-3`, … the first no profile has.
    fn free_name(&self, base: &str) -> String {
        let taken = |name: &str| self.drafts.iter().any(|d| d.name() == name);
        if !taken(base) {
            return base.to_string();
        }
        // Of one more candidate than there are profiles, one is free.
        (2..=self.drafts.len() + 2)
            .map(|n| {
                let suffix = format!("-{n}");
                let room = 32_usize.saturating_sub(suffix.len());
                format!("{}{suffix}", base.chars().take(room).collect::<String>())
            })
            .find(|name| !taken(name))
            .unwrap_or_default()
    }

    /// What the boxes say, as settings, and everything wrong with them,
    /// each where the panel shows it.
    fn read(&self) -> (SeatSettings, Vec<PanelFault>) {
        let mut faults = Vec::new();
        let mut profiles = BTreeMap::new();
        let mut place: BTreeMap<String, usize> = BTreeMap::new();
        let mut uses: BTreeMap<String, usize> = BTreeMap::new();
        for draft in &self.drafts {
            *uses.entry(draft.name()).or_default() += 1;
        }
        for (at, draft) in self.drafts.iter().enumerate() {
            let (profile, problems) = draft.read();
            faults.extend(problems.into_iter().map(|(slot, problem)| PanelFault {
                spot: Spot::Profile(at, slot),
                problem,
            }));
            let name = draft.name();
            // Both of two profiles with one name say so: the one just
            // typed into may be either.
            if uses.get(&name).is_some_and(|n| *n > 1) {
                faults.push(PanelFault {
                    spot: Spot::Profile(at, Slot::Name),
                    problem: Problem::NameTaken,
                });
            }
            if place.contains_key(&name) {
                continue;
            }
            place.insert(name.clone(), at);
            profiles.insert(name, profile);
        }
        let caps = self.read_caps(&mut faults);
        let default = self
            .default
            .and_then(|at| self.drafts.get(at))
            .map(Draft::name);
        let settings = SeatSettings {
            default,
            caps,
            profiles,
        };
        for fault in settings.faults() {
            let spot = match &fault.place {
                Place::Default => Some(Spot::Default),
                Place::Name(name) => place.get(name).map(|at| Spot::Profile(*at, Slot::Name)),
                Place::Profile(name, field) => place
                    .get(name)
                    .map(|at| Spot::Profile(*at, Slot::of(*field))),
                Place::Cap(cap) => Some(Spot::Cap(*cap)),
                Place::Elsewhere(_) => None,
            };
            faults.push(match spot {
                Some(spot) if fault.why != Why::Unwritable => PanelFault {
                    spot,
                    problem: Problem::Refused(fault.why),
                },
                _ => PanelFault {
                    spot: Spot::File,
                    problem: Problem::Said(fault.sentence),
                },
            });
        }
        // A model the panel already refuses has no price to speak of, and
        // the dollar limit's fault would be a second word about that one box.
        let refused_models: Vec<usize> = faults
            .iter()
            .filter_map(|fault| match fault.spot {
                Spot::Profile(at, Slot::Model) => Some(at),
                _ => None,
            })
            .collect();
        faults.retain(|fault| {
            !(fault.problem == Problem::Refused(Why::Unpriced)
                && matches!(fault.spot, Spot::Profile(at, _) if refused_models.contains(&at)))
        });
        (settings, faults)
    }

    fn read_caps(&self, faults: &mut Vec<PanelFault>) -> Caps {
        let mut caps = Caps::default();
        for (cap, buffer) in CapField::ALL.iter().zip(&self.caps) {
            let mut note = |problem| {
                faults.push(PanelFault {
                    spot: Spot::Cap(*cap),
                    problem,
                });
            };
            match cap {
                CapField::DayUsd => {
                    caps.day_usd = dollars(buffer).unwrap_or_else(|p| {
                        note(p);
                        None
                    });
                }
                CapField::MonthUsd => {
                    caps.month_usd = dollars(buffer).unwrap_or_else(|p| {
                        note(p);
                        None
                    });
                }
                CapField::DayTokens => {
                    caps.day_tokens = whole(buffer).unwrap_or_else(|p| {
                        note(p);
                        None
                    });
                }
                CapField::MonthTokens => {
                    caps.month_tokens = whole(buffer).unwrap_or_else(|p| {
                        note(p);
                        None
                    });
                }
            }
        }
        caps
    }

    /// Everything wrong, each where the panel shows it. Save waits until
    /// there is nothing.
    #[must_use]
    pub fn faults(&self) -> Vec<PanelFault> {
        self.read().1
    }

    /// The settings to write, when there is something to write and nothing
    /// wrong: what `super::desk` hands to `store::save`, and nothing else.
    #[must_use]
    pub fn to_save(&self) -> Option<SeatSettings> {
        if !self.editable() || !self.changed() {
            return None;
        }
        let (settings, faults) = self.read();
        faults.is_empty().then_some(settings)
    }

    /// The file was written with `settings`.
    pub fn saved(&mut self, settings: SeatSettings) {
        self.disk = Disk::Read(settings);
        self.newer = None;
        self.pristine = self.form();
        self.saved = Some(Saved::Written);
    }

    /// The file could not be written, and why.
    pub fn not_saved(&mut self, why: String) {
        self.saved = Some(Saved::Failed(why));
    }

    /// What the last save came to, until the next edit.
    #[must_use]
    pub fn last_save(&self) -> Option<&Saved> {
        self.saved.as_ref()
    }

    /// The models of a profile's provider this build has a price for, by
    /// their place in [`PRICED`].
    #[must_use]
    pub fn suggestions(&self, at: usize) -> Vec<(usize, &'static str, Price)> {
        let Some(provider) = self.provider(at) else {
            return Vec::new();
        };
        PRICED
            .iter()
            .enumerate()
            .filter(|(_, (of, _, _))| *of == provider)
            .map(|(k, (_, model, price))| (k, *model, *price))
            .collect()
    }

    /// This build's price for the model in a profile's box.
    #[must_use]
    pub fn build_price(&self, at: usize) -> Option<Price> {
        self.drafts
            .get(at)
            .and_then(|d| price(d.model.text().trim()))
    }

    /// The variable a profile's key is read from: the box's, or the
    /// provider's where the box is empty. `None` while the box holds no
    /// variable's name.
    #[must_use]
    pub fn key_variable(&self, at: usize) -> Option<String> {
        let draft = self.drafts.get(at)?;
        let (profile, _) = draft.read();
        let faulty = profile
            .faults()
            .iter()
            .any(|fault| fault.field == Field::KeyEnv)
            || profile.key_env.as_deref().is_some_and(shaped_like_a_key);
        profile.key_env().filter(|_| !faulty).map(str::to_string)
    }

    /// A period the caps count dollars in and no tokens, which a profile
    /// with no price could not play in: the bridge refuses its games there
    /// ([`Caps::unpriced_fault`]). A warning, not a fault: the file is
    /// sound, and a price or a token cap given later mends it.
    #[must_use]
    pub fn unpriced_period(&self, at: usize) -> Option<Period> {
        let (profile, _) = self.drafts.get(at)?.read();
        if profile.price().is_some() {
            return None;
        }
        self.read_caps(&mut Vec::new()).unpriced_period()
    }

    /// The spend book as the desk read it, at `now`.
    pub fn read_book(&mut self, book: Result<Ledger, String>, now: Moment) {
        self.book = Some(book);
        self.now = Some(now);
    }

    /// The time is now `now`: whether that moves the day, the month or the
    /// zone the spend is shown in.
    pub fn tick(&mut self, now: Moment) -> bool {
        let before = self.now.map(|m| (m.day(), m.offset));
        self.now = Some(now);
        before != Some((now.day(), now.offset))
    }

    /// What today's and this month's games count, at the moment last told;
    /// `None` before the book was read, and the reason for a book that
    /// cannot be.
    #[must_use]
    pub fn spent(&self) -> Option<Result<SpentView, &str>> {
        let book = self.book.as_ref()?;
        let now = self.now?;
        Some(match book {
            Ok(ledger) => {
                let (day, month) = (now.day(), now.month());
                Ok(SpentView {
                    today: ledger.on_day(&day),
                    this_month: ledger.in_month(&month),
                    day,
                    month,
                    offset: now.utc_offset(),
                })
            }
            Err(why) => Err(why.as_str()),
        })
    }

    /// What an empty box stands for, in the player's language: the value
    /// the bridge plays with when the file leaves the field out. `None` for
    /// a box that is never left empty, and for a cap, whose empty box is
    /// no cap at all.
    #[must_use]
    pub fn hint(&self, spot: Spot, lang: Lang) -> Option<String> {
        let Spot::Profile(at, slot) = spot else {
            return None;
        };
        let provider = self.provider(at)?;
        let cli = provider == Provider::Cli;
        let by_default = |value: &str| Phrase::SeatByDefault.fill(lang, &[value]);
        Some(match slot {
            Slot::Effort if cli => Phrase::SeatEffortCli.text(lang).to_string(),
            Slot::Effort => provider.default_effort().map_or_else(
                || Phrase::SeatEffortEndpoint.text(lang).to_string(),
                by_default,
            ),
            Slot::MaxTokens => by_default(&provider.default_max_tokens().to_string()),
            Slot::PriceIn => by_default(&usd_text(self.build_price(at)?.input)),
            Slot::PriceOut => by_default(&usd_text(self.build_price(at)?.output)),
            // A model with no price has no dollar limit to default to.
            Slot::GameUsd => {
                self.drafts.get(at)?.read().0.price()?;
                by_default(&usd_text(super::DEFAULT_SPEND_USD))
            }
            Slot::GameTokens if cli => by_default(&DEFAULT_CLI_SPEND_TOKENS.to_string()),
            Slot::GameTokens => by_default(&super::DEFAULT_SPEND_TOKENS.to_string()),
            Slot::GameCalls if cli => by_default(&DEFAULT_CLI_CALLS.to_string()),
            Slot::GameCalls => Phrase::SeatNoCallLimit.text(lang).to_string(),
            Slot::ThinkSecs => by_default(&super::DEFAULT_THINK_SECS.to_string()),
            Slot::KeyEnv => by_default(provider.default_key_env()?),
            Slot::BaseUrl => Phrase::SeatBaseByDefault
                .fill(lang, &[provider.base_env()?, provider.default_base()?]),
            Slot::Command => {
                let draft = self.drafts.get(at)?;
                let tool = cli_model(draft.model.text().trim()).ok()?.0;
                Phrase::SeatCommandByDefault.fill(lang, &[tool.name()])
            }
            Slot::Name | Slot::Provider | Slot::Model | Slot::Answer => return None,
        })
    }

    /// How a CLI profile names its model, as a line under the model's box;
    /// `None` for an API's, whose priced models are offered there instead.
    #[must_use]
    pub fn model_note(&self, at: usize, lang: Lang) -> Option<String> {
        (self.provider(at)? == Provider::Cli).then(|| Phrase::SeatCliModel.text(lang).to_string())
    }

    /// What this build knows of a profile's model's price, as a line under
    /// its price boxes; for a CLI, that a subscription has none.
    #[must_use]
    pub fn price_note(&self, at: usize, lang: Lang) -> String {
        if self.provider(at) == Some(Provider::Cli) {
            return Phrase::SeatCliNoPrice.text(lang).to_string();
        }
        self.build_price(at).map_or_else(
            || Phrase::SeatNoBuildPrice.text(lang).to_string(),
            |price| {
                Phrase::SeatBuildPrice
                    .fill(lang, &[&rate(price.input, lang), &rate(price.output, lang)])
            },
        )
    }

    /// A priced model as a suggestion under the model box says it.
    #[must_use]
    pub fn suggestion(model: &str, price: Price, lang: Lang) -> String {
        Phrase::SeatPriced.fill(
            lang,
            &[model, &rate(price.input, lang), &rate(price.output, lang)],
        )
    }

    /// Whether a profile's key variable is set, as a line under its box,
    /// and whether it is: `present` answers for a variable's name, and
    /// nothing here ever holds the value. `None` while the box holds no
    /// variable's name, whose own fault says so.
    #[must_use]
    pub fn key_line(
        &self,
        at: usize,
        present: &dyn Fn(&str) -> bool,
        lang: Lang,
    ) -> Option<(String, bool)> {
        let name = self.key_variable(at)?;
        let set = present(&name);
        let phrase = if set {
            Phrase::SeatKeySet
        } else {
            Phrase::SeatKeyUnset
        };
        Some((phrase.fill(lang, &[&name]), set))
    }

    /// The warning of [`Self::unpriced_period`], as a line.
    #[must_use]
    pub fn warning(&self, at: usize, lang: Lang) -> Option<String> {
        self.unpriced_period(at).map(|period| {
            match period {
                Period::Day => Phrase::SeatUnpricedDay,
                Period::Month => Phrase::SeatUnpricedMonth,
            }
            .text(lang)
            .to_string()
        })
    }

    /// The spend as lines: whose clock the periods are on, today, and this
    /// month, each against the caps the boxes hold; or the one line saying
    /// why the book cannot be read. Nothing before the book was read.
    #[must_use]
    pub fn spent_lines(&self, lang: Lang) -> Vec<String> {
        let view = match self.spent() {
            None => return Vec::new(),
            Some(Err(why)) => return vec![Phrase::SeatBookUnreadable.fill(lang, &[why])],
            Some(Ok(view)) => view,
        };
        let caps = self.read_caps(&mut Vec::new());
        let zone = view.offset.as_deref().map_or_else(
            || Phrase::SeatZoneUtc.text(lang).to_string(),
            |offset| Phrase::SeatZoneLocal.fill(lang, &[offset]),
        );
        vec![
            zone,
            spent_line(
                Phrase::SeatSpentDay,
                &view.day,
                &view.today,
                (caps.day_usd, caps.day_tokens),
                lang,
            ),
            spent_line(
                Phrase::SeatSpentMonth,
                &view.month,
                &view.this_month,
                (caps.month_usd, caps.month_tokens),
                lang,
            ),
        ]
    }

    /// A fault in the player's language.
    #[must_use]
    pub fn say(&self, fault: &PanelFault, lang: Lang) -> String {
        let phrase = match &fault.problem {
            Problem::Said(sentence) => return sentence.clone(),
            Problem::NotAWholeNumber => Phrase::SeatFaultWhole,
            Problem::NotDollars => Phrase::SeatFaultDollars,
            Problem::HalfAPrice => Phrase::SeatFaultHalfPrice,
            Problem::NameTaken => Phrase::SeatFaultNameTaken,
            Problem::Refused(why) => match why {
                Why::KeyNamed => Phrase::SeatFaultKeyNamed,
                Why::KeyShaped => Phrase::SeatFaultKeyShaped,
                Why::NoSuchProfile => Phrase::SeatFaultNoDefault,
                Why::NotAName => Phrase::SeatFaultName,
                Why::NotAModelId => Phrase::SeatFaultModel,
                Why::NoSuchTool => Phrase::SeatFaultTool,
                Why::NotAWord => Phrase::SeatFaultEffort,
                Why::JsonNeedsOpenAi => Phrase::SeatFaultJson,
                Why::NotForCli => match fault.spot {
                    Spot::Profile(_, Slot::Answer) => Phrase::SeatFaultCliAnswer,
                    Spot::Profile(_, Slot::KeyEnv | Slot::BaseUrl) => Phrase::SeatFaultCliKey,
                    _ => Phrase::SeatFaultCliPrice,
                },
                Why::CliOnly => Phrase::SeatFaultCliOnly,
                Why::NotAbsolute => Phrase::SeatFaultCommand,
                Why::Zero => match fault.spot {
                    Spot::Profile(_, Slot::ThinkSecs) => Phrase::SeatFaultThinkSecs,
                    Spot::Profile(_, Slot::GameCalls) => Phrase::SeatFaultGameCalls,
                    _ => Phrase::SeatFaultMaxTokens,
                },
                Why::NotAnAmount => Phrase::SeatFaultDollars,
                Why::Unpriced => Phrase::SeatFaultUnpriced,
                Why::NotAVariable => {
                    let provider = match fault.spot {
                        Spot::Profile(at, _) => self.provider(at),
                        _ => None,
                    };
                    let example = provider
                        .and_then(Provider::default_key_env)
                        .unwrap_or("ANTHROPIC_API_KEY");
                    return Phrase::SeatFaultKeyEnv.fill(lang, &[example]);
                }
                Why::NotSecure => Phrase::SeatFaultAddress,
                Why::Unwritable => Phrase::SeatFaultFile,
            },
        };
        phrase.text(lang).to_string()
    }
}

/// One period's spend against its caps: "Today, 2026-09-30: $1.25 of
/// $10.00 · 0 tokens · games: 2, still open: 1".
fn spent_line(
    phrase: Phrase,
    period: &str,
    spent: &Spent,
    (usd_cap, token_cap): (Option<f64>, Option<u64>),
    lang: Lang,
) -> String {
    let tokens = |n: u64| {
        let count = usize::try_from(n).unwrap_or(usize::MAX);
        Phrase::counted(count, Phrase::SeatTokensOne, Phrase::SeatTokensMany)
            .fill(lang, &[&grouped(n, lang)])
    };
    let dollars = match usd_cap {
        Some(cap) => Phrase::SeatOf.fill(lang, &[&usd(spent.usd, lang), &usd(cap, lang)]),
        None => usd(spent.usd, lang),
    };
    let tokens = match token_cap {
        Some(cap) => Phrase::SeatOf.fill(lang, &[&grouped(spent.tokens, lang), &tokens(cap)]),
        None => tokens(spent.tokens),
    };
    let games = Phrase::SeatGames.fill(
        lang,
        &[
            &(spent.priced.played + spent.unpriced.played).to_string(),
            &(spent.priced.open + spent.unpriced.open).to_string(),
        ],
    );
    phrase.fill(lang, &[period, &format!("{dollars} · {tokens} · {games}")])
}

fn cap_index(cap: CapField) -> usize {
    CapField::ALL
        .iter()
        .position(|c| *c == cap)
        .unwrap_or_default()
}

/// Dollars as the player's language writes them, to the cent: `$1,234.50`,
/// `1.234,50 $`.
#[must_use]
pub fn usd(amount: f64, lang: Lang) -> String {
    let cents = format!("{:.2}", amount.max(0.0));
    let (whole, fraction) = cents.split_once('.').unwrap_or((&cents, "00"));
    let whole = whole
        .parse::<u64>()
        .map_or_else(|_| whole.to_string(), |n| grouped(n, lang));
    match lang {
        Lang::En => format!("${whole}.{fraction}"),
        Lang::De => format!("{whole},{fraction} $"),
    }
}

/// A price per million tokens, as short as it reads: `$2`, `$0.20`,
/// `2 $`, `0,20 $`.
#[must_use]
pub fn rate(amount: f64, lang: Lang) -> String {
    if amount.fract() == 0.0 && amount.abs() < 1e15 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // whole, and in range
        let whole = grouped(amount.max(0.0) as u64, lang);
        match lang {
            Lang::En => format!("${whole}"),
            Lang::De => format!("{whole} $"),
        }
    } else {
        usd(amount, lang)
    }
}

/// A count with its thousands grouped as the player's language groups them:
/// `20,000,000`, `20.000.000`.
#[must_use]
pub fn grouped(n: u64, lang: Lang) -> String {
    let separator = match lang {
        Lang::En => ',',
        Lang::De => '.',
    };
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (at, digit) in digits.chars().enumerate() {
        if at > 0 && (digits.len() - at).is_multiple_of(3) {
            out.push(separator);
        }
        out.push(digit);
    }
    out
}

#[cfg(test)]
#[path = "panel_tests.rs"]
mod tests;
