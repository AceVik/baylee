//! The guided tours (`.claude/tours/TOURS.md` v3): the scripts as data, the
//! device's switches and seen marks, and the steps' walk as pure functions.
//!
//! Three tours — the lobby and the room, the deck builder, the table — each
//! a list of chapters, each a list of steps. A step is **narrated** (the
//! bubble holds the keys), **try-it** (the bubble lets go and waits on a
//! [`Check`] the screen reads from state), **just-in-time** (not in the
//! scripted order: it fires the first time its anchor stands, once per
//! device, under the tips switch) or a **text** step (no anchor, centred).
//!
//! The tours explain the client, never the game: the audience knows Magic
//! (TOURS.md's audience rule), and two tests here hold every step's words to
//! the bubble's budget and to a short tripwire of rules vocabulary.
//!
//! The renderer (`baylee-client`'s `tour`) draws what [`Run`] says and asks
//! the anchors by [`Anchor`] every frame; nothing here knows a node.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::i18n::Phrase;

#[cfg(test)]
mod tests;

/// One of the three tours.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Tour {
    /// The front door, the Play screen, creating a table, the room.
    Lobby,
    /// The shelf, the builder, import, export, history.
    Builder,
    /// The table, in a practice game.
    Table,
}

impl Tour {
    /// Every tour.
    pub const ALL: [Self; 3] = [Self::Lobby, Self::Builder, Self::Table];

    /// Its chapters, in the order a run walks them.
    #[must_use]
    pub const fn chapters(self) -> &'static [Chapter] {
        match self {
            Self::Lobby => LOBBY,
            Self::Builder => BUILDER,
            Self::Table => TABLE,
        }
    }

    /// The prefix of its seen marks (`"lobby/door"`).
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Lobby => "lobby",
            Self::Builder => "builder",
            Self::Table => "table",
        }
    }

    /// Whether its chapters follow on one from the next in one run (the
    /// table's), rather than each waiting for its own screen.
    #[must_use]
    pub const fn continuous(self) -> bool {
        matches!(self, Self::Table)
    }
}

/// A thing on screen a step points at. The renderer's spawners put a
/// marker with one of these on the node; the tour asks for it every frame
/// and caches nothing (TOURS.md §3.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[allow(missing_docs)] // each name is the id of TOURS.md §3.1's table
pub enum Anchor {
    // The front door.
    FrontGatewayRow,
    FrontGuest,
    FrontTextRow,
    // The shell's header.
    ShellNav,
    ShellBell,
    ShellAccount,
    // Play.
    PlayHero,
    PlayHouse,
    PlayCreate,
    PlayTables,
    CreateSheet,
    // The room.
    RoomSeats,
    RoomMySeatMenu,
    RoomRules,
    RoomInvite,
    RoomStart,
    // The Decks screen and the builder.
    DecksTabs,
    DecksHouseTab,
    DecksFirstTile,
    DecksTileActions,
    DecksNew,
    DecksImport,
    BuildHeader,
    BuildCount,
    BuildSearch,
    BuildFirstRow,
    BuildDeck,
    BuildFooter,
    BuildSave,
    BuildPaneSwitch,
    ImportSheet,
    ExportSheet,
    HistorySheet,
    // The table.
    MyPod,
    Hand,
    HandSort,
    HandTab,
    HandCard,
    LitCard,
    Shelf,
    ShelfPass,
    ShelfCancelCast,
    TrayLog,
    TrayZones,
    Burger,
    MenuPanel,
    StackPanel,
    StackControls,
    DecisionSheet,
    DecisionFold,
    DecisionPill,
    AbilitySheet,
    PlayersStrip,
    PlayerChip,
    MyPlate,
    MySteps,
    Dial,
    LogPanel,
    RevealSheet,
    ZoneDialog,
    ArrangementPill,
    ReportCorner,
    ReportForm,
}

/// What a try-it step waits for, read from state, never from UI events
/// (TOURS.md §3.4). The renderer answers each; once true it is latched for
/// the step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Check {
    /// L8: the create-table sheet is open.
    CreateSheetOpen,
    /// L9: the player stands in a room.
    InRoom,
    /// D7: the pool's query is not empty.
    QueryTyped,
    /// D8: the deck's entry count differs from the step's opening count.
    DeckChanged,
    /// D11: the builder saved since the step opened.
    Saved,
    /// T8: this seat's cast was seen and a `CancelCast` left.
    CastCancelled,
    /// T14: the question's sheet was folded and unfolded.
    DecisionFolded,
    /// T21: the arrangement changed since the step opened.
    ArrangementChanged,
    /// T22: a visit began and ended since the step opened.
    Visited,
    /// T26: the log panel stood since the step opened.
    LogOpened,
    /// T31: the report form is open.
    ReportOpened,
}

/// A step's kind (TOURS.md §1.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// The bubble holds the keys and the pointer.
    Narrated,
    /// The bubble lets go and waits on a check.
    Try(Check),
    /// Fires the first time its anchor stands; once per device.
    Jit,
    /// No anchor: the bubble centred, the scrim whole.
    Text,
    /// L14: a text step that offers the practice game.
    Offer,
}

impl Kind {
    /// Whether the bubble holds the keyboard while this step stands.
    #[must_use]
    pub const fn holds(self) -> bool {
        !matches!(self, Self::Try(_))
    }
}

/// Which device class a step runs on (TOURS.md §2's size marks).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Only {
    /// Every device.
    Any,
    /// Phones only.
    Phone,
    /// Never on a phone.
    Desktop,
    /// Three seats or more (the arrangement and the visits), never on a
    /// phone.
    RingDesktop,
    /// Three seats or more.
    Ring,
}

impl Only {
    /// Whether a step so marked runs here.
    #[must_use]
    pub const fn runs(self, phone: bool, seats: usize) -> bool {
        match self {
            Self::Any => true,
            Self::Phone => phone,
            Self::Desktop => !phone,
            Self::RingDesktop => !phone && seats >= 3,
            Self::Ring => seats >= 3,
        }
    }
}

/// One step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    /// Its id in TOURS.md (`"L4"`), also its JIT seen mark's tail.
    pub id: &'static str,
    /// What it points at; `None` for a text step.
    pub anchor: Option<Anchor>,
    /// What it points at on a phone, where that differs.
    pub phone_anchor: Option<Anchor>,
    /// Its kind.
    pub kind: Kind,
    /// Its title (≤ 22 characters, one line on a phone).
    pub title: Phrase,
    /// Its one paragraph.
    pub body: Phrase,
    /// Where it runs.
    pub only: Only,
}

impl Step {
    /// The anchor on this device class.
    #[must_use]
    pub fn anchor_on(&self, phone: bool) -> Option<Anchor> {
        if phone {
            self.phone_anchor.or(self.anchor)
        } else {
            self.anchor
        }
    }
}

/// Where a chapter opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// The front door, before sign-in.
    Door,
    /// The signed-in Play screen (the create sheet over it included).
    Play,
    /// A room.
    Room,
    /// The Decks screen.
    Shelf,
    /// The builder.
    Builder,
    /// The table of a practice game.
    Table,
    /// Nowhere: a chapter of just-in-time steps, which fire on their own.
    Jit,
}

/// One chapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chapter {
    /// Its id, the tail of its seen mark (`"door"`).
    pub id: &'static str,
    /// Its name on the chapter line.
    pub name: Phrase,
    /// Where it opens.
    pub place: Place,
    /// The chapter of the same tour that must have been seen first.
    pub after: Option<&'static str>,
    /// Its steps.
    pub steps: &'static [Step],
}

const fn step(
    id: &'static str,
    anchor: Option<Anchor>,
    kind: Kind,
    title: Phrase,
    body: Phrase,
) -> Step {
    Step {
        id,
        anchor,
        phone_anchor: None,
        kind,
        title,
        body,
        only: Only::Any,
    }
}

const fn only(step: Step, only: Only) -> Step {
    Step { only, ..step }
}

const fn on_phone(step: Step, anchor: Anchor) -> Step {
    Step {
        phone_anchor: Some(anchor),
        ..step
    }
}

use Anchor as A;
use Kind::{Jit, Narrated as N, Offer, Text, Try};
use Phrase as P;

/// The lobby & room tour: 14 steps in 5 chapters (TOURS.md §2.1).
pub const LOBBY: &[Chapter] = &[
    Chapter {
        id: "door",
        name: P::TourChapterDoor,
        place: Place::Door,
        after: None,
        steps: &[
            step(
                "L1",
                Some(A::FrontGatewayRow),
                N,
                P::TourL1Title,
                P::TourL1Body,
            ),
            step("L2", Some(A::FrontGuest), N, P::TourL2Title, P::TourL2Body),
            step(
                "L3",
                Some(A::FrontTextRow),
                N,
                P::TourL3Title,
                P::TourL3Body,
            ),
        ],
    },
    Chapter {
        id: "play",
        name: P::TourChapterPlay,
        place: Place::Play,
        after: None,
        steps: &[
            step("L4", Some(A::ShellNav), N, P::TourL4Title, P::TourL4Body),
            step("L5", Some(A::PlayHero), N, P::TourL5Title, P::TourL5Body),
            step("L6", Some(A::PlayHouse), N, P::TourL6Title, P::TourL6Body),
            step("L7", Some(A::PlayTables), N, P::TourL7Title, P::TourL7Body),
        ],
    },
    Chapter {
        id: "create",
        name: P::TourChapterCreate,
        place: Place::Play,
        after: Some("play"),
        steps: &[
            step(
                "L8",
                Some(A::PlayCreate),
                Try(Check::CreateSheetOpen),
                P::TourL8Title,
                P::TourL8Body,
            ),
            step(
                "L9",
                Some(A::CreateSheet),
                Try(Check::InRoom),
                P::TourL9Title,
                P::TourL9Body,
            ),
        ],
    },
    Chapter {
        id: "room",
        name: P::TourChapterRoom,
        place: Place::Room,
        after: None,
        steps: &[
            step(
                "L10",
                Some(A::RoomSeats),
                N,
                P::TourL10Title,
                P::TourL10Body,
            ),
            step(
                "L11",
                Some(A::RoomMySeatMenu),
                N,
                P::TourL11Title,
                P::TourL11Body,
            ),
            step(
                "L12",
                Some(A::RoomRules),
                N,
                P::TourL12Title,
                P::TourL12Body,
            ),
            step(
                "L13",
                Some(A::RoomStart),
                N,
                P::TourL13Title,
                P::TourL13Body,
            ),
        ],
    },
    Chapter {
        id: "closing",
        name: P::TourChapterClosing,
        place: Place::Play,
        after: Some("room"),
        steps: &[step("L14", None, Offer, P::TourL14Title, P::TourL14Body)],
    },
];

/// The deck builder tour: 14 steps in 4 chapters (TOURS.md §2.2 less D11b,
/// the phone's: the tours are desktop-only for now). The shelf's and the
/// builder's chapters open on their
/// screens; import, export and history are just-in-time, each the first
/// time its sheet stands.
pub const BUILDER: &[Chapter] = &[
    Chapter {
        id: "shelf",
        name: P::TourChapterShelf,
        place: Place::Shelf,
        after: None,
        steps: &[
            step("D1", Some(A::DecksTabs), N, P::TourD1Title, P::TourD1Body),
            step(
                "D2",
                Some(A::DecksTileActions),
                N,
                P::TourD2Title,
                P::TourD2Body,
            ),
            step(
                "D3",
                Some(A::DecksHouseTab),
                N,
                P::TourD3Title,
                P::TourD3Body,
            ),
            step("D4", Some(A::DecksNew), N, P::TourD4Title, P::TourD4Body),
        ],
    },
    Chapter {
        id: "editing",
        name: P::TourChapterEditing,
        place: Place::Builder,
        after: None,
        steps: &[
            step("D5", Some(A::BuildHeader), N, P::TourD5Title, P::TourD5Body),
            step("D6", Some(A::BuildCount), N, P::TourD6Title, P::TourD6Body),
            step(
                "D7",
                Some(A::BuildSearch),
                Try(Check::QueryTyped),
                P::TourD7Title,
                P::TourD7Body,
            ),
            step(
                "D8",
                Some(A::BuildFirstRow),
                Try(Check::DeckChanged),
                P::TourD8Title,
                P::TourD8Body,
            ),
            step("D9", Some(A::BuildDeck), N, P::TourD9Title, P::TourD9Body),
            step(
                "D10",
                Some(A::BuildFooter),
                N,
                P::TourD10Title,
                P::TourD10Body,
            ),
            step(
                "D11",
                Some(A::BuildSave),
                Try(Check::Saved),
                P::TourD11Title,
                P::TourD11Body,
            ),
            // D11b, the phone's one pane, is not here: the tours are
            // desktop-only for now (the owner, 09.10.2026).
        ],
    },
    Chapter {
        id: "lists",
        name: P::TourChapterLists,
        place: Place::Jit,
        after: None,
        steps: &[
            step(
                "D12",
                Some(A::ImportSheet),
                Jit,
                P::TourD12Title,
                P::TourD12Body,
            ),
            step(
                "D13",
                Some(A::ExportSheet),
                Jit,
                P::TourD13Title,
                P::TourD13Body,
            ),
        ],
    },
    Chapter {
        id: "history",
        name: P::TourChapterHistory,
        place: Place::Jit,
        after: None,
        steps: &[step(
            "D14",
            Some(A::HistorySheet),
            Jit,
            P::TourD14Title,
            P::TourD14Body,
        )],
    },
];

/// The table tour: 34 steps in 11 chapters plus the closing, in the run
/// order (TOURS.md §2.3: Reporting before Keys, so conceding is the last
/// word before the closing).
pub const TABLE: &[Chapter] = &[
    Chapter {
        id: "arrival",
        name: P::TourChapterArrival,
        place: Place::Table,
        after: None,
        steps: &[step("T1", None, Text, P::TourT1Title, P::TourT1Body)],
    },
    Chapter {
        id: "place",
        name: P::TourChapterPlace,
        place: Place::Table,
        after: None,
        steps: &[
            step("T2", Some(A::MyPod), N, P::TourT2Title, P::TourT2Body),
            on_phone(
                step("T3", Some(A::Hand), N, P::TourT3Title, P::TourT3Body),
                A::HandTab,
            ),
            step("T4", Some(A::HandCard), N, P::TourT4Title, P::TourT4Body),
            step("T5", Some(A::Shelf), N, P::TourT5Title, P::TourT5Body),
        ],
    },
    Chapter {
        id: "casting",
        name: P::TourChapterCasting,
        place: Place::Table,
        after: None,
        steps: &[
            step("T6", Some(A::LitCard), N, P::TourT6Title, P::TourT6Body),
            step("T7", None, Text, P::TourT7Title, P::TourT7Body),
            step(
                "T8",
                Some(A::Shelf),
                Try(Check::CastCancelled),
                P::TourT8Title,
                P::TourT8Body,
            ),
            step(
                "T9",
                Some(A::AbilitySheet),
                Jit,
                P::TourT9Title,
                P::TourT9Body,
            ),
        ],
    },
    Chapter {
        id: "stack",
        name: P::TourChapterStack,
        place: Place::Table,
        after: None,
        steps: &[
            step(
                "T10",
                Some(A::StackPanel),
                Jit,
                P::TourT10Title,
                P::TourT10Body,
            ),
            step(
                "T11",
                Some(A::StackControls),
                Jit,
                P::TourT11Title,
                P::TourT11Body,
            ),
            step("T12", Some(A::Shelf), N, P::TourT12Title, P::TourT12Body),
        ],
    },
    Chapter {
        id: "decisions",
        name: P::TourChapterDecisions,
        place: Place::Table,
        after: None,
        steps: &[
            step(
                "T13",
                Some(A::DecisionSheet),
                Jit,
                P::TourT13Title,
                P::TourT13Body,
            ),
            step(
                "T14",
                Some(A::DecisionFold),
                Jit,
                P::TourT14Title,
                P::TourT14Body,
            ),
            step("T15", None, Text, P::TourT15Title, P::TourT15Body),
        ],
    },
    Chapter {
        id: "phases",
        name: P::TourChapterPhases,
        place: Place::Table,
        after: None,
        steps: &[
            step("T16", Some(A::MySteps), N, P::TourT16Title, P::TourT16Body),
            step("T17", Some(A::Shelf), N, P::TourT17Title, P::TourT17Body),
            step("T18", Some(A::MyPod), N, P::TourT18Title, P::TourT18Body),
        ],
    },
    Chapter {
        id: "dial",
        name: P::TourChapterDial,
        place: Place::Table,
        after: None,
        steps: &[
            step("T19", Some(A::Dial), N, P::TourT19Title, P::TourT19Body),
            step("T20", Some(A::MyPlate), N, P::TourT20Title, P::TourT20Body),
        ],
    },
    Chapter {
        id: "seeing",
        name: P::TourChapterSeeing,
        place: Place::Table,
        after: None,
        steps: &[
            only(
                step(
                    "T21",
                    Some(A::ArrangementPill),
                    Try(Check::ArrangementChanged),
                    P::TourT21Title,
                    P::TourT21Body,
                ),
                Only::RingDesktop,
            ),
            only(
                step(
                    "T22",
                    Some(A::PlayersStrip),
                    Try(Check::Visited),
                    P::TourT22Title,
                    P::TourT22Body,
                ),
                Only::Ring,
            ),
            step("T23", None, Text, P::TourT23Title, P::TourT23Body),
            only(
                step(
                    "T24",
                    Some(A::PlayersStrip),
                    N,
                    P::TourT24Title,
                    P::TourT24Body,
                ),
                Only::Phone,
            ),
        ],
    },
    Chapter {
        id: "reveals",
        name: P::TourChapterReveals,
        place: Place::Table,
        after: None,
        steps: &[
            step(
                "T25",
                Some(A::RevealSheet),
                Jit,
                P::TourT25Title,
                P::TourT25Body,
            ),
            step(
                "T26",
                Some(A::TrayLog),
                Try(Check::LogOpened),
                P::TourT26Title,
                P::TourT26Body,
            ),
            step(
                "T27",
                Some(A::TrayZones),
                N,
                P::TourT27Title,
                P::TourT27Body,
            ),
        ],
    },
    Chapter {
        id: "reporting",
        name: P::TourChapterReporting,
        place: Place::Table,
        after: None,
        steps: &[
            step(
                "T31",
                Some(A::ReportCorner),
                Try(Check::ReportOpened),
                P::TourT31Title,
                P::TourT31Body,
            ),
            step(
                "T32",
                Some(A::ReportForm),
                N,
                P::TourT32Title,
                P::TourT32Body,
            ),
            step(
                "T33",
                Some(A::ReportForm),
                N,
                P::TourT33Title,
                P::TourT33Body,
            ),
        ],
    },
    Chapter {
        id: "keys",
        name: P::TourChapterKeys,
        place: Place::Table,
        after: None,
        steps: &[
            step("T28", Some(A::Burger), N, P::TourT28Title, P::TourT28Body),
            step("T29", Some(A::Burger), N, P::TourT29Title, P::TourT29Body),
            step("T30", Some(A::Burger), N, P::TourT30Title, P::TourT30Body),
        ],
    },
    Chapter {
        id: "closing",
        name: P::TourChapterClosing,
        place: Place::Table,
        after: None,
        steps: &[step("T34", None, Text, P::TourT34Title, P::TourT34Body)],
    },
];

/// This device's tour switches and what it has seen (`ClientSettings.tours`,
/// TOURS.md §1.7). Per device: a phone and a desk want different tours, and
/// guests and offline players have no account. A file from before it reads
/// every tour on and nothing seen.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
#[allow(clippy::struct_excessive_bools)] // four switches, as the settings screen draws them
pub struct Tours {
    /// The lobby & room tour.
    pub lobby: bool,
    /// The deck builder tour.
    pub builder: bool,
    /// The table tour.
    pub table: bool,
    /// The just-in-time steps, and the networked game's tips.
    pub tips: bool,
    /// Seen chapters (`"lobby/door"`) and seen JIT steps (`"table/T10"`).
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub seen: BTreeSet<String>,
}

impl Default for Tours {
    fn default() -> Self {
        Self {
            lobby: true,
            builder: true,
            table: true,
            tips: true,
            seen: BTreeSet::new(),
        }
    }
}

impl Tours {
    /// Whether this tour is switched on.
    #[must_use]
    pub const fn on(&self, tour: Tour) -> bool {
        match tour {
            Tour::Lobby => self.lobby,
            Tour::Builder => self.builder,
            Tour::Table => self.table,
        }
    }

    /// Switches one tour.
    pub const fn set(&mut self, tour: Tour, on: bool) {
        match tour {
            Tour::Lobby => self.lobby = on,
            Tour::Builder => self.builder = on,
            Tour::Table => self.table = on,
        }
    }

    /// The bubble's one box: every tour and the tips at once.
    pub const fn set_all(&mut self, on: bool) {
        self.lobby = on;
        self.builder = on;
        self.table = on;
        self.tips = on;
    }

    /// Settings › Restart tours: every seen mark gone.
    pub fn restart(&mut self) {
        self.seen.clear();
    }

    /// Whether a chapter was seen here.
    #[must_use]
    pub fn chapter_seen(&self, tour: Tour, chapter: &Chapter) -> bool {
        self.seen.contains(&mark(tour, chapter.id))
    }

    /// Whether a JIT step was seen here.
    #[must_use]
    pub fn step_seen(&self, tour: Tour, step: &Step) -> bool {
        self.seen.contains(&mark(tour, step.id))
    }

    /// The chapter of `tour` that opens at `place` now: the first one there
    /// not seen whose `after` was. `None` while the tour is off.
    #[must_use]
    pub fn due(&self, tour: Tour, place: Place) -> Option<usize> {
        if !self.on(tour) {
            return None;
        }
        tour.chapters().iter().position(|c| {
            c.place == place
                && !self.chapter_seen(tour, c)
                && c.after.is_none_or(|a| self.seen.contains(&mark(tour, a)))
                && c.steps.iter().any(|s| s.kind != Kind::Jit)
        })
    }
}

/// A seen mark: `"lobby/door"`, `"table/T10"`.
#[must_use]
pub fn mark(tour: Tour, id: &str) -> String {
    format!("{}/{id}", tour.key())
}

/// How the bubble stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The bubble holds the keys.
    Narrated,
    /// The bubble lets go; the screen has the keys.
    Try,
    /// Folded to the pill.
    Folded,
}

/// What a move along a run came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Moved {
    /// Another step of the run stands.
    Step,
    /// The run is over; its chapters are marked seen.
    Over,
}

/// One tour running: where it stands and how. A run is one chapter of the
/// lobby or the builder, the whole of the table, or one JIT step.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // independent facts about one bubble
pub struct Run {
    /// Which tour.
    pub tour: Tour,
    /// The chapter's index in [`Tour::chapters`].
    pub chapter: usize,
    /// The step's index in the chapter.
    pub step: usize,
    /// How the bubble stands.
    pub mode: Mode,
    /// The try-it check held since the step opened (latched).
    pub held: bool,
    /// The bubble folded itself for a question, and unfolds when it goes.
    pub auto_folded: bool,
    /// A single just-in-time step, not a chapter.
    pub single: bool,
    /// Steps passed over because their anchor was not there (hollow dots).
    pub skipped: Vec<&'static str>,
    /// Phone or not, and how many seats: which steps run.
    pub phone: bool,
    /// Seats at the table (the table tour's ring steps).
    pub seats: usize,
}

impl Run {
    /// A run of `chapter` from its first scripted step that runs here, or
    /// `None` if it has none.
    #[must_use]
    pub fn chapter(tour: Tour, chapter: usize, phone: bool, seats: usize) -> Option<Self> {
        let mut run = Self {
            tour,
            chapter,
            step: 0,
            mode: Mode::Narrated,
            held: false,
            auto_folded: false,
            single: false,
            skipped: Vec::new(),
            phone,
            seats,
        };
        loop {
            let chapters = tour.chapters();
            let c = chapters.get(run.chapter)?;
            if let Some(i) = (0..c.steps.len()).find(|&i| run.scripted(&c.steps[i])) {
                run.step = i;
                run.enter();
                return Some(run);
            }
            if !tour.continuous() {
                return None;
            }
            run.chapter += 1;
        }
    }

    /// A run of one just-in-time step.
    #[must_use]
    pub fn jit(tour: Tour, chapter: usize, step: usize, phone: bool) -> Self {
        let mut run = Self {
            tour,
            chapter,
            step,
            mode: Mode::Narrated,
            held: false,
            auto_folded: false,
            single: true,
            skipped: Vec::new(),
            phone,
            seats: 0,
        };
        run.enter();
        run
    }

    fn scripted(&self, step: &Step) -> bool {
        step.kind != Kind::Jit && step.only.runs(self.phone, self.seats)
    }

    fn enter(&mut self) {
        self.held = false;
        self.auto_folded = false;
        self.mode = if self.current().kind.holds() {
            Mode::Narrated
        } else {
            Mode::Try
        };
    }

    /// The chapter standing.
    #[must_use]
    pub fn current_chapter(&self) -> &'static Chapter {
        &self.tour.chapters()[self.chapter]
    }

    /// The step standing.
    #[must_use]
    pub fn current(&self) -> &'static Step {
        &self.current_chapter().steps[self.step]
    }

    /// The steps of this chapter the dots stand for: the scripted ones
    /// that run here, or the one JIT step.
    #[must_use]
    pub fn dots(&self) -> Vec<&'static Step> {
        if self.single {
            return vec![self.current()];
        }
        self.current_chapter()
            .steps
            .iter()
            .filter(|s| self.scripted(s))
            .collect()
    }

    /// Whether this is the last step of the run (the primary says Done).
    #[must_use]
    pub fn last(&self) -> bool {
        if self.single {
            return true;
        }
        let mut probe = self.clone();
        probe.step_on().is_none()
    }

    /// Whether a step before this one stands in the chapter (Back shows).
    #[must_use]
    pub fn has_back(&self) -> bool {
        !self.single
            && self.current_chapter().steps[..self.step]
                .iter()
                .any(|s| self.scripted(s))
    }

    /// The next step's place, without marking anything.
    fn step_on(&mut self) -> Option<(usize, usize)> {
        let chapters = self.tour.chapters();
        let mut chapter = self.chapter;
        let mut from = self.step + 1;
        loop {
            let c = chapters.get(chapter)?;
            if let Some(i) = (from..c.steps.len()).find(|&i| self.scripted(&c.steps[i])) {
                return Some((chapter, i));
            }
            if !self.tour.continuous() {
                return None;
            }
            chapter += 1;
            from = 0;
        }
    }

    /// Next (or Done): the next step, or the run is over and every chapter
    /// it walked through is marked seen in `tours`.
    pub fn next(&mut self, tours: &mut Tours) -> Moved {
        if self.single {
            tours.seen.insert(mark(self.tour, self.current().id));
            return Moved::Over;
        }
        let Some((chapter, step)) = self.step_on() else {
            tours
                .seen
                .insert(mark(self.tour, self.current_chapter().id));
            return Moved::Over;
        };
        if chapter != self.chapter {
            tours
                .seen
                .insert(mark(self.tour, self.current_chapter().id));
        }
        self.chapter = chapter;
        self.step = step;
        self.enter();
        Moved::Step
    }

    /// The step's anchor was not there when it would open: passed over,
    /// its dot hollow (TOURS.md §3.3), then on as [`Self::next`].
    pub fn pass_over(&mut self, tours: &mut Tours) -> Moved {
        self.skipped.push(self.current().id);
        self.next(tours)
    }

    /// Back: the previous step of the chapter, if there is one.
    pub fn back(&mut self) {
        if let Some(i) = (0..self.step)
            .rev()
            .find(|&i| self.scripted(&self.current_chapter().steps[i]))
        {
            self.step = i;
            self.enter();
        }
    }

    /// Skip: the chapter is marked seen; the table's run goes on with the
    /// next chapter, every other run is over.
    pub fn skip(&mut self, tours: &mut Tours) -> Moved {
        if self.single {
            return self.next(tours);
        }
        tours
            .seen
            .insert(mark(self.tour, self.current_chapter().id));
        if self.tour.continuous()
            && let Some(next) = Self::chapter(self.tour, self.chapter + 1, self.phone, self.seats)
        {
            let skipped = std::mem::take(&mut self.skipped);
            *self = next;
            self.skipped = skipped;
            return Moved::Step;
        }
        Moved::Over
    }

    /// The bubble folded by hand (Esc, its ×).
    pub fn fold(&mut self) {
        self.mode = Mode::Folded;
        self.auto_folded = false;
    }

    /// The pill pressed: the bubble back as its step wants it.
    pub fn unfold(&mut self) {
        self.auto_folded = false;
        self.mode = if self.current().kind.holds() {
            Mode::Narrated
        } else {
            Mode::Try
        };
    }

    /// The try-it check came true (latched for the step).
    pub fn hold(&mut self) {
        self.held = true;
    }

    /// Whether the primary works: always, but on a try-it step before its
    /// check held.
    #[must_use]
    pub fn primary_live(&self) -> bool {
        !matches!(self.current().kind, Kind::Try(_)) || self.held
    }

    /// Whether the bubble holds the keyboard now.
    #[must_use]
    pub fn holds_keyboard(&self) -> bool {
        self.mode == Mode::Narrated
    }

    /// Follows whether a question is pending for this seat (TOURS.md §1.5):
    /// a bubble that holds the keys folds itself while one is, and unfolds
    /// when it is answered unless the player folded it by hand meanwhile.
    /// What makes "no narrated step stands over a pending question" true by
    /// construction.
    pub fn follow_question(&mut self, pending: bool) {
        let holds = self.current().kind.holds();
        if pending && holds && self.mode == Mode::Narrated {
            self.mode = Mode::Folded;
            self.auto_folded = true;
        } else if !pending && self.auto_folded && self.mode == Mode::Folded {
            self.unfold();
        }
    }
}

/// Every step of every tour, with its tour (for the tests and the walk).
#[must_use]
pub fn every_step() -> Vec<(Tour, &'static Step)> {
    Tour::ALL
        .into_iter()
        .flat_map(|t| {
            t.chapters()
                .iter()
                .flat_map(move |c| c.steps.iter().map(move |s| (t, s)))
        })
        .collect()
}

/// Every phrase a tour says: the chapter names and each step's title and
/// words.
#[must_use]
pub fn every_phrase() -> Vec<Phrase> {
    let mut all: Vec<Phrase> = Phrase::ALL
        .iter()
        .copied()
        .filter(|p| format!("{p:?}").starts_with("Tour"))
        .collect();
    all.dedup();
    all
}

/// The bubble's budget, derived from the phone's bubble (TOURS.md §1.6):
/// 300 px wide less 2 × 16 of padding is 268 px of line; Alegreya Sans
/// Regular at the phone's 12 px averages about 5.8 px a character, so ≈ 46
/// a line; the body has about seven lines of the 234-px-tall bubble's
/// height left over, ≈ 320 characters. English gets 300 (it sets wider per
/// character than German's long words wrap).
pub const BODY_DE: usize = 320;
/// The English body's budget.
pub const BODY_EN: usize = 300;
/// A title is one line: h2 22 px over 268 px.
pub const TITLE: usize = 22;
