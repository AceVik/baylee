//! baylee-client — the Bevy duel client.
//!
//! # Shape
//!
//! Everything that *decides* lives in `baylee-client-core` and is tested
//! headlessly. This crate is the part that cannot be: meshes, textures, input
//! devices, and a camera.
//!
//! ```text
//!   host          a socket, or an engine in this process
//!     |  HostMessage
//!     v
//!   Duel          the client's state: static payload, latest view, board model
//!     |  BoardModel
//!     v
//!   table/hud     3D pods and cards, 2D overlay
//! ```
//!
//! # Embedding
//!
//! [`DuelPlugin`] is a plugin, not an application. The open-world client adds it
//! to an app it already owns, installs a host, and pushes [`DuelCommand::Open`]
//! when two players sit down — the duel takes over the screen and hands it back
//! on [`DuelCommand::Close`]. Nothing here creates a window or a schedule of its
//! own, and the standalone binary in `main.rs` is only the thinnest possible
//! wrapper around the same plugin.

#![warn(missing_docs)]
// The client converts small counts (seats, cards in a lane, list indices) to
// floats for layout. All are bounded by what fits on a table.
#![allow(clippy::cast_precision_loss)]
// Bevy's system-param contract takes `Res`, `Query` and friends by value —
// they *are* the parameter, and a reference to one is not a system param at
// all. The lint cannot see that, and firing it on every system would bury the
// cases where it is right.
#![allow(clippy::needless_pass_by_value)]

pub mod abilities;
pub mod ambience;
#[cfg(target_os = "macos")]
mod app_icon;
pub mod arrangement;
mod arrival;
pub mod arrowmat;
#[cfg(not(target_arch = "wasm32"))]
pub mod artreader;
pub mod badgemat;
pub mod buildui;
mod card_loading;
pub mod cardart;
pub mod cardmat;
pub mod cardtext;
pub mod castmodes;
pub mod choices;
mod combatfx;
pub mod combatlines;
/// The console a Windows release build opens only on `--console`.
pub mod console;
pub mod depart;
/// The dev-control harness. Native dev builds only; see the module docs for
/// why it is a compile-time feature rather than a runtime switch.
#[cfg(all(feature = "dev-control", not(target_arch = "wasm32")))]
pub mod devctl;
pub mod dial;
pub mod face;
pub mod feltmat;
pub mod filterui;
pub mod flip;
pub mod floormat;
pub mod frontal;
pub mod gpu;
pub mod hand_order;
pub mod host;
pub mod hud;
pub mod input;
pub mod keys;
pub mod lifeflash;
pub mod loading;
pub mod lobby;
pub mod manasources;
pub mod manaui;
pub mod markatlas;
pub mod marksmat;
pub mod matmat;
pub mod music;
pub mod net;
pub mod platemat;
pub mod prefs;
pub mod quality;
pub(crate) mod records;
pub mod report;
pub mod rowbar;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod seatbin;
pub mod seatpanel;
pub mod settings;
pub mod settingsui;
pub mod sheen;
pub mod shellkit;
pub mod shellmat;
pub mod shellui;
pub mod sky;
pub mod softkeys;
pub mod sound;
pub mod standalone;
pub mod table;
pub(crate) mod tablellm;
pub(crate) mod tableseats;
pub mod targeting;
pub mod textures;
pub mod tokenart;
pub mod touch;
pub(crate) mod transport;
pub mod unlit;
pub mod update;
pub mod vista;
mod window_icon;
mod yes_batch;

use baylee_client_core::automation::{self, AutoPilot, Situation};
use baylee_client_core::board::BoardModel;
use baylee_client_core::browser::Placement;
use baylee_client_core::i18n::{Phrase, Refusal};
use baylee_client_core::interaction::Interaction;
use baylee_client_core::layout::{Seat, TableLayout};
use baylee_client_core::reconnect::{Retry, Window};
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::{GameStatic, PlayerView};
use bevy::prelude::*;
use host::{DuelHost, HostMessage, LinkState};

pub use host::LocalHost;
pub use lobby::{LobbyPlugin, LobbyState};
pub use net::{NetworkHost, SeatTicket};

/// Whether a duel is on screen.
///
/// A state rather than a flag so that an embedding application can run its own
/// systems in [`DuelPhase::Closed`] and have every duel system stop cleanly,
/// without the duel having to know what else exists.
#[derive(States, Default, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum DuelPhase {
    /// No duel; the host application owns the screen.
    #[default]
    Closed,
    /// The table is prepared beneath its loading cover. Input remains
    /// disabled until render readiness and the shared entrance complete.
    Opening,
    /// A duel is on screen.
    Playing,
    /// The game has ended and the result is being shown.
    Finished,
}

/// Asks the duel to open or close, from an embedding application.
#[derive(Message, Clone, Debug)]
pub enum DuelCommand {
    /// Take over the screen; a host must already be installed.
    Open,
    /// Tear the duel down and return the screen.
    Close,
}

/// Something the duel tells the embedding application.
#[derive(Message, Clone, Debug)]
pub enum DuelReport {
    /// The game ended.
    Finished,
    /// Something went wrong; the string is safe to show a player.
    ///
    /// Not necessarily fatal, and deliberately not the signal to leave a
    /// table. The gateway's `Error` envelope carries the engine's refusal of
    /// a *single action* — "illegal action for your seat" — through the same
    /// door, so a shell that returned to the lobby on every one of these
    /// would eject a player for a misclick.
    Failed(String),
    /// The table cannot be reached and this client has stopped trying.
    ///
    /// The one report that does mean the duel is over as far as this client
    /// is concerned, which is why it is its own variant rather than another
    /// [`DuelReport::Failed`] string for a reader to pattern-match prose on.
    Unreachable,
}

/// The installed source of duel state.
#[derive(Resource)]
pub struct InstalledHost(pub Box<dyn DuelHost>);

/// The retry schedule for a table whose socket went away.
///
/// A resource rather than a field on [`Duel`] because it survives what `Duel`
/// does not: `Duel::default()` is written over the whole struct when a duel
/// **opens**, and a schedule that reset there would forget how long it had
/// been trying every time anything else about the duel changed.
///
/// It said "closes" until 20.09.2026 and the code has always said `Open`
/// (`handle_commands`). The difference is load-bearing now rather than
/// pedantic: `lobby::systems::came_back` reads the finished game's verdict
/// off `Duel` *after* the close, which is only sound because nothing clears
/// it there.
#[derive(Resource, Default)]
pub struct Reconnect {
    /// When to dial next.
    schedule: Retry,
    /// Whether the player has already been told this one is hopeless, so the
    /// report goes out once rather than once a frame.
    told: bool,
}

/// A tap that has been made and not yet sent.
///
/// There is no undo in the engine and there should not be one — a journaled,
/// deterministic kernel does not roll back — so the client's job is to make
/// the irreversible **two-stage**. The first tap arms; a second tap on the
/// same card, the confirm key, or the button in the prompt bar sends it;
/// cancel disarms with nothing on the wire.
///
/// Mana abilities are the exception and stay one tap. Floating mana is the
/// one cheap mistake in the game: it empties at end of step, and a wrong
/// colour is fixed by tapping another land. Confirming those would put a
/// second click on the most common action a player makes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Armed {
    /// The card or permanent the player tapped.
    pub object: ObjectId,
    /// What the second tap will do with it.
    pub deed: Deed,
}

/// What this client is currently proposing the player tap, and on whose
/// authority.
///
/// An enum with a variant per source rather than an `Option<&Armed>` beside an
/// `Option<&Plan>`: two options are four states, two of which are nonsense,
/// and the third source this grows in six months would arrive as a third
/// `Option` that every reader could go on ignoring. A `match` here has to name
/// it.
///
/// The two are not the same claim, which is the point of telling them apart at
/// all. [`Self::Armed`] is a **commitment** — the player picked something and
/// a second tap sends it, because the deed cannot be taken back.
/// [`Self::Owed`] is a **suggestion**: the engine is holding a CR 605.3a
/// window open, the plan says which lands would pay it, and nothing is armed,
/// because tapping a land for mana is one tap by the rule at the top of this
/// file — mana abilities are the exemption the arming contract was written
/// with, and a payment window is made of nothing else.
///
/// Which of the two a frame is in is [`Duel::proposing`]'s to say, and it says
/// the commitment: **an armed deed wins over an open window**, because a
/// window opening underneath a choice the player already made does not get to
/// relight the board around it.
#[derive(Debug, Clone, Copy)]
pub enum Proposing<'a> {
    /// Nothing at all: the ordinary state of the game.
    Nothing,
    /// A deed waiting on its second tap.
    Armed(&'a Armed),
    /// An open payment window, and the taps that would settle it.
    Owed(&'a baylee_client_core::manaplan::Plan),
}

/// Who is offering this seat a card that lies in a pile, if anyone.
///
/// The hand's two lights, asked of the cards the hand does not hold — a
/// graveyard, an exile pile, the command zone — which the table draws on the
/// pile and the zone browser on its rows (#242). Two answers rather than a
/// flag for the hand's reason: one is the game saying yes, the other is this
/// client offering to tap lands first, and a player who cannot tell them
/// apart cannot tell a click that acts from a click that spends the mana.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    /// The engine: a click plays it, casts it, or activates something on it,
    /// with what is already floating. Gold in the hand.
    Offered,
    /// This client: a click taps lands first and then casts it
    /// ([`Duel::reachable`]). Indigo in the hand.
    Taps,
}

/// What an [`Armed`] tap is waiting to do.
///
/// Two of the three are *intents* rather than built actions, and the third
/// carries the action so it can be re-checked. That is the same rule
/// [`ManaRun`] follows step by step: between the two taps the engine may have
/// withdrawn the option, so every path that fires one of these resolves it
/// against the *current* `LegalActions` and disarms instead of guessing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Deed {
    /// Cast the spell, or play the land — `Interaction::play_card`.
    Play,
    /// Activate exactly this ability.
    ///
    /// The action and not its position in the chooser: a list rebuilt after
    /// the engine withdrew an earlier entry would shift under a stored
    /// index and fire the neighbour. Membership in the rebuilt list is
    /// checked before it is sent, so an ability that is gone disarms.
    Ability(PlayerAction),
    /// Suspend the card — `Interaction::suspend`.
    ///
    /// Its own deed rather than a shape of [`Self::Play`], because it is not
    /// playing the card: "rather than cast this card from your hand, pay {U}
    /// and exile it with four time counters on it" (CR 702.62a).
    Suspend,
    /// Tap the sources of this plan, then do `then` — see [`ManaRun`].
    Run {
        /// The taps, and the cost they are for.
        plan: baylee_client_core::manaplan::Plan,
        /// What the floated mana is spent on when the last tap is made.
        then: RunEnd,
    },
}

/// What a mana run does once the mana is up.
///
/// The run makes mana for a particular cast, suspend or activation. That
/// action is remembered from confirmation and rechecked against the engine's
/// actual offer after the final tap; it is never guessed from the new list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunEnd {
    /// Activate this exact ability after its mana has been made.
    Ability(u32),
    /// `PlayerAction::CastSpell`.
    Cast,
    /// `PlayerAction::Suspend`.
    Suspend,
    /// Nothing at all: the taps **are** the point.
    ///
    /// A mana bubble pours one mana into the pool and the player spends it
    /// themselves, so the run is a single step with a colour already in its
    /// hand. That is also the whole of how "the card taps only once the mana
    /// has been chosen" is arranged — the activation is not sent until the
    /// pip is pressed, and `asking` answers the engine's `ChooseColor` on the
    /// very next frame.
    Float,
    /// Settle a payment window (CR 605.3a, `PlayerView::owed`): once the
    /// taps are made, pass, which is what pays it — a miracle is cast, a tax
    /// paid. The pass is sent only after the last tap, so the engine hears
    /// "pay" with the mana already floating; a run that stops early leaves
    /// the window open and the player to finish it.
    Settle,
}

/// The client's own question: which of several ways to cast one card.
///
/// It is the same chooser the engine's own `Pending::ChooseCastMode` opens —
/// [`Self::prompt`] builds a real `Prompt::CastMode` and the prompt bar draws
/// it through [`crate::choices::options`] like any other — asked one step
/// earlier. The engine counts a spell's ways against the mana that is already
/// floating and cannot do otherwise; this client is what floats that mana, so
/// it is the one that has to ask first. [`crate::castmodes`] is where the ways
/// come from, and its module doc is what will not be offered.
///
/// The answer is remembered in [`Duel::cast_answer`] rather than here, because
/// it outlives the menu: the menu closes on the press that picks a row, and
/// the engine asks its own question several round trips later.
#[derive(Debug, Clone)]
pub struct CastMenu {
    /// The card in hand the chooser is about.
    pub card: ObjectId,
    /// The ways, in the order they are drawn.
    pub modes: Vec<crate::castmodes::ReachableMode>,
    /// Which row the cursor is on.
    pub pick: usize,
}

impl CastMenu {
    /// The question, in the shape the prompt bar already knows how to draw.
    ///
    /// Built on demand rather than stored beside [`Self::modes`]: two copies
    /// of one list are two things that can disagree, and this one is cheap —
    /// it is read only while the chooser stands.
    #[must_use]
    pub fn prompt(&self) -> baylee_client_core::interaction::Prompt {
        baylee_client_core::interaction::Prompt::CastMode {
            object: self.card,
            options: self
                .modes
                .iter()
                .enumerate()
                .map(|(i, m)| baylee_engine::choice::CastModeDesc {
                    // This client's own numbering, and it never leaves the
                    // client: the answer travels as a `CastModeKind` and is
                    // matched against the engine's list when the engine
                    // finally asks, because the engine's index is a position
                    // in a list built against the pool at that instant — the
                    // very pool this chooser is about to change.
                    index: u8::try_from(i).unwrap_or(u8::MAX),
                    kind: m.kind,
                    cost: m.cost,
                })
                .collect(),
        }
    }

    /// The way row `at` stands for, if the list still has that row.
    #[must_use]
    pub fn mode(&self, at: usize) -> Option<&crate::castmodes::ReachableMode> {
        self.modes.get(at)
    }
}

/// What the hover preview has to stand beside.
///
/// A tooltip stands beside the thing it describes, and the two variants are
/// the two qualities of answer the client can give to "where is that thing".
///
/// For a permanent on the felt the answer is exact: the card is a quad with a
/// `GlobalTransform`, so its four corners project to a screen rectangle and
/// the panel can open at its edge. That is [`Self::Card`], and it is what the
/// pointer's own position could never be — the pointer enters a card at its
/// rim, so a panel opened beside *the pointer* opens on top of the card it is
/// describing, which is exactly what it did.
///
/// For a row in the zone browser or an entry in the stack panel there is no
/// such rectangle to hand at the moment the hover is recorded, so the panel
/// falls back to standing beside the pointer, the way an ordinary tooltip
/// does.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum HoverSpot {
    /// Beside the pointer, in logical pixels.
    Point(Vec2),
    /// Beside the card's own screen rectangle, in logical pixels.
    Card(Rect),
}

/// How long another seat's turn must last before *Tisch folgt dem Zug*
/// shows it, in seconds: the house's turns that pass in a blink are skipped
/// and the camera lands where something happens (the owner, 08.10.2026).
pub const FOLLOW_DWELL: f32 = 1.0;

/// The client's own state for one duel.
#[derive(Resource, Default)]
// This is the client's bag of screen state, and each bool is a different
// thing standing open or not: a menu, a preview, a run of taps. They are not
// a mode — most pairs of them occur together — so an enum would have to
// enumerate the product and would be read back through the same number of
// comparisons.
#[allow(clippy::struct_excessive_bools)]
pub struct Duel {
    /// Finite identical may decisions explicitly approved together.
    pub(crate) yes_batch: yes_batch::YesBatch,
    pub(crate) combat_auto: Option<baylee_client_core::combat_auto::CombatAuto>,
    /// Page of the current legal target list.
    pub target_page: usize,
    /// Show legal targets controlled by this seat; `None` shows every seat.
    pub target_filter: Option<PlayerId>,
    /// Stable random surface for this local duel lifetime.
    pub table_pattern: feltmat::TablePattern,
    /// Local, explicit ordering of the visible hand.
    pub hand_order: hand_order::HandOrder,
    /// Group boundaries in the sorted presentation hand.
    pub hand_groups: Vec<hand_order::HandGroup>,
    /// The once-per-game payload.
    pub statics: Option<GameStatic>,
    /// The most recent snapshot.
    pub view: Option<PlayerView>,
    /// The render model derived from it.
    pub board: Option<BoardModel>,
    /// The choice being answered, if any.
    pub interaction: Option<Interaction>,
    /// Seat geometry for the current table.
    pub layout: Option<TableLayout>,
    /// The seat the camera is visiting (DESIGN-v7 §2): `None` is home, the
    /// whole ring. A visit moves the camera and nothing else — no card, no
    /// lane, no seat: it never reaches the layout ([`rebuild_board`] seats
    /// the ring without it), so every `Motion` target stands still through
    /// it. Set by a seat's button and `F`/`Shift+F`
    /// ([`input::navigate_to_player`]); cleared by `H`, `Esc`, my own button,
    /// the visited button again, my turn beginning and my own combat
    /// question ([`input::navigate_home`]); read by [`table::frame_table`].
    pub visiting: Option<PlayerId>,
    /// How the seats are placed at this table — the arrangement in
    /// **effect**: this game's switch, else the device's choice for this
    /// seat count, else its default, refused to the ring where it is not
    /// offered and resolved where it is a rule (DESIGN-v8 §2.5–§2.6).
    /// Written by [`arrangement::choose`], read by [`rebuild_board`] (which
    /// has no settings) and by the camera.
    pub arrangement: baylee_client_core::tableview::Arrangement,
    /// The arrangement chosen for this game only: a switch made in the menu
    /// with *remember for this seat count* unticked (DESIGN-v8 §2.6). Wins
    /// over the device's settings until the game ends.
    pub arrangement_game: Option<baylee_client_core::tableview::Arrangement>,
    /// The arrangement menu, while it is open: the row the keyboard stands
    /// on (DESIGN-v8 §2.1, §2.3). The player's, like [`Self::game_menu`].
    pub arrangement_menu: Option<usize>,
    /// The menu's last row, *remember for this seat count*: ticked, a
    /// choice is written to the device's per-count memory; unticked, it is
    /// this game's alone.
    pub arrangement_remember: bool,
    /// *Tisch folgt dem Zug*, as this device's settings say (copied by
    /// [`arrangement::choose`]; DESIGN-v8 §1.1).
    pub follow: bool,
    /// A seat the follow switch is waiting to show: a turn changed hands
    /// while a question was open for me or the pointer was reading the
    /// board shown; shown at the first view that allows it, dropped at the
    /// next turn's start.
    pub follow_pending: Option<PlayerId>,
    /// When the seat in [`Self::follow_pending`] began its turn, on
    /// [`Self::follow_now`]'s clock: the follow switch moves only to a turn
    /// that has lasted [`FOLLOW_DWELL`] (the owner, 08.10.).
    pub follow_since: f32,
    /// The follow switch's clock, in seconds, told once a frame
    /// ([`arrangement::follow_after_the_dwell`]).
    pub follow_now: f32,
    /// The follow switch moved the table and it has not settled yet: a
    /// `Space` pressed now is dropped (DESIGN-v8 §1.1 — a card that moved
    /// under the finger is not the card the finger meant).
    pub follow_settling: bool,
    /// The game menu's arrangement row was pressed: the menu opens on the
    /// next frame ([`arrangement::choose`], which holds the settings).
    pub arrangement_menu_asked: bool,
    /// What [`Self::arrangement`] was resolved from, and at how many seats:
    /// the Turntable with rows is decided once per table and held, so a
    /// resize never flips it while the game goes on.
    pub arrangement_latch: Option<(
        baylee_client_core::tableview::Arrangement,
        usize,
        baylee_client_core::tableview::Arrangement,
    )>,
    /// The tear under way, if one is (the owner's of 07.10.2026: a layout
    /// arrangement's change of seat tears the table open, swings the new
    /// seat across and docks it — `layout::transition`). While it runs,
    /// [`Self::layout`] is the stage the cards glide to and the tear holds
    /// the instant layout the stages end on.
    pub tear: Option<Tear>,
    /// The seat of interest the layout was last solved round
    /// ([`rebuild_board`]): a layout arrangement whose [`Self::visiting`]
    /// differs from it is seated again ([`arrangement::lay_the_interest`]),
    /// whoever moved the interest.
    pub interest_laid: Option<PlayerId>,
    /// The hand's drawer on a phone (DESIGN-v8 WA11,
    /// `client-core::handdrawer`): what the player last did with it.
    pub hand_drawer: baylee_client_core::handdrawer::HandDrawer,
    /// How far open the drawer is drawn, 0 to 1 (`hud::hand_drawer`).
    pub hand_shown: f32,
    /// Whether the question's sheet is folded to its pill (the owner,
    /// 08.10.2026): the fold belongs to the question it was made on.
    pub decision_fold: baylee_client_core::decisionfold::DecisionFold,
    /// Whether another seat's reveal is folded to its pill, keyed on the
    /// reveal's number: the next reveal stands up open.
    pub reveal_fold: baylee_client_core::decisionfold::DecisionFold,
    /// Whether the drawer stands open as drawn — the player's choice and
    /// the question's, and open while the table is still being prepared —
    /// which is what the table's canvas is framed against
    /// (`Canvas::with_drawer`). Written by `hud::hand_drawer::slide_the_hand`.
    pub hand_drawn_open: bool,
    /// The card the pointer or keyboard cursor is on.
    pub hovered: Option<ObjectId>,
    /// The *place* the pointer is on, for a pile that is drawn through no
    /// card of its own.
    ///
    /// A second field rather than a second meaning for [`Self::hovered`],
    /// because the two are different kinds of thing and only one of them can
    /// be previewed. In practice it is only ever a library: every other pile
    /// is found through its top card, which is an object like any other, and
    /// a library is face down to everybody — its owner included, CR 401.2 —
    /// so it has no card to be found by.
    pub hovered_pile: Option<(PlayerId, baylee_client_core::PileKind)>,
    /// What the preview should stand beside, in logical pixels.
    ///
    /// `None` when the hover came from the keyboard cursor, which has no
    /// position on the screen at all.
    pub hovered_at: Option<HoverSpot>,
    /// A card a line of the game log names, under the pointer in the log
    /// (#300): previewed as the line showed it, in place of [`Self::hovered`]
    /// while it lasts.
    pub hovered_log: Option<hud::LogHover>,
    /// The aspect ratio of the part of the window the table is *seen*
    /// through, once anything has measured it.
    ///
    /// Not the window's. The HUD is on top of the battlefield rather than
    /// beside it and covers about a fifth of it, so a table laid out against
    /// the window is a table the camera then has to fit into something else.
    /// `TableLayout` is built from this; until a frame has been drawn there
    /// is no window to ask, and `None` means "assume a wide screen", which is
    /// the hard-coded `16.0 / 9.0` this replaces.
    pub canvas_aspect: Option<f32>,
    /// The window's class, measured with [`Self::canvas_aspect`]: a phone's
    /// ring is laid on the ellipse, every other window's may be packed on a
    /// frame (`TableLayout::arranged_in`). `None` until measured, read as
    /// a wide window.
    pub canvas_frame: Option<baylee_client_core::tableview::TableFrame>,
    /// The engaged autopilot, if any ("next phase" / "end turn").
    pub autopilot: Option<AutoPilot>,
    /// Stack entry chosen as the next manual response boundary.
    pub stack_selected: Option<ObjectId>,
    /// A requested boundary must also suppress the client's phase autopilot.
    pub stack_stop_requested: bool,
    /// The standing orders the engine has been sent this game: the ones for
    /// cards in [`Duel::known_cards`], as they stood when last sent.
    ///
    /// Not cleared when a `GameStatic` arrives. The engine keeps a seat's
    /// automation for the whole game, and that payload is re-sent every time
    /// the seat earns a printing, so clearing it re-sent every order each
    /// time a new card came into view, and every one made the engine
    /// re-offer its question (#285).
    pub ability_orders_applied: Vec<automation::AbilityOrder>,
    /// Every card this game has shown this seat, from each view's own walk
    /// ([`PlayerView::cards`]); it only grows.
    ///
    /// A standing order goes out only for a card in here (#285). The host is
    /// told nothing about the account's other decks, and a join sends the
    /// orders this table can use rather than every one the account holds.
    /// An order about an opponent's card goes out the view that card first
    /// shows up in, which is before its ability can be on the stack.
    pub known_cards: std::collections::BTreeSet<baylee_core::ids::CardIndex>,
    /// Whether the table is open (#256). Until it is, the engine reads
    /// nothing this seat sends, so the outbox is held rather than sent:
    /// the standing ability orders go out the moment there is a view, and
    /// would otherwise be dropped. Set by `HostMessage::Curtain` and never
    /// cleared for the rest of the game.
    pub curtain_up: bool,
    /// Whether this seat has said it has drawn its table since it last
    /// attached; set only after the renderer acknowledges the prepared table.
    pub ready_sent: bool,
    /// Hand bar scroll offset in pixels.
    pub hand_scroll: f32,
    /// The first card each scrolled battlefield row shows
    /// (`table::follow_the_rows`, `hud::scrolls`).
    pub rows: baylee_client_core::rowscroll::RowScroll,
    /// Whether the preview resize handle is being dragged.
    pub resize_drag: bool,
    /// The zone browser's sheet being moved or stretched, and where the
    /// pointer was on the frame before.
    ///
    /// The cursor is recorded at the press rather than the delta being read
    /// from `MouseMotion`, because the two are different units: motion events
    /// are raw device counts and a `Node`'s `left` is logical pixels. The
    /// preview's resize gets away with the raw ones only because it multiplies
    /// them by an arbitrary constant and clamps; a sheet that has to end up
    /// under the pointer cannot.
    pub tray_drag: Option<crate::hud::TrayDrag>,
    /// Where the sheet stood before it was maximised, so the ⤢ corner can put
    /// it back.
    ///
    /// In memory and not in `ClientSettings`, deliberately: what is worth
    /// carrying to the next session is where the player *left* the sheet, and
    /// that is what `zone_browser` already holds. A restore target is a fact
    /// about this click and the one that undoes it.
    pub tray_restore: Option<Placement>,
    /// The taps the client is making on the player's behalf, if any.
    pub mana_run: Option<ManaRun>,
    /// Cards in hand that are not castable yet and would be after tapping.
    ///
    /// Kept beside the board model rather than in it: it is a *client*
    /// judgement, not something the engine said, and the difference is worth
    /// keeping visible at the type level.
    pub reachable: std::collections::HashSet<ObjectId>,
    /// Cards in hand that are not suspendable yet and would be after tapping.
    ///
    /// [`Self::reachable`] for the other thing a hand card can be paid for.
    /// The two are kept apart because a card in both is two deeds and the
    /// click has to ask which, and because the run that spends the mana must
    /// know which list to look in when it finishes.
    pub suspend_reach: std::collections::HashSet<ObjectId>,
    /// Permanents whose unpaid ability this client can fund automatically.
    pub ability_reach: std::collections::HashSet<ObjectId>,
    /// Permanents the engine listed at least one activatable ability for.
    ///
    /// The engine's own answer, unlike [`Self::reachable`] — `LegalActions`
    /// names every source whose ability may be activated right now, mana
    /// abilities included. Kept here so the table can draw it and the board
    /// model does not have to recompute it per frame.
    pub activatable: std::collections::HashSet<ObjectId>,
    /// What the answer being built proposed when the board was last built.
    ///
    /// A proposal is part of what the board merges on
    /// (`baylee_client_core::board::Proposal`), so `table::track_proposals`
    /// compares the answer against this every frame and rebuilds when they
    /// part.
    pub proposed: std::collections::HashMap<ObjectId, baylee_client_core::board::Proposal>,
    /// The permanent whose abilities the prompt bar is offering.
    ///
    /// Only ever set for one with more than one thing to do: a single
    /// ability activates on the click that found it, because a menu of one is
    /// a menu that only ever wastes a tap.
    pub ability_menu: Option<ObjectId>,
    /// Explicit temporary-action selection; life is never spent by opening it.
    pub granted_menu: baylee_client_core::granted::Draft,
    /// The card whose *ways to be cast* the prompt bar is offering.
    ///
    /// [`ability_menu`](Self::ability_menu)'s sibling, on the same rule: only
    /// ever set for a card with more than one way, because a chooser of one
    /// row only ever costs a press. See [`CastMenu`] for why the client asks
    /// this at all and [`crate::castmodes`] for what it may ask about.
    pub cast_menu: Option<CastMenu>,
    /// The way the player picked, and the card they picked it for.
    ///
    /// It outlives the chooser on purpose. Between the press that answers
    /// this client's question and the engine asking its own there is a whole
    /// mana run — several round trips, each one a fresh `Pending` — and the
    /// answer has to survive all of it. It is **not** carried on the
    /// [`ManaRun`]: `advance_mana_run` clears the run on the frame it sends
    /// `CastSpell`, and the `ChooseCastMode` arrives after that, to no run at
    /// all. The other half of the same argument is the free alternative cost,
    /// which has no run in the first place.
    ///
    /// A [`baylee_engine::choice::CastModeKind`] and never an index: the
    /// engine numbers its options by position in a list it rebuilds against
    /// the pool of the moment, and the moment has moved.
    pub cast_answer: Option<(ObjectId, baylee_engine::choice::CastModeKind)>,
    /// Which entry of that menu the keyboard is on.
    ///
    /// A menu the pointer can answer and the keyboard cannot is not a menu,
    /// it is a trap — and it was one: the chooser used to swallow every key
    /// except cancel. Reset whenever the menu opens, and clamped to the list
    /// as it is rebuilt, because the engine may withdraw an ability while
    /// the menu stands.
    pub ability_pick: usize,
    /// Which page of that menu the sheet is showing.
    ///
    /// Nine rows fit, because a row is sent by the digit drawn on it and
    /// there are nine digits that are not zero
    /// ([`baylee_client_core::abilitysheet`]). A tenth ability is real —
    /// a land under a Chromatic Lantern is granted a mana ability on top of
    /// what it prints — and turns the page rather than taking a key nobody
    /// would guess. Reset with [`Self::ability_pick`] when the sheet opens.
    pub ability_page: usize,
    /// Which tap of that permanent the sheet has stepped *into*.
    ///
    /// The sheet is then a bubble of that tap's colours and nothing else —
    /// the owner's third point, *"wenn man den Effekt auswählt … es wird
    /// wieder der Mana Dialog angezeigt"*. It is set by pressing a mana row
    /// the pips could not stand for, because such a row pours a number this
    /// side cannot count and the only question it has left is which colour.
    ///
    /// Read through [`Self::asking_tap`] and never directly: it is only ever
    /// meaningful about the permanent [`Self::ability_menu`] names, so a
    /// value left behind by a sheet that has closed is inert rather than
    /// something every write site of that field has to remember to clear.
    pub ability_tap: Option<u32>,
    /// The zone browser: every zone a choice can reach that the table
    /// cannot draw.
    ///
    /// Beside the interaction rather than inside it, and holding no
    /// selection of its own: the interaction is the one truth about the
    /// answer being assembled, and two copies of a selection are two things
    /// that can disagree. What lives here is only what the *player* said
    /// about the panel — open, which tab, what is typed.
    pub browser: baylee_client_core::browser::Browser,
    /// What every seat's life total was last time a view arrived, and what
    /// is being drawn about the ones that have changed since.
    ///
    /// Beside the view for the reason the browser is beside the interaction:
    /// it is a reading *of* the view and has no truth of its own. It has to
    /// live somewhere that outlives the seat bar, which is rebuilt by the
    /// very change it is animating — see [`crate::lifeflash`].
    pub life_flash: baylee_client_core::lifeflash::Ledger,
    /// What this seat's hand and the table's counters were last time a view
    /// arrived — the ear's half of the same idea.
    ///
    /// A second reader beside [`Self::life_flash`] rather than a field inside
    /// it, because the two answer to different masters: the ledger's numbers
    /// are *drawn*, so it carries a clock and merges hits that arrive
    /// together, and this one is only ever heard, so it carries neither. See
    /// [`baylee_client_core::cue::Tally`] for why a draw of three needs no
    /// merge window at all.
    pub tally: baylee_client_core::cue::Tally,
    /// What this frame has decided is worth hearing.
    ///
    /// Beside the ledger it mostly reads, for the same reason the ledger is
    /// beside the view: it is a reading *of* the game and has no truth of its
    /// own. Filled on the edges — a view arriving, a question arriving, the
    /// engine refusing something — and emptied once per frame by
    /// [`crate::sound::play_the_cues`]. Nothing about audio is in it; see
    /// [`baylee_client_core::cue`].
    pub cues: baylee_client_core::cue::Cues,
    /// How long the awaited seat has left, counted between views.
    ///
    /// Kept here beside [`Duel::cues`] and [`Duel::tally`] because it is the
    /// same kind of thing: a reading of the view that only a *sequence* of
    /// views can produce. `decision_remaining_ms` is relative to the moment
    /// its view was built, so somebody has to hold the number and count, and
    /// the counting is what makes it a second of arc rather than a figure
    /// that changes when the table does.
    pub clock: baylee_client_core::decisionclock::DecisionClock,
    /// Every seat's decision clock, counted between views: what each
    /// player's plate draws beside it (`PlayerView::clocks`, owner
    /// 08.10.2026). [`Self::clock`] is the awaited seat's and rings; this
    /// one only counts.
    pub seat_clocks: baylee_client_core::decisionclock::SeatClocks,
    /// Strikes waiting for their visual presentation, read at snapshot edges.
    pub strikes: Vec<baylee_client_core::strike::Strike>,
    /// This game's log, as far as the host has told it (#262).
    ///
    /// One book per game, and this is where that is kept: `DuelCommand::Open`
    /// replaces the whole `Duel`, so the next game starts with an empty book
    /// rather than one that would read its log as a rewrite of the last
    /// game's. A reconnect keeps it, and the host's retelling from the start
    /// adds only what is missing.
    pub log: baylee_client_core::gamelog::LogBook,
    /// Cards another seat revealed, held up for this seat (`hud::revealed`).
    ///
    /// Read off the log as it arrives ([`baylee_client_core::reveals`]), so
    /// it lives and dies with [`Self::log`]: a new game starts with none, and
    /// a reconnect's telling of the log from its first line stands none up.
    pub reveals: baylee_client_core::reveals::Reveals,
    /// The game menu's "report a problem" was pressed: `report` opens its
    /// form on the next frame (#309). A flag rather than a call, because the
    /// menu's handler holds only the duel.
    pub report_asked: bool,
    /// What has been typed into the creature-type filter.
    ///
    /// It lives here and not on the `Interaction` because the interaction is
    /// rebuilt from scratch on every `HostMessage::Choice`, and a re-sent
    /// snapshot (a print table earned, a seat reattaching) would empty the
    /// box under the player's fingers. It is cleared when an action is sent
    /// and when a choice arrives that is not asking for a type.
    pub subtype_filter: String,
    /// Which letter group the creature-type chooser's full list shows while
    /// nothing is typed (`typechooser::GROUPS`). Kept with the filter, and
    /// cleared with it.
    pub subtype_group: usize,
    /// This seat's own decklist, one entry per card, where this client knows
    /// it (a game it hosts itself, `DuelHost::own_deck`): what the
    /// creature-type chooser's quick list counts (`choices::type_lists`).
    pub own_deck: Vec<baylee_core::ids::CardIndex>,
    /// Whether the concede button is waiting for its second press.
    ///
    /// There is no undo in the engine and conceding is the most irreversible
    /// thing in the game, so it is the one menu item that takes two presses.
    /// Any other click, any bound key and any choice arriving from the host
    /// disarm it — an armed button left standing across a turn would be a
    /// worse trap than no confirmation at all.
    pub concede_armed: bool,
    /// Whether the game menu is open.
    ///
    /// The burger at the shelf's right end, and the two ways out plus the
    /// version line behind it — `hud::ledge::menu`. Entirely the client's own
    /// state, like [`Self::ability_menu`], and cleared by `Esc`, by a second
    /// press of the button, by a press outside the panel and by either entry
    /// answering.
    ///
    /// It does **not** close when a new question arrives, which is the one
    /// place it parts company with the ability chooser. That chooser belongs
    /// to a choice and is meaningless once the choice has gone; this panel
    /// belongs to the *player*, the same way the zone browser does — it holds
    /// what this client is and the two ways out of the game, neither of which
    /// the engine's next question has anything to say about. A menu taken
    /// away because the opponent passed priority is a menu a player cannot
    /// read.
    pub game_menu: bool,
    /// Whether the game log's panel is open (#262).
    ///
    /// The player's, like [`Self::game_menu`]: no question opens or closes
    /// it, and it stays up while the game goes on under it, because a log is
    /// read beside the table and not instead of it.
    pub log_open: bool,
    /// Whether the AI log's panel is open.
    pub ai_log_open: bool,
    /// Whether an AI seat's mind has said anything this game: the AI log's
    /// door stands in the tray only from then on. Never in a release build,
    /// which has no AI log (`docs/protocol.md` §"An AI seat's reasoning").
    pub ai_log_heard: bool,
    /// What an AI seat's mind said and the AI log has not drawn yet:
    /// filled by `poll_host`, drained by `hud::update_ai_log`. A queue on
    /// the duel rather than a Bevy message, so every harness that runs
    /// `poll_host` runs it without registering anything.
    pub ai_said: Vec<baylee_protocol::v1::AiLog>,
    /// The tap that has been made and not sent — see [`Armed`].
    ///
    /// Deliberately *not* cleared when a choice arrives: `pump` hands the
    /// acting seat its question again whenever anybody says anything, an
    /// opponent's priority hold included, and a spell that disarmed itself
    /// because the other player pressed `F6` would be a worse trap than no
    /// confirmation at all. It is cleared on cancel, on firing, on arming
    /// something else, and when the game asks a question that is not this
    /// seat's priority — and it heals itself everywhere it is read, because
    /// every one of those paths re-resolves it first.
    pub armed: Option<Armed>,
    /// The taps that would pay what this seat owes, while a payment window is
    /// open for it.
    ///
    /// Cached rather than derived at the four places that draw a land,
    /// because it depends on **both** edges — the view carries `owed` and the
    /// pool, the pending carries the mana abilities that could pay it — and
    /// the two arrive in either order. [`Self::refresh_owed_plan`] is called
    /// from both for that reason, and neither one alone is enough.
    pub owed_plan: Option<baylee_client_core::manaplan::Plan>,
    /// Actions waiting to be sent.
    outbox: Vec<PlayerAction>,
    /// The last thing that went wrong, shown in the prompt bar.
    ///
    /// A [`Refusal`] and not a `String`, because two different authorities
    /// write here: this client, whose sentences are its own and are
    /// translated, and the engine process, whose sentence is a fact about
    /// the game and arrives as prose. A `String` could hold only the second
    /// and so made the first English — #121. `Refusal`'s own doc has the
    /// argument and names `link_note` four fields down as the precedent.
    pub last_error: Option<Refusal>,
    /// What the connection to the table is doing, when that is worth saying.
    ///
    /// A phrase rather than a rendered string so the decision stays in
    /// [`keep_the_table_connected`], where a test can read it, and the words
    /// stay in the overlay, which is the only thing that knows the language.
    /// Deliberately not `last_error`: that clears in [`Duel::submit`], a call
    /// a disconnected player cannot make, so the notice would have outlived
    /// the disconnection it described.
    pub link_note: Option<Phrase>,
}

impl Duel {
    /// The result, once the game has one.
    ///
    /// `Pending::GameOver` is the only question that is not a question: it
    /// carries the answer instead of asking for one, and three readers want
    /// it — the veil that darkens a table nobody is playing on any more, the
    /// end screen, and the prompt slip, which uses it to *stop* speaking.
    /// Written once here rather than matched three times.
    #[must_use]
    pub fn ending(&self) -> Option<&baylee_engine::win::GameResult> {
        match self.interaction.as_ref()?.pending() {
            Pending::GameOver(result) => Some(result),
            _ => None,
        }
    }

    /// The sentence the shelf speaks, if any: the client's own cast chooser
    /// while it stands, else the question being asked, else — with no
    /// question — the opening-hand wait (`Prompt::after_keeping`). Nothing
    /// once the game is over, which the end screen says instead.
    ///
    /// One method because two systems ask it: the ledge, which draws it, and
    /// the overlay, which rebuilds on it. Two copies of the chain would be
    /// two places for a third source to be added to one of.
    ///
    /// `texts` names the creatures a question is about in the player's
    /// language ([`crate::face::name_of`]); client-core decides which.
    #[must_use]
    pub fn headline(
        &self,
        lang: baylee_client_core::i18n::Lang,
        texts: &crate::cardtext::CardTexts,
    ) -> Option<String> {
        if self.ending().is_some() {
            return None;
        }
        let prompt = self
            .cast_menu
            .as_ref()
            .map(CastMenu::prompt)
            .or_else(|| {
                self.interaction
                    .as_ref()
                    .map(baylee_client_core::Interaction::prompt)
            })
            .or_else(|| {
                self.view
                    .as_ref()
                    .and_then(baylee_client_core::interaction::Prompt::after_keeping)
            })?;
        // Whose turn it is, for the one line that changes with it. A seat
        // holds priority on every turn at the table, so the bar has to be
        // told which one this is or it says "Your move" through the whole
        // game.
        let turn = self
            .view
            .as_ref()
            .map_or(baylee_client_core::Turn::Mine, |v| {
                baylee_client_core::Turn::of(v.active, v.seat)
            });
        let name = |id: baylee_core::ids::ObjectId| {
            let view = self.view.as_ref()?;
            Some(crate::face::name_of(view.object(id)?, view, texts))
        };
        if matches!(prompt, baylee_client_core::interaction::Prompt::Priority)
            && let Some(payment) = self
                .view
                .as_ref()
                .and_then(|v| v.owed.filter(|_| v.awaiting == Some(v.seat)))
        {
            return Some(baylee_client_core::interaction::payment_line(lang, payment));
        }
        Some(prompt.headline_naming_targets(
            lang,
            turn,
            self.statics.as_ref(),
            self.view.as_ref().is_some_and(|v| v.owed.is_some()),
            &name,
            &|source| {
                Some(crate::choices::target_label(
                    lang,
                    baylee_view::TargetRef::Object(source),
                    crate::choices::FaceNames {
                        view: self.view.as_ref(),
                        texts: Some(texts),
                    },
                    self.statics.as_ref(),
                ))
            },
        ))
    }

    /// Queues an action for the host.
    ///
    /// Queuing rather than sending directly keeps every mutation of the game on
    /// one system boundary, which is what lets input handlers stay plain
    /// functions of the board model.
    pub fn submit(&mut self, action: PlayerAction) {
        if !action.is_automation_setting()
            && self
                .interaction
                .as_ref()
                .is_some_and(|i| i.is_mine() && matches!(i.pending(), Pending::Priority { .. }))
        {
            self.stack_stop_requested = false;
        }
        if let PlayerAction::SetPriorityHold(hold) = &action {
            self.stack_stop_requested = matches!(
                hold,
                baylee_engine::choice::PriorityHold::UntilTopOfStack { .. }
                    | baylee_engine::choice::PriorityHold::Always
            );
            self.autopilot = None;
        }
        // Whatever was typed belonged to the question just answered.
        self.subtype_filter.clear();
        self.subtype_group = 0;
        // And so did the last refusal. Cleared here rather than when a new
        // question arrives, because the acting seat is re-sent its own
        // question every time anybody says anything — a refusal would have
        // flashed and been gone before it was read. It stands until this
        // player tries something else.
        self.last_error = None;
        // And so does a chime announcing a question this player is about to
        // answer. The standing orders and the autopilot answer in the same
        // half-frame that installs a question (`poll_host` → `run_autopilot`,
        // both in `DuelSet::Sync`, both ahead of the drain in
        // `DuelSet::Present`), so a question the player never saw makes no
        // sound. A player answering a question they *did* hear reaches this
        // line frames later, when the queue no longer holds it, and the call
        // is then the no-op it should be.
        self.cues.retract(baylee_client_core::cue::Cue::YourMove);
        // And a grant that only hands this action back says nothing
        // (DESIGN-v7 §4.3, rule 3).
        self.cues.note_own_action();
        self.outbox.push(action);
    }

    /// Installs the question the host is asking.
    ///
    /// The line this draws is what everything in it obeys: state that belongs
    /// to the *previous question* is cleared, state that belongs to *this
    /// player* is not. The acting seat is re-sent its own question every time
    /// anybody at the table says anything, so a refusal or an armed deed
    /// dropped here would be dropped by the opponent pressing `F6`.
    ///
    /// A method rather than a match arm because that is the only way a test
    /// can ask what a re-sent question does.
    /// A new view of the table.
    ///
    /// Two things beyond storing it, and they are here rather than at the
    /// message loop for the reason [`Self::receive_choice`] is: a method is
    /// the only shape a test can ask a question of. Cards the engine is
    /// *showing* this seat live in no zone the table can draw, so a reveal
    /// opens the sheet that does — on the edge, never per frame. And a life
    /// total that moved exists only as the difference between this view and
    /// the last one, so it has to be read on the edge too: a frame later the
    /// previous total is gone.
    pub(crate) fn receive_view(&mut self, view: PlayerView) {
        self.stack_selected = self
            .stack_selected
            .filter(|id| view.stack.iter().any(|item| item.id == *id));
        // One reading of the difference, two things told about it: the number
        // over the bar and the sound in the room are the same event on the
        // same clock, which is what `Change::started` is for.
        let strikes = baylee_client_core::strike::between(self.view.as_ref(), &view);
        self.strikes.extend(strikes);
        let changes = self.life_flash.read(&view.seats);
        self.cues.note_life(&changes, view.seat);
        // The same edge, the second reading: a card that arrived in this hand
        // and a creature that grew exist only as the difference between this
        // view and the last, and a frame later the last one is gone. Unlike
        // the life ledger this one draws nothing, so it is read here and
        // nowhere else.
        let flow = self.tally.read(&view);
        self.cues.note_flow(&flow);
        // Two more readings on the same edge for the priority cue's policy
        // and the turn's click: whether the table moved by somebody else
        // since this seat last acted, and whether the turn passed.
        if baylee_client_core::cue::moved_by_another(self.view.as_ref(), &view) {
            self.cues.note_foreign();
        }
        self.cues.note_turn(view.active, view.turn);
        // The third reading on the same edge, and the one that is a *reset*
        // rather than a difference: the clock is restarted from what this
        // view says, and whether the sounds are re-armed is its own question
        // — see `DecisionClock::RESTART`, which tells a new question from a
        // correction by size, because the view carries no question identity.
        // The shelf counts this seat's own clock only (owner via the PM,
        // 08.10.2026): another seat's time stands beside its plate
        // (`seat_clocks`), and a second copy mid-shelf read as a stray. In
        // the mulligans `awaiting` is this seat while it still decides, so
        // the rule is the same there.
        let mine = view.awaiting == Some(view.seat);
        self.clock
            .sync(view.decision_remaining_ms.filter(|_| mine), mine);
        self.seat_clocks.sync(&view.clocks);
        self.known_cards.extend(view.cards());
        // My turn beginning brings the camera home (DESIGN-v7 §2.4): an edge
        // read against the view before this one, never a state per frame.
        if baylee_client_core::tableview::comes_home(&baylee_client_core::tableview::HomeEdge {
            me: view.seat,
            active_before: self.view.as_ref().map(|v| v.active),
            active_now: Some(view.active),
            combat_question_opened: false,
        }) {
            self.visiting = None;
            self.follow_pending = None;
        }
        self.follow_the_turn(&view);
        self.view = Some(view);
        let was_choosing = self.browser.for_choice();
        if let Some(v) = self.view.as_ref() {
            self.browser.saw_reveal(v);
        }
        self.clear_hover_for_new_browser(was_choosing);
        self.refresh_owed_plan();
    }

    /// Works out afresh which lands would settle an open payment window.
    ///
    /// Both edges call it and neither is redundant: a view with no pending
    /// yet has an `owed` and no mana abilities to pay it with, and a pending
    /// arriving against a stale view would plan against the wrong pool.
    ///
    /// `owed` is the **total** and this passes it whole, because
    /// `manaplan::plan` spends the pool first by its own contract — so the
    /// plan is already for the remainder and nothing here subtracts anything.
    /// A second arithmetic would be a second thing to keep in step with the
    /// pool, which is the reason the view carries the total in the first
    /// place.
    pub(crate) fn refresh_owed_plan(&mut self) {
        self.owed_plan = self.compute_owed_plan();
    }

    fn compute_owed_plan(&self) -> Option<baylee_client_core::manaplan::Plan> {
        let view = self.view.as_ref()?;
        // Only while this seat is the one being asked. `owed` names what the
        // *awaited* seat owes, and the pair is one sentence.
        if view.awaiting != Some(view.seat) {
            return None;
        }
        let baylee_core::mana::ManaPayment::Fixed(cost) = view.owed? else {
            return None;
        };
        let legal = self.interaction.as_ref()?.legal_actions()?;
        let pool = view
            .seat(baylee_client_core::decision::resource_player(view))?
            .mana_pool;
        baylee_client_core::manaplan::plan(&cost, &pool, &manasources::sources(view, legal))
    }

    /// Whether this seat is in a payment window it owes a fixed amount in
    /// (CR 605.3a): a miracle's cost, a tax, a pact, a "you may cast it".
    #[must_use]
    pub fn paying(&self) -> bool {
        self.view.as_ref().is_some_and(|v| {
            v.awaiting == Some(v.seat)
                && matches!(v.owed, Some(baylee_core::mana::ManaPayment::Fixed(_)))
        })
    }

    /// The spell this seat may take back now (CR 732): the view's
    /// `casting`, which the engine names only in the caster's own view and
    /// only through the cast's own questions and its payment window, while
    /// this seat is the one asked. A cast an effect made, or a mana
    /// ability's colour asked inside the window, names none.
    #[must_use]
    pub fn cancellable_cast(&self) -> Option<ObjectId> {
        self.interaction.as_ref().filter(|i| i.is_mine())?;
        self.view.as_ref().and_then(|v| v.casting)
    }

    /// Takes the cast back (`PlayerAction::CancelCast`), and with it the run
    /// that was paying for it or waiting on its question: the engine puts
    /// the card, the lands and the floating mana back as they were, and a
    /// run that outlived it would tap them again. Nothing if no cast is
    /// cancellable now.
    pub fn cancel_cast(&mut self) {
        if self.cancellable_cast().is_none() {
            return;
        }
        self.mana_run = None;
        self.armed = None;
        self.cast_answer = None;
        self.submit(PlayerAction::CancelCast);
    }

    /// Pays what a payment window still owes: taps the owed plan's lands,
    /// then passes, which settles it ([`RunEnd::Settle`]). The confirm key
    /// and the shelf's pay button both come here.
    ///
    /// The plan is for the **remainder**: it is worked out against the pool
    /// (`manaplan::plan` spends the pool first), so lands the player tapped
    /// by hand are already counted and only the rest is tapped. With nothing
    /// left to tap, or nothing that could pay, it is a plain pass — the
    /// first pays, the second declines, which is the engine's to say.
    ///
    /// Returns whether a run was started; `false` means the caller sends the
    /// pass itself.
    pub fn pay_owed(&mut self) -> bool {
        if !self.paying() || self.mana_run.is_some() {
            return false;
        }
        let Some(plan) = self.owed_plan.clone().filter(|p| !p.is_empty()) else {
            return false;
        };
        let Some(first) = plan.steps.first().map(|s| s.source) else {
            return false;
        };
        self.mana_run = Some(ManaRun::new(plan, first, RunEnd::Settle));
        advance_mana_run(self);
        true
    }

    /// What this client is proposing, if anything.
    ///
    /// An armed deed wins: it is a commitment the player made, and a payment
    /// window that opened underneath it does not get to relight the board.
    /// The engine withdraws the arming path's own option anyway, and every
    /// reader re-resolves an `Armed` before it fires one.
    #[must_use]
    pub fn proposing(&self) -> Proposing<'_> {
        if let Some(armed) = self.armed.as_ref() {
            return Proposing::Armed(armed);
        }
        match self.owed_plan.as_ref() {
            Some(plan) => Proposing::Owed(plan),
            None => Proposing::Nothing,
        }
    }

    /// Who is offering this seat `object`, in the two lights of [`Reach`].
    ///
    /// The one predicate behind both drawings of a pile card, the table's and
    /// the zone browser's, so the two cannot light the same card differently.
    /// The engine's offer is its three lists — a land to play, a spell to
    /// cast, an ability to activate — because a pile card is not a
    /// permanent, and `activatable` alone would leave the Opt that Snapcaster
    /// Mage made castable dark once its `{U}` was floating.
    #[must_use]
    pub fn reach_of(&self, object: ObjectId) -> Option<Reach> {
        let offered = self.activatable.contains(&object)
            || self
                .interaction
                .as_ref()
                .and_then(Interaction::legal_actions)
                .is_some_and(|legal| {
                    legal.lands.contains(&object) || legal.castable.contains(&object)
                });
        if offered {
            Some(Reach::Offered)
        } else if self.reachable.contains(&object) || self.ability_reach.contains(&object) {
            Some(Reach::Taps)
        } else {
            None
        }
    }

    /// A newly opened chooser takes focus from the card that launched it.
    /// Re-sent views leave the player's hover inside that chooser intact.
    fn clear_hover_for_new_browser(&mut self, was_choosing: bool) {
        if !was_choosing && self.browser.for_choice() {
            self.hovered = None;
            self.hovered_at = None;
            self.hovered_log = None;
        }
    }

    /// A versioned source chooser has no current-object preview to inherit.
    fn clear_hover_for_source_choice(&mut self, pending: &Pending) {
        if let Pending::ChooseDamageSource { choice, .. } = pending
            && self.interaction.as_ref().and_then(Interaction::decision_id)
                != Some(baylee_client_core::interaction::DecisionId::Source(*choice))
        {
            self.hovered = None;
            self.hovered_at = None;
            self.hovered_log = None;
        }
    }

    /// A Raging River label is a question about one attacker, and the
    /// table's way of pointing at a card is the cursor: it goes onto the
    /// attacker as each label is asked, so the card is lifted on the table,
    /// stands in the preview, and the keyboard cursor starts from it. Played
    /// live, the two attackers looked alike and the preview showed whichever
    /// card the pointer had last crossed. Only when the label is new: a view
    /// re-sending the same question leaves a player's own hover alone.
    fn hover_the_river_label(&mut self, before: Option<ObjectId>) {
        let Some(attacker) = self
            .interaction
            .as_ref()
            .filter(|i| i.is_mine())
            .and_then(river_label)
        else {
            return;
        };
        if before != Some(attacker) {
            self.hovered = Some(attacker);
            self.hovered_at = None;
            self.hovered_log = None;
        }
    }

    /// Makes `pending` the question, keeping what the last one carries over,
    /// and returns the attacker the last one was a river label for.
    fn install_question(&mut self, pending: Pending, seat: PlayerId) -> Option<ObjectId> {
        let label_before = self.interaction.as_ref().and_then(river_label);
        self.interaction = Some(Interaction::new_keeping(
            pending,
            self.view
                .as_ref()
                .map_or(seat, baylee_client_core::decision::resource_player),
            self.interaction.as_ref(),
        ));
        label_before
    }

    pub(crate) fn receive_choice(&mut self, pending: Pending) {
        self.clear_hover_for_source_choice(&pending);
        self.target_page = 0;
        self.target_filter = None;
        let seat = self.seat().unwrap_or(PlayerId::new(0));
        if !matches!(
            pending,
            Pending::ChooseSubtype { .. } | Pending::ChooseCardName { .. }
        ) {
            self.subtype_filter.clear();
            self.subtype_group = 0;
        }
        let combat_before = self.interaction.as_ref().is_some_and(my_combat_question);
        let label_before = self.install_question(pending, seat);
        self.refresh_owed_plan();
        // My own attackers or blockers being asked for brings the camera home
        // as the question opens: its subject is my board (DESIGN-v7 §2.4).
        // My priority, a target or a yes-no do not — a player visits a seat
        // to target what is on it.
        if baylee_client_core::tableview::comes_home(&baylee_client_core::tableview::HomeEdge {
            me: seat,
            active_before: None,
            active_now: None,
            combat_question_opened: !combat_before
                && self.interaction.as_ref().is_some_and(my_combat_question),
        }) {
            // A layout arrangement keeps my board on the felt whatever the
            // seat of interest is, so my blockers question brings the
            // attacker across instead of the side across the ring: the
            // creatures to block stand on the felt (DESIGN-v8 §1.1).
            let blocking = self
                .interaction
                .as_ref()
                .is_some_and(|i| matches!(i.pending(), Pending::ChooseBlockers { .. }));
            let attacker = self.view.as_ref().map(|v| v.active).filter(|a| *a != seat);
            self.visiting = if self.arrangement.moves_cards() && blocking {
                attacker
            } else {
                None
            };
        }
        // The flank, not the state: `Cues` remembers whether the last
        // question was this seat's, so the acting seat being re-sent its own
        // question — which happens every time anybody at the table says
        // anything — is silent. A game ending is addressed to nobody, so it
        // clears the flag on its way past.
        self.cues
            .note_question(self.interaction.as_ref().is_some_and(Interaction::is_mine));
        if let Some(result) = self.ending() {
            let outcome = baylee_client_core::interaction::outcome(result, seat, self.my_team());
            self.cues.note_ending(outcome);
        }
        // Decided here and not per frame: a panel that re-decided
        // every frame whether to be open could never be closed.
        let was_choosing = self.browser.for_choice();
        if let Some(v) = self.view.as_ref() {
            self.browser.follow(v, self.interaction.as_ref());
        }
        self.clear_hover_for_new_browser(was_choosing);
        self.hover_the_river_label(label_before);
        // A chooser belongs to the choice it was opened under. It
        // would heal itself anyway — the options are rebuilt from the
        // current `LegalActions` — but a menu that outlives its
        // question is a menu a player has to dismiss.
        self.ability_menu = None;
        // The cast chooser goes with it, and for the same reason twice over:
        // it is a chooser, and it is a chooser about a card the game may have
        // just moved.
        self.cast_menu = None;
        // …and so is a half-pressed concession. The game moved on.
        self.concede_armed = false;
        // An armed deed survives this, and that is the point. Only a question
        // that is *not* this seat's priority window takes it — everything
        // armable is a priority-window action.
        if !matches!(
            self.interaction.as_ref().map(Interaction::pending),
            Some(Pending::Priority { player, .. }) if *player == seat
        ) {
            self.armed = None;
        }
        // A chosen way is owed to exactly one question, and this is where it
        // is written off when that question never comes: the seat is holding
        // priority again with nothing armed and no run going, so the spell is
        // on the stack and the engine had only one way to offer. Every step
        // in between fails one of the three — a run's taps come back as
        // priority *with* a run, a `ChooseColor` is not priority at all, and
        // the `ChooseCastMode` this is for is not either.
        if self.mana_run.is_none()
            && self.armed.is_none()
            && matches!(
                self.interaction.as_ref().map(Interaction::pending),
                Some(Pending::Priority { player, .. }) if *player == seat
            )
        {
            self.cast_answer = None;
        }
        rebuild_board(self);
    }

    /// The actions queued for the host but not yet sent.
    ///
    /// Read-only, and there for the tests that ask what a click *did*: the
    /// outbox is the one place a client's decision is visible before the
    /// engine has seen it, and a test that reached past it would be checking
    /// the engine rather than the client.
    #[must_use]
    pub fn outbox(&self) -> &[PlayerAction] {
        &self.outbox
    }

    /// Takes the queued actions, as `flush_outbox` does on a frame.
    ///
    /// The `pub` half of the same seam, for a test that drives more than one
    /// round trip: reading [`Self::outbox`] and never emptying it makes the
    /// second step of a run look like the first one repeated.
    ///
    /// **It is not the whole of what a frame does, and the remainder is not
    /// bookkeeping.** `flush_outbox` drops [`Self::interaction`] after
    /// sending, because the answer is on its way and the next choice replaces
    /// it — so between a send and that choice the seat has no legal actions
    /// at all, and every card in hand is neither `playable` nor offered. A
    /// harness that calls this and stops has closed that window, which is the
    /// one a player is looking at while they wait.
    pub fn take_outbox(&mut self) -> Vec<PlayerAction> {
        std::mem::take(&mut self.outbox)
    }

    /// The local seat, once the static payload has arrived.
    #[must_use]
    pub fn seat(&self) -> Option<PlayerId> {
        self.statics.as_ref().map(|s| s.your_seat)
    }

    /// The follow switch's half of a view arriving (DESIGN-v8 §1.1): a turn
    /// that changed hands shows its active seat, now or once nothing holds
    /// the table still; a seat waiting to be shown is shown when that is so.
    fn follow_the_turn(&mut self, view: &PlayerView) {
        let question_for_me = view.awaiting == Some(view.seat);
        let pointer_on_interest = self.hovered.is_some_and(|id| {
            view.object(id)
                .is_some_and(|o| Some(o.controller) == self.visiting)
        });
        let edge = baylee_client_core::tableview::FollowEdge {
            on: self.follow,
            me: view.seat,
            active_before: self.view.as_ref().map(|v| v.active),
            active_now: Some(view.active),
            question_for_me,
            pointer_on_interest,
        };
        match baylee_client_core::tableview::follow(&edge) {
            // A turn changed hands: the dwell starts again for its seat. A
            // turn the house plays in a blink is never shown; the camera
            // lands where a turn lasts (the owner, 08.10.).
            baylee_client_core::tableview::Follow::Show(seat)
            | baylee_client_core::tableview::Follow::Defer(seat) => {
                self.follow_pending = Some(seat);
                self.follow_since = self.follow_now;
            }
            baylee_client_core::tableview::Follow::Stay => {}
        }
    }

    /// Whether the seat waiting to be shown may be shown now: its turn has
    /// lasted [`FOLLOW_DWELL`], no decision of mine is open, and the pointer
    /// is not reading the board shown.
    ///
    /// A bare priority grant on an empty stack is not a decision: the house
    /// answers inside the engine, so every view of another seat's turn
    /// already awaits this seat, and a switch that waited for a view not
    /// awaiting it never moved (beta.6 QA). A target, a choice, a block or
    /// something on the stack to answer holds the table until it is
    /// answered.
    #[must_use]
    pub fn follow_due(&self) -> Option<PlayerId> {
        let seat = self.follow_pending.filter(|_| self.follow)?;
        if self.follow_now - self.follow_since < FOLLOW_DWELL {
            return None;
        }
        let deciding = self.interaction.as_ref().is_some_and(|i| {
            i.is_mine()
                && !(matches!(i.pending(), Pending::Priority { .. })
                    && self.view.as_ref().is_some_and(|v| v.stack.is_empty()))
        });
        let pointer_on_interest = self.hovered.is_some_and(|id| {
            self.view
                .as_ref()
                .and_then(|v| v.object(id))
                .is_some_and(|o| Some(o.controller) == self.visiting)
        });
        (!deciding && !pointer_on_interest).then_some(seat)
    }

    /// Shows the seat [`Self::follow_due`] names.
    pub fn follow_show(&mut self, seat: PlayerId) {
        self.follow_pending = None;
        if self.visiting != Some(seat) {
            self.visiting = Some(seat);
            self.follow_settling = true;
        }
    }

    /// The table as it stands once whatever is moving has arrived: a tear's
    /// end, else the layout. What the camera frames and the slab is cut to,
    /// so neither breathes with the tear's stages.
    #[must_use]
    pub fn settled_layout(&self) -> Option<&TableLayout> {
        self.tear.as_ref().map(Tear::to).or(self.layout.as_ref())
    }

    /// A seat's slot if its board is drawn: on the felt, or swinging out of
    /// a tear (DESIGN-v8 §0, the owner's tear).
    #[must_use]
    pub fn drawn_slot(&self, player: PlayerId) -> Option<&baylee_client_core::SeatSlot> {
        let layout = self.layout.as_ref()?;
        layout.shown(player).or_else(|| {
            self.tear
                .as_ref()
                .filter(|t| t.draws(player))
                .and_then(|_| layout.slot(player))
        })
    }

    /// The seat the **camera** visits: [`Self::visiting`] under a camera
    /// arrangement, nothing under a layout one, whose seat of interest is
    /// answered by moving cards while the camera stays home (DESIGN-v8 §0).
    #[must_use]
    pub fn camera_visit(&self) -> Option<PlayerId> {
        self.visiting.filter(|_| !self.arrangement.moves_cards())
    }

    /// Which tap the ability sheet is asking the colour of, if it is asking.
    ///
    /// [`Self::ability_tap`] and nothing else, gated on the sheet being open:
    /// the step *into* a tap belongs to the sheet it was taken on, so a value
    /// that outlives its sheet answers nothing rather than opening a bubble
    /// over the next card clicked.
    #[must_use]
    pub fn asking_tap(&self) -> Option<u32> {
        self.ability_menu.and(self.ability_tap)
    }

    /// The local seat's side, if the table has sides at all.
    ///
    /// The roster and not the view: a seat's team is in `GameStatic` and no
    /// `PlayerView` carries it. It is the one thing about the table that
    /// reading a `Victor::Team` needs — see
    /// [`baylee_client_core::interaction::outcome`] — and it is asked twice,
    /// by the end screen and by the sound the end of a game makes, which is
    /// why it is written once here.
    #[must_use]
    pub fn my_team(&self) -> Option<u8> {
        let statics = self.statics.as_ref()?;
        statics
            .seats
            .iter()
            .find(|s| s.player == statics.your_seat)
            .and_then(|s| s.team)
    }

    /// Whether the local seat is being asked something right now.
    #[must_use]
    pub fn is_my_turn_to_act(&self) -> bool {
        self.interaction.as_ref().is_some_and(Interaction::is_mine)
    }

    /// Whether this seat's own standing order is currently withholding its
    /// priority.
    ///
    /// Read off the view rather than remembered here on purpose: the engine
    /// drops a hold the moment its condition is met, and a client keeping its
    /// own copy would light an indicator for a hold that expired two
    /// resolutions ago.
    #[must_use]
    pub fn priority_held(&self) -> bool {
        self.view.as_ref().is_some_and(|v| v.priority_held)
    }

    /// What the two hold keys send, given which of them was pressed.
    ///
    /// One door for both keys and for the prompt bar's button, because the
    /// toggle rule is the part worth having in one place: a hold that is
    /// already running is **cancelled** by either key rather than replaced.
    /// A player who has stopped being asked and cannot remember which key did
    /// it should not have to guess to get the game back.
    ///
    /// `UntilStackEmpty` carries the stack depth this seat can see, and a
    /// stale view is safe by construction: if something was added since, the
    /// engine reads a depth above the one sent and cancels the hold on the
    /// spot — which is exactly right, because somebody just responded to what
    /// was being let through.
    ///
    /// `None` before the first view arrives: there is no game to hold yet.
    #[must_use]
    pub fn hold_action(&self, until_turn_ends: bool) -> Option<PlayerAction> {
        let view = self.view.as_ref()?;
        let hold = if view.priority_held {
            baylee_engine::choice::PriorityHold::Always
        } else if until_turn_ends {
            baylee_engine::choice::PriorityHold::UntilEndOfTurn { turn: view.turn }
        } else if let Some(object) = self
            .stack_selected
            .filter(|id| view.stack.iter().any(|obj| obj.id == *id))
        {
            baylee_engine::choice::PriorityHold::UntilTopOfStack { object }
        } else {
            baylee_engine::choice::PriorityHold::UntilStackEmpty {
                depth: u16::try_from(view.stack.len()).unwrap_or(u16::MAX),
            }
        };
        Some(PlayerAction::SetPriorityHold(hold))
    }

    /// Whether "let the stack resolve" is a thing this seat could ask for.
    ///
    /// Two facts, and each of them changes what the *same* press does rather
    /// than merely greying it out. On an empty stack [`hold_action(false)`]
    /// sends `UntilStackEmpty { depth: 0 }`, a hold that is over before it
    /// begins — a button promising to do nothing. And while a hold is already
    /// running it sends `Always`, which **cancels** that hold: the button
    /// under the F6 cap would do the opposite of what its label says. §4.4 of
    /// the ledge design draws that second state as its own sentence, which is
    /// why this is a predicate and not a disabled control.
    ///
    /// Read by both the drawing and the press, so a stack that emptied between
    /// the two cannot be held against.
    ///
    /// [`hold_action(false)`]: Duel::hold_action
    #[must_use]
    pub fn can_hold_for_stack(&self) -> bool {
        self.view
            .as_ref()
            .is_some_and(|v| !v.stack.is_empty() && !v.priority_held)
    }

    /// Whether the engine would take a draw offer right now.
    ///
    /// `Engine::offer_draw` refuses anything but the offerer's own priority,
    /// because the offer suspends a decision that has to be handed back
    /// untouched if anyone refuses (CR 104.4i). The button was drawn live
    /// whatever the game was doing, so the usual answer to pressing it was an
    /// `IllegalAction` in the prompt bar.
    ///
    /// Priority is the whole condition. The engine's other refusal — nobody
    /// left to offer to — cannot happen while this seat holds priority, since
    /// a game with one player in it has already ended.
    #[must_use]
    pub fn can_offer_draw(&self) -> bool {
        self.interaction.as_ref().is_some_and(|i| {
            i.is_mine() && matches!(i.pending(), baylee_engine::choice::Pending::Priority { .. })
        })
    }
}

mod duel_plugin;
pub use duel_plugin::*;

/// Opens and closes the duel on request.
fn handle_commands(
    mut commands: MessageReader<DuelCommand>,
    phase: Res<State<DuelPhase>>,
    mut next: ResMut<NextState<DuelPhase>>,
    mut duel: ResMut<Duel>,
) {
    for command in commands.read() {
        match command {
            DuelCommand::Open if *phase.get() == DuelPhase::Closed => {
                *duel = Duel::default();
                next.set(DuelPhase::Opening);
            }
            DuelCommand::Close => next.set(DuelPhase::Closed),
            DuelCommand::Open => {}
        }
    }
}

/// Drains the host and keeps the client's state current.
#[allow(clippy::too_many_arguments)]
fn poll_host(
    host: Option<ResMut<InstalledHost>>,
    mut duel: ResMut<Duel>,
    mut textures: ResMut<textures::CardTextures>,
    phase: Res<State<DuelPhase>>,
    mut next: ResMut<NextState<DuelPhase>>,
    mut reports: MessageWriter<DuelReport>,
    mut journey: Option<ResMut<arrival::Journey>>,
    time: Option<Res<Time<Real>>>,
) {
    let Some(mut host) = host else {
        return;
    };
    if *phase.get() == DuelPhase::Closed {
        return;
    }
    for message in host.0.poll() {
        match message {
            HostMessage::Static(statics) => {
                if let Some(journey) = journey.as_mut() {
                    journey.changed_scene();
                }
                duel.statics = Some(*statics);
                // The seat's own decklist, where this host knows it.
                if duel.own_deck.is_empty() {
                    duel.own_deck = host.0.own_deck();
                }
                // Every attach opens with this payload. One before the
                // curtain is up is a seat the engine may not have heard
                // from, so it is told again once the view is built.
                if !duel.curtain_up {
                    duel.ready_sent = false;
                }
                // A print table arriving is the one event that can turn an
                // unresolvable printing into a resolvable one: this payload is
                // re-sent, before the view that needs it, whenever the seat
                // earns an entry it did not have. Without this the first ask
                // decided the answer for the whole game, and a permanent whose
                // entry arrived a frame late never drew its art again.
                textures.forget_unresolved();
            }
            HostMessage::View(view, log) => {
                // The lines first, and read against their own frame's view.
                // A frame that only carries the next part of the log repeats
                // the view the client already holds, and its lines are no
                // less new for that.
                //
                // The same door hears which of them are another seat's
                // reveals, read against the same frame: a reveal is news only
                // on a live socket, never in a telling from the first line.
                if let Some(tail) = &log {
                    let duel = &mut *duel;
                    duel.reveals.take(&mut duel.log, tail, &view);
                }
                if let Some(journey) = journey.as_mut() {
                    journey.changed_scene();
                }
                duel.receive_view(*view);
                rebuild_board(&mut duel);
                if journey.is_none() && *phase.get() == DuelPhase::Opening {
                    next.set(DuelPhase::Playing);
                }
                if journey.is_none() && !duel.curtain_up && !duel.ready_sent {
                    host.0.ready();
                    duel.ready_sent = true;
                }
            }
            HostMessage::Curtain => duel.curtain_up = true,
            // Debug builds only (`docs/protocol.md` §"An AI seat's
            // reasoning"): a release engine sends none, and one that came
            // anyway is not queued, so a release client's door never stands.
            HostMessage::AiLog(ai_log) => {
                if cfg!(debug_assertions) {
                    duel.ai_log_heard = true;
                    duel.ai_said.push(ai_log);
                }
            }

            HostMessage::Preparing {
                ready,
                total,
                starts_in,
            } => {
                if let Some(journey) = journey.as_mut() {
                    journey.synchronize(
                        ready,
                        total,
                        starts_in,
                        time.as_ref().map_or(0.0, |t| t.elapsed_secs_f64()),
                    );
                }
            }
            HostMessage::Choice(pending) => {
                if matches!(*pending, Pending::GameOver(_)) {
                    next.set(DuelPhase::Finished);
                    reports.write(DuelReport::Finished);
                }
                duel.receive_choice(*pending);
            }
            HostMessage::Failed(reason) => {
                // Gated the way the prompt bar's refusal line is: a game that
                // has ended keeps none of the things that answer a question,
                // and a refusal chiming over the end screen would be the
                // client objecting to something nobody can still do.
                if duel.ending().is_none() {
                    duel.cues.note_refusal();
                }
                if *phase.get() == DuelPhase::Opening
                    && let Some(journey) = journey.as_mut()
                {
                    journey.fail(reason.clone());
                }
                duel.last_error = Some(Refusal::Verbatim(reason.clone()));
                reports.write(DuelReport::Failed(reason));
            }
        }
    }
}

/// Applies the standing orders and the autopilot: hands control back at
/// the boundary, and never makes a real decision for the player.
fn run_autopilot(mut duel: ResMut<Duel>, prefs: Res<prefs::Prefs>) {
    if !duel.outbox.is_empty() {
        return;
    }
    // Only the orders for cards this game has shown (#285), as a difference
    // from what the engine already has: a card coming into view sends its
    // own orders, and a change in the settings sends the change.
    let known = |order: &&automation::AbilityOrder| duel.known_cards.contains(&order.ability.card);
    let stale = duel.view.is_some()
        && !duel
            .ability_orders_applied
            .iter()
            .eq(prefs.all().ability_orders.iter().filter(known));
    if stale {
        let wanted: Vec<_> = prefs
            .all()
            .ability_orders
            .iter()
            .filter(known)
            .copied()
            .collect();
        let old = std::mem::take(&mut duel.ability_orders_applied);
        for order in &old {
            if !wanted.iter().any(|new| new.ability == order.ability) {
                duel.submit(automation::AbilityOrder::manual(order.ability).action());
            }
        }
        for order in &wanted {
            if !old.contains(order) {
                duel.submit(order.action());
            }
        }
        duel.ability_orders_applied = wanted;
        if !duel.outbox.is_empty() {
            return;
        }
    }
    // Each write below is asked for first: this runs every frame, and a
    // `&mut duel` through the `ResMut` marks the whole duel changed.
    if duel.stack_stop_requested {
        if !duel.yes_batch.is_idle() {
            duel.yes_batch = yes_batch::YesBatch::default();
        }
        return;
    }
    if (duel.combat_auto.is_some() || !duel.yes_batch.is_idle())
        && let Some(answer) = requested_batch_answer(&mut duel)
    {
        duel.submit(answer);
        return;
    }
    // A plan in flight owns the priority it is spending; passing under it
    // would throw the mana away between the tap and the spell.
    if duel.mana_run.is_some() {
        return;
    }
    let Some((phase, step, turn)) = duel.view.as_ref().map(|v| (v.phase, v.step, v.turn)) else {
        return;
    };
    if let Some(pilot) = duel.autopilot
        && pilot.reached(phase, turn)
    {
        duel.autopilot = None;
    }
    let answer = {
        let Some(view) = duel.view.as_ref() else {
            return;
        };
        let active_is_mine = hud::same_team(duel.statics.as_ref(), view.active, view.seat);
        let Some(interaction) = duel.interaction.as_ref() else {
            return;
        };
        automation::auto_answer(
            interaction.pending(),
            Situation {
                mine: interaction.is_mine(),
                active_is_mine,
                phase,
                step,
                // Read here rather than in `automation`, because "the other
                // side" is a question about the roster and that module knows
                // only about turns.
                opposing_stack: view
                    .stack
                    .iter()
                    .any(|o| !hud::same_team(duel.statics.as_ref(), o.controller, view.seat)),
                // What this client is offering that the engine's list does
                // not name. Without it `pass_when_nothing_to_do` reads an
                // empty `castable` over four untapped Forests as an empty
                // hand and passes the window away.
                offering: !duel.reachable.is_empty()
                    || !duel.suspend_reach.is_empty()
                    || !duel.ability_reach.is_empty(),
                owing: view.owed.is_some() && view.awaiting == Some(view.seat),
            },
            prefs.orders(),
            prefs.auto(),
            duel.autopilot.as_ref(),
        )
    };
    let action = match answer {
        automation::AutoAnswer::None => return,
        automation::AutoAnswer::Pass => PlayerAction::PassPriority,
        automation::AutoAnswer::DeclareNoAttackers => {
            PlayerAction::DeclareAttackers { attackers: vec![] }
        }
        automation::AutoAnswer::DeclareNoBlockers => {
            PlayerAction::DeclareBlockers { blockers: vec![] }
        }
    };
    duel.submit(action);
}

/// Continue only the finite batch explicitly approved on the current prompt.
fn requested_batch_answer(duel: &mut Duel) -> Option<PlayerAction> {
    let (view, interaction) = duel.view.as_ref().zip(duel.interaction.as_ref())?;
    if let Some(auto) = duel.combat_auto.as_mut() {
        if !auto.accepts(view, interaction.pending()) {
            duel.combat_auto = None;
        } else if let Some(answer) = auto.answer(view, interaction.pending()) {
            return Some(answer);
        }
    }
    duel.yes_batch.answer(view, interaction.pending())
}

/// Tapping permanents for mana, one action at a time.
///
/// The plan is decided in one go (`manaplan::plan`) and then spent one step
/// per engine round trip, because that is how the engine works: every
/// activation is an action, and each one comes back as a fresh `Pending` with
/// a fresh `LegalActions`. That round trip is also the safety property — each
/// step is re-checked against what the engine is offering *now*, so a plan
/// that has gone stale stops instead of guessing.
///
/// Usually the mana is *for* something and [`RunEnd`] says what. It is not
/// always: [`RunEnd::Float`] is a run of one tap made because the player
/// asked for that mana and nothing else, which is what a mana bubble sends.
#[derive(Debug)]
pub struct ManaRun {
    /// Taps still to make.
    steps: std::collections::VecDeque<baylee_client_core::manaplan::Step>,
    /// The colour to answer with while an ability is asking for one.
    asking: Option<baylee_core::mana::ManaColor>,
    /// The card all of this is for — the spell being paid for, or, under
    /// [`RunEnd::Float`], the permanent whose own mana was asked for.
    card: ObjectId,
    /// What the mana is spent on once every tap is made.
    then: RunEnd,
    /// Set for a cast made before its mana (CR 601.2g): the card was
    /// `LegalActions::payable`, so the cast is sent first, its own questions
    /// are answered as they come, and the plan is worked out against the
    /// payment window it opens (`PlayerView::owed`). Its mana's triggers
    /// then wait until the spell is cast (CR 601.2i). `None` for a run that
    /// taps first.
    cast_first: Option<CastFirst>,
}

/// Where a cast-first run stands.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CastFirst {
    /// The cast is still to be sent.
    Unsent,
    /// The cast was sent; the run waits for its payment window.
    Sent,
}

impl ManaRun {
    /// Starts a run for `card`.
    #[must_use]
    pub fn new(plan: baylee_client_core::manaplan::Plan, card: ObjectId, then: RunEnd) -> Self {
        Self {
            steps: plan.steps.into(),
            asking: None,
            card,
            then,
            cast_first: None,
        }
    }

    /// Starts a run that casts `card` first and pays for it in the window
    /// the cast opens (CR 601.2g), for a card in `LegalActions::payable`.
    #[must_use]
    pub fn cast_first(card: ObjectId) -> Self {
        Self {
            steps: std::collections::VecDeque::new(),
            asking: None,
            card,
            then: RunEnd::Cast,
            cast_first: Some(CastFirst::Unsent),
        }
    }

    /// The spell being paid for — the HUD says so while it happens.
    #[must_use]
    pub const fn card(&self) -> ObjectId {
        self.card
    }

    /// The spell a cast-first run has sent and is answering the questions
    /// of before it pays (CR 601.2c–g): the decision sheet says the cast and
    /// the question in one sentence, and that the mana comes after.
    #[must_use]
    pub fn casting_first(&self) -> Option<ObjectId> {
        (self.cast_first == Some(CastFirst::Sent)).then_some(self.card)
    }

    /// The same run once its cast has been sent, for a test that stands in
    /// that moment.
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn sent(mut self) -> Self {
        self.cast_first = Some(CastFirst::Sent);
        self
    }
}

/// Spends a mana plan, one action per frame the engine asks us something.
///
/// Aborting is a first-class outcome and not an error path: anything the
/// engine offers that is not the next step of the plan ends the run and hands
/// the player back their turn, with whatever was already tapped left tapped.
/// That is the honest failure — mana in the pool is a thing the player can
/// see and spend — and it is much better than the alternative of pushing an
/// action the engine will refuse.
///
/// Asked before the duel is borrowed: `&mut duel` through the `ResMut` is a
/// write, and with no run under way this would mark the duel changed on
/// every frame.
fn run_mana_plan(mut duel: ResMut<Duel>) {
    if duel.mana_run.is_some() {
        advance_mana_run(&mut duel);
    }
}

/// One step of a run, against whatever the engine is asking right now.
///
/// The system above is the wiring; this is the decision, and it is `pub` so a
/// test can drive a whole run against a real `LocalHost` the way the frame
/// loop does — press, send, take the answer, press again. A run asserted on
/// through its `ManaRun` alone would prove the plan and not the spending, and
/// spending is where every one of this function's failure modes lives.
pub fn advance_mana_run(duel: &mut Duel) {
    if duel.mana_run.is_none() {
        return;
    }
    // Between sending and the next snapshot there is nothing to decide; the
    // run is not stale, it is simply waiting.
    let Some(interaction) = duel.interaction.as_ref() else {
        return;
    };
    let seat = duel.seat().unwrap_or(PlayerId::new(0));
    let pending = interaction.pending().clone();
    if let Some(phase) = duel.mana_run.as_ref().and_then(|r| r.cast_first)
        && advance_cast_first(duel, phase, &pending, seat)
    {
        return;
    }
    let mut action = None;
    let mut finished = false;
    let mut abort = None;

    match &pending {
        Pending::ChooseColor { player, options } if *player == seat => {
            let asked = duel.mana_run.as_ref().and_then(|r| r.asking);
            match asked.filter(|c| options.contains(c)) {
                Some(color) => {
                    action = Some(PlayerAction::ChooseColor(color));
                    if let Some(run) = duel.mana_run.as_mut() {
                        run.asking = None;
                    }
                }
                None => abort = Some(Phrase::PlanColourGone),
            }
        }
        Pending::Priority { player, legal } if *player == seat => {
            let step = duel.mana_run.as_mut().and_then(|r| r.steps.pop_front());
            if let Some(step) = step {
                action = tap_action(&step, legal);
                if action.is_none() {
                    abort = Some(Phrase::PlanLandGone);
                } else if let Some(run) = duel.mana_run.as_mut() {
                    run.asking = step.color;
                }
            } else {
                // Every tap is made; the mana is floating and the engine is
                // offering the thing it was floated for. Which list to look
                // in was decided when the run was armed, not here — a card
                // that is castable *and* suspendable is two different deeds
                // and neither is undoable.
                let run = duel.mana_run.as_ref().map(|r| (r.card, r.then));
                match run {
                    Some((source, RunEnd::Ability(ability_index))) => {
                        if legal.abilities.contains(&(source, ability_index)) {
                            action = Some(PlayerAction::ActivateAbility {
                                source,
                                ability_index,
                            });
                        } else {
                            abort = Some(Phrase::DeedWithdrawn);
                        }
                    }
                    Some((card, RunEnd::Cast)) if legal.castable.contains(&card) => {
                        action = Some(PlayerAction::CastSpell { card });
                    }
                    Some((card, RunEnd::Suspend)) if legal.suspendable.contains(&card) => {
                        action = Some(PlayerAction::Suspend { card });
                    }
                    // Nothing is owed at the end of a pour, and there is
                    // nothing to check either: the mana is in the pool, which
                    // is a thing the player can see and spend.
                    Some((_, RunEnd::Float)) => {}
                    // Still a payment window this seat owes in: settle it.
                    // Anything else and the window was closed under the run,
                    // which is the game moving on, not a pass to send.
                    Some((_, RunEnd::Settle))
                        if legal.can_pass
                            && duel.view.as_ref().is_some_and(|v| {
                                v.owed.is_some() && v.awaiting == Some(v.seat)
                            }) =>
                    {
                        action = Some(PlayerAction::PassPriority);
                    }
                    Some((_, RunEnd::Settle)) => {
                        abort = Some(Phrase::PlanQuestionChanged);
                    }
                    Some((_, RunEnd::Cast)) => {
                        abort = Some(Phrase::PlanSpellRefused);
                    }
                    Some((_, RunEnd::Suspend)) => {
                        abort = Some(Phrase::PlanSuspendRefused);
                    }
                    None => abort = Some(Phrase::PlanCardGone),
                }
                finished = true;
            }
        }
        // Mana abilities do not use the stack, so priority never leaves the
        // seat in the middle of a plan. Anything else means the game moved on
        // without us and the plan is void.
        _ => abort = Some(Phrase::PlanQuestionChanged),
    }

    if let Some(reason) = abort {
        duel.last_error = Some(Refusal::Said(reason));
        duel.mana_run = None;
        return;
    }
    if finished {
        duel.mana_run = None;
    }
    if let Some(action) = action {
        duel.submit(action);
    }
}

/// The cast-first half of a run (CR 601.2g); returns whether it handled this
/// question, and `false` once the payment window is open and the run has
/// become an ordinary settle.
///
/// The cast goes out on this seat's priority. Every question after it that
/// is not the window is the cast's own (its mode, X, targets) and is the
/// player's to answer: the run waits. The window's plan is worked out the
/// way the pay button's is ([`Duel::compute_owed_plan`]), against the pool
/// as it stands, so a land tapped by hand is counted. A priority that is not
/// a window means the cast is over: on the stack, or reversed.
fn advance_cast_first(
    duel: &mut Duel,
    phase: CastFirst,
    pending: &Pending,
    seat: PlayerId,
) -> bool {
    let Some(card) = duel.mana_run.as_ref().map(|r| r.card) else {
        return true;
    };
    let Pending::Priority { player, legal } = pending else {
        // The cast's own questions, answered by the player (or the cast
        // chooser's answer, `take_the_chosen_cast_mode`).
        if phase == CastFirst::Unsent {
            duel.last_error = Some(Refusal::Said(Phrase::PlanQuestionChanged));
            duel.mana_run = None;
        }
        return true;
    };
    if *player != seat {
        return true;
    }
    if phase == CastFirst::Unsent {
        if legal.castable.contains(&card) || legal.payable.contains(&card) {
            if let Some(run) = duel.mana_run.as_mut() {
                run.cast_first = Some(CastFirst::Sent);
            }
            duel.submit(PlayerAction::CastSpell { card });
        } else {
            duel.last_error = Some(Refusal::Said(Phrase::DeedWithdrawn));
            duel.mana_run = None;
        }
        return true;
    }
    if !duel.paying() {
        let cast = duel
            .view
            .as_ref()
            .is_some_and(|v| v.stack.iter().any(|o| o.id == card));
        duel.mana_run = None;
        if !cast {
            duel.last_error = Some(Refusal::Said(Phrase::PlanSpellRefused));
        }
        return true;
    }
    let Some(plan) = duel.compute_owed_plan() else {
        // Nothing here pays it: passing reverses the cast (CR 601.2h).
        duel.mana_run = None;
        duel.submit(PlayerAction::PassPriority);
        duel.last_error = Some(Refusal::Said(Phrase::CardCostsUnavailable));
        return true;
    };
    if let Some(run) = duel.mana_run.as_mut() {
        run.steps = plan.steps.into();
        run.then = RunEnd::Settle;
        run.cast_first = None;
    }
    false
}

/// The action for one tap, or `None` when the engine is no longer offering it.
fn tap_action(
    step: &baylee_client_core::manaplan::Step,
    legal: &baylee_engine::choice::LegalActions,
) -> Option<PlayerAction> {
    match step.tap {
        baylee_client_core::manaplan::Tap::Intrinsic => legal
            .mana_abilities
            .contains(&step.source)
            .then_some(PlayerAction::ActivateManaAbility {
                source: step.source,
            }),
        baylee_client_core::manaplan::Tap::Ability(ability_index) => legal
            .abilities
            .contains(&(step.source, ability_index))
            .then_some(PlayerAction::ActivateAbility {
                source: step.source,
                ability_index,
            }),
    }
}

/// Answers the engine's `ChooseCastMode` with the way the player already
/// chose, when they chose one.
///
/// Asked before the duel is borrowed, as [`run_mana_plan`] is.
fn answer_the_chosen_cast_mode(mut duel: ResMut<Duel>) {
    if duel.cast_answer.is_some() {
        take_the_chosen_cast_mode(&mut duel);
    }
}

/// The decision behind that system, `pub` for the same reason
/// [`advance_mana_run`] is: a test drives the whole cast through the same
/// door the frame loop does.
///
/// It is the far end of [`CastMenu`]. The player answered this client's
/// question before any mana was floated; the engine asks its own once the
/// mana is up, and this is where the two are joined. The match is on
/// [`baylee_engine::choice::CastModeKind`] and never on an index, because the
/// engine's indices are positions in a list it builds against the pool at the
/// instant it asks — and the run that just finished is what changed that pool.
///
/// A way the engine does not offer is reported rather than substituted. It is
/// reachable: the pitch card this client counted can have left the hand
/// between the chooser and the cast (a Force of Will countering the very
/// spell), and casting the *other* way instead would be the silence this
/// whole repair exists to end.
pub fn take_the_chosen_cast_mode(duel: &mut Duel) {
    let Some((card, kind)) = duel.cast_answer else {
        return;
    };
    let seat = duel.seat().unwrap_or(PlayerId::new(0));
    let Some(interaction) = duel.interaction.as_ref() else {
        return;
    };
    let Pending::ChooseCastMode {
        player,
        object,
        options,
        ..
    } = interaction.pending()
    else {
        return;
    };
    if *player != seat || *object != card {
        return;
    }
    let at = options.iter().position(|option| option.kind == kind);
    duel.cast_answer = None;
    let Some(at) = at else {
        duel.last_error = Some(Refusal::Said(Phrase::CastModeWithdrawn));
        return;
    };
    // Through `choose_index` and `confirm` rather than building the action
    // here, so this answers the question the same way a press on the row
    // would — one door, and a client that cannot express an answer fails
    // rather than sending one nothing on screen could have produced.
    let action = duel
        .interaction
        .as_mut()
        .and_then(|i| i.choose_index(at).then(|| i.confirm())?);
    if let Some(action) = action {
        duel.submit(action);
    }
}

/// The taps that would make `card` castable, if any.
///
/// `None` both when the spell needs no help and when nothing here can pay for
/// it — the caller has already asked the engine the first question.
#[must_use]
pub fn mana_for(duel: &Duel, card: ObjectId) -> Option<baylee_client_core::manaplan::Plan> {
    let view = duel.view.as_ref()?;
    let legal = duel.interaction.as_ref()?.legal_actions()?;
    // A hand card, a card in the seat's own graveyard it may flash back, or a
    // commander standing in the command zone. The three are the only places
    // this client offers to tap lands *for*, and they have to be the same
    // three [`reachable`] admits — a card in one set and not the other is a
    // card that lights up and then does nothing when it is clicked. The
    // graveyard's price is the flashback cost, for the reason given there.
    let cost = if let Some(hand_card) = baylee_client_core::decision::hand(view)
        .iter()
        .find(|c| c.id == card)
    {
        if baylee_cards::by_index(hand_card.card.index)
            .is_some_and(|def| def.faces[0].kicked_targets.is_some())
        {
            return castmodes::reachable_modes(view, legal, card)
                .first()
                .map(|mode| mode.plan.clone());
        }
        manasources::hand_cost(hand_card)?
    } else if let Some(buried) = view
        .graveyards
        .get(view.seat.get() as usize)
        .and_then(|zone| zone.iter().find(|o| o.id == card))
    {
        buried.flashback?
    } else {
        let commander = view
            .seat(view.seat)?
            .commanders
            .iter()
            .find(|c| c.object == card)?;
        commander_cost(commander)?.with_more_generic(2 * commander.casts)
    };
    let cost = cost
        .with_more_generic(legal.spell_increase(card, baylee_engine::choice::CastModeKind::Normal));
    let pool = view
        .seat(baylee_client_core::decision::resource_player(view))?
        .mana_pool;
    baylee_client_core::manaplan::plan(&cost, &pool, &manasources::sources(view, legal))
}

/// The taps that would make `card` suspendable, if any.
///
/// [`mana_for`] for the suspend cost. Separate rather than a flag on it,
/// because the two costs are different numbers on the same card — Ancestral
/// Vision prints no mana cost at all and suspends for `{U}` — and a caller
/// that took the wrong one would tap the wrong lands.
#[must_use]
pub fn suspend_mana_for(duel: &Duel, card: ObjectId) -> Option<baylee_client_core::manaplan::Plan> {
    let view = duel.view.as_ref()?;
    let legal = duel.interaction.as_ref()?.legal_actions()?;
    let hand_card = baylee_client_core::decision::hand(view)
        .iter()
        .find(|c| c.id == card)?;
    let cost = suspend_cost(hand_card.card)?;
    let pool = view
        .seat(baylee_client_core::decision::resource_player(view))?
        .mana_pool;
    baylee_client_core::manaplan::plan(&cost, &pool, &manasources::sources(view, legal))
}

/// Whether there is a game on screen that a lost socket would interrupt.
///
/// Not `Finished`: a table whose game has ended closes its socket in the
/// ordinary course of things, and a client that redialled then would spend two
/// minutes trying to rejoin a game it just watched end.
fn duel_is_live(phase: Res<State<DuelPhase>>) -> bool {
    matches!(*phase.get(), DuelPhase::Opening | DuelPhase::Playing)
}

/// Dials the table again when the socket goes away.
///
/// [`NetworkHost`] could always do this — it re-dials and asks for the frames
/// the seat missed — and nothing ever called it, so a dropped connection ended
/// the game with a line in the prompt bar while the table sat there waiting.
/// What was missing is this: someone to decide *when*.
///
/// The schedule itself is [`Retry`], in `baylee-client-core`, so it can be
/// exercised without a gateway to disconnect from. Everything policy-shaped is
/// there; what is here is only the wiring to a host and a frame clock.
fn keep_the_table_connected(
    host: Option<ResMut<InstalledHost>>,
    mut duel: ResMut<Duel>,
    mut retry: ResMut<Reconnect>,
    time: Res<Time>,
    mut reports: MessageWriter<DuelReport>,
) {
    let Some(mut host) = host else {
        return;
    };
    // Read before the arms rather than inside them, because `duel` is written
    // in every one of them and this is the one thing read off it. It is also
    // the last payload that arrived and not a live one: `GameStatic` reaches a
    // client once, at join, and nothing clears it — which is what makes the
    // window readable at all, since this runs only while the socket is gone.
    let table = Window::of(duel.statics.as_ref());
    match host.0.link() {
        // `Local` is a host with no socket to lose, and the schedule must
        // never start on one: an in-process engine would otherwise be
        // "reconnected" to twelve times and then declared unreachable.
        // Every frame of a live table comes here, so the notes are cleared
        // only where they stand: an assignment through the `ResMut` marks
        // the whole duel changed.
        LinkState::Local | LinkState::Up => {
            retry.schedule.settle();
            if retry.told {
                retry.told = false;
            }
            if duel.link_note.is_some() {
                duel.link_note = None;
            }
        }
        // A dial is in flight. Saying the same thing as `Down` is deliberate:
        // the player is told the connection dropped and that something is
        // being done, and which of those two states a given frame is in is
        // not information anyone can act on. Which sentence that is comes
        // from the same place in both arms — see `link_note`.
        LinkState::Connecting => {
            retry.schedule.stayed_down(time.delta_secs());
            duel.link_note = Some(link_note(&retry.schedule, table));
        }
        // The table said this client speaks another protocol (#271). Only an
        // update on one side changes that, so there is no schedule: the
        // player is told which side is behind, once, and nothing redials.
        LinkState::Refused { table } => {
            duel.link_note = Some(refusal_note(table));
            if !retry.told {
                retry.told = true;
                reports.write(DuelReport::Unreachable);
            }
        }
        LinkState::Down => {
            retry.schedule.stayed_down(time.delta_secs());
            if retry.schedule.exhausted() {
                duel.link_note = Some(Phrase::LinkGaveUp);
                if !retry.told {
                    retry.told = true;
                    reports.write(DuelReport::Unreachable);
                }
            } else {
                duel.link_note = Some(link_note(&retry.schedule, table));
                if retry.schedule.tick(time.delta_secs()) {
                    // A dial that could not even be started is not a reason
                    // to stop: the schedule has counted the attempt, and the
                    // next one comes round on its own. Only running out of
                    // attempts ends this.
                    drop(host.0.reconnect());
                }
            }
        }
    }
}

/// What the bar says about a table that refused this client's protocol,
/// which speaks `table` (#271): which side is behind.
///
/// Equal numbers are refused only from a client that did not say which
/// protocol it speaks, which this one always does; they fall in with the
/// newer table, the one case the player can still fix from here.
fn refusal_note(table: u32) -> Phrase {
    if table < baylee_protocol::PROTOCOL_VERSION {
        Phrase::TableOlder
    } else {
        Phrase::ClientOutdated
    }
}

/// Which of the two connection sentences the bar carries.
///
/// One function rather than a phrase written into each arm, because
/// `Connecting` and `Down` alternate for as long as a drop lasts: a client
/// that read the schedule only where it dials would fall back to the short
/// sentence for the length of every dial — once every fifteen seconds, once
/// the back-off is at its cap — and the bar would alternate between two
/// accounts of one outage. That is not a hypothetical; the arm above said
/// `LinkLost` outright.
///
/// `table` is what this client was told about the reconnect window, and the
/// second sentence is only reachable when it names one: `LinkStandIn`
/// promises the house will answer for the seat, and at a table that hands no
/// chair over — or one this client has not been told about — that is not
/// early, it is a fabrication.
fn link_note(schedule: &Retry, table: Window) -> Phrase {
    if schedule.brief(table) {
        Phrase::LinkLost
    } else {
        Phrase::LinkStandIn
    }
}

/// Sends everything the player has queued.
fn flush_outbox(host: Option<ResMut<InstalledHost>>, mut duel: ResMut<Duel>) {
    let Some(mut host) = host else {
        return;
    };
    // Held, not dropped, until the table is open (#256): the engine drops
    // what a seat sends before then.
    if duel.outbox.is_empty() || !duel.curtain_up {
        return;
    }
    for action in std::mem::take(&mut duel.outbox) {
        host.0.submit(action);
    }
    // The answer has been sent; the next choice replaces this one.
    duel.interaction = None;
}

/// What the answer being built proposes for each permanent it names: the
/// permanents the mana plan on offer would tap, the pairs of a combat
/// declaration, and the objects a choice has picked, the last two winning
/// over a tap because they are the answer itself.
#[must_use]
pub fn proposals(
    duel: &Duel,
) -> std::collections::HashMap<ObjectId, baylee_client_core::board::Proposal> {
    use baylee_client_core::board::Proposal;
    let plan = match duel.proposing() {
        Proposing::Armed(Armed {
            deed: Deed::Run { plan, .. },
            ..
        })
        | Proposing::Owed(plan) => Some(plan),
        Proposing::Armed(_) | Proposing::Nothing => None,
    };
    let spent = plan
        .into_iter()
        .flat_map(|plan| plan.steps.iter().map(|step| (step.source, Proposal::Spent)));
    let answer = duel.interaction.as_ref().into_iter().flat_map(|i| {
        i.assignments()
            .into_iter()
            .map(|(id, focus)| (id, Proposal::Combat(focus)))
            .chain(i.selected().map(|id| (id, Proposal::Picked)))
    });
    spent.chain(answer).collect()
}

impl Duel {
    /// Whether the hand's drawer stands open: the player's choice, or a
    /// question answered from the hand (`client-core::handdrawer`). Off a
    /// phone the drawer is not drawn, and this is read only there.
    #[must_use]
    pub fn hand_drawer_open(&self) -> bool {
        self.hand_drawer.open(
            self.view.as_ref(),
            self.interaction.as_ref().map(Interaction::pending),
        )
    }

    /// The snapshot the question in hand was asked at: what a fold of its
    /// sheet is kept against, so the next question opens unfolded.
    #[must_use]
    pub fn decision_seq(&self) -> Option<u64> {
        self.view.as_ref().map(|v| v.seq)
    }

    /// Folds the question's sheet to its pill, or opens it again. It answers
    /// nothing: the question stands, folded or not.
    pub fn fold_decision(&mut self) {
        let seq = self.decision_seq();
        self.decision_fold.toggle(seq);
    }

    /// The tab tapped, or `I`: open becomes shut and shut open.
    pub fn toggle_hand_drawer(&mut self) {
        let view = self.view.as_ref();
        let pending = self.interaction.as_ref().map(Interaction::pending);
        self.hand_drawer.toggle(view, pending);
    }

    /// How many cards in my hand are castable now (the tab says so: a
    /// priority with something castable does not open the drawer by
    /// itself).
    #[must_use]
    pub fn castable_in_hand(&self) -> usize {
        let (Some(view), Some(legal)) = (
            self.view.as_ref(),
            self.interaction
                .as_ref()
                .and_then(Interaction::legal_actions),
        ) else {
            return 0;
        };
        view.hand
            .iter()
            .filter(|card| legal.castable.contains(&card.id) || legal.lands.contains(&card.id))
            .count()
    }
}

/// A tear under way ([`Duel::tear`]).
#[derive(Clone, Debug)]
pub struct Tear {
    /// The pieces and the two tables (`layout::transition::Tear`).
    pub plan: baylee_client_core::layout::transition::Tear,
    /// Seconds into it.
    pub t: f32,
    /// The seed of this tear's jagged line (`felt.wgsl`'s `tear_line`): each
    /// tear tears differently, and both pieces share it.
    pub seed: f32,
}

impl Tear {
    /// The stage it is in.
    #[must_use]
    pub fn phase(&self) -> baylee_client_core::layout::transition::Phase {
        baylee_client_core::layout::transition::Phase::at(self.t)
    }

    /// The table it ends on.
    #[must_use]
    pub fn to(&self) -> &TableLayout {
        &self.plan.to
    }

    /// Whether `player`'s board is drawn although the table parks it: it is
    /// sinking away on the leaving piece, which is drawn until it is under
    /// the void.
    #[must_use]
    pub fn draws(&self, player: PlayerId) -> bool {
        use baylee_client_core::layout::transition::Piece;
        self.plan.leaving.contains(&player) && self.plan.pose(Piece::Leaving, self.t).shown
    }

    /// How high `player`'s board rides over the table now: its piece's lift.
    #[must_use]
    pub fn lift(&self, player: PlayerId) -> f32 {
        self.plan.lift(player, self.t)
    }
}

/// Rebuilds the render model from the current view.
///
/// `pub` because the two indigo sets it computes — [`Duel::reachable`] and
/// [`Duel::suspend_reach`] — are what the click path reads, so an end-to-end
/// test has to build them the way a frame does rather than assign them by
/// hand.
pub fn rebuild_board(duel: &mut Duel) {
    duel.reachable = reachable(duel);
    duel.suspend_reach = suspend_reach(duel);
    duel.activatable = activatable(duel);
    duel.ability_reach = ability_reach(duel);
    duel.proposed = proposals(duel);

    let Some(view) = duel.view.as_ref() else {
        return;
    };
    // Three lists, one light. `lands` and `castable` are what the engine will
    // accept this instant, and `suspendable` is the same kind of claim about
    // the other way a card leaves a hand — suspend's first ability is an
    // activated one the engine offers or does not (CR 702.62a), so it is gold
    // beside them rather than a fourth colour.
    let playable: std::collections::HashSet<ObjectId> = duel
        .interaction
        .as_ref()
        .and_then(Interaction::legal_actions)
        .map(|legal| {
            legal
                .lands
                .iter()
                .chain(legal.castable.iter())
                .chain(legal.suspendable.iter())
                .copied()
                .collect()
        })
        .unwrap_or_default();

    // Indigo is this client's own offer, and it has two halves for the same
    // reason gold does. They stay apart on `Duel` because a click has to know
    // which `PlayerAction` a mana run ends in; the union is drawing only.
    let reach: std::collections::HashSet<ObjectId> = duel
        .reachable
        .iter()
        .chain(duel.suspend_reach.iter())
        .copied()
        .collect();

    // Allies sit on one side of the table, so the roster is part of the
    // geometry: a partner's board is read as often as one's own, and across
    // the table it is upside down. The team travels in `GameStatic`, which a
    // seat is sent once — before any view — so a table that has a roster has
    // it by the time it is first laid out.
    let team_of = |player: PlayerId| {
        duel.statics
            .as_ref()
            .and_then(|statics| statics.seats.iter().find(|seat| seat.player == player))
            .and_then(|seat| seat.team)
    };
    let seats: Vec<Seat> = std::iter::once(view.seat)
        .chain(view.opponents_in_turn_order())
        .map(|player| Seat::on(player, team_of(player)))
        .collect();
    let mut layout = TableLayout::arranged_in(
        &seats,
        duel.canvas_aspect.unwrap_or(16.0 / 9.0),
        duel.arrangement,
        duel.visiting,
        duel.canvas_frame
            .unwrap_or(baylee_client_core::tableview::TableFrame::Wide),
    );
    duel.interest_laid = duel.visiting;
    for slot in &mut layout.slots {
        if view
            .seats
            .iter()
            .any(|seat| seat.player == slot.player && seat.commanders.is_empty())
        {
            slot.reclaim_command_strip();
        }
    }

    // A tear under way keeps its stages: the table rebuilt is the one it
    // ends on, and what the cards glide to now is the stage it is in.
    if let Some(tear) = duel.tear.as_mut() {
        tear.plan = baylee_client_core::layout::transition::Tear::new(
            tear.plan.from.clone(),
            layout.clone(),
        );
        layout = tear.plan.staged(tear.t);
    }

    duel.board = Some(BoardModel::from_view(
        view,
        baylee_client_core::board::Openings {
            playable: &playable,
            reachable: &reach,
            activatable: &duel.activatable,
            proposed: &duel.proposed,
        },
        // Who is answering for each chair. From the roster and not from the
        // view, because that is the payload that knows: a chair the house is
        // holding keeps its player's name, life and hand, and only the roster
        // says nobody is behind them. An empty slice until `GameStatic`
        // arrives, which is the honest answer for a table nobody has been
        // introduced at yet.
        duel.statics
            .as_ref()
            .map_or(&[][..], |statics| &statics.seats),
        // What card a projected name belongs to, so a permanent that has
        // become a copy is drawn as the card it copies rather than as the
        // cardboard underneath it.
        crate::cardart::registry(),
    ));
    if let Some(board) = &mut duel.board {
        duel.hand_groups = duel.hand_order.apply(&mut board.hand, view);
    }
    duel.layout = Some(layout);
}

mod reach;
#[allow(clippy::wildcard_imports)] // what `rebuild_board` and the reach tests read
use reach::*;

#[cfg(test)]
mod reconnect_tests;

#[cfg(test)]
mod rest_tests;

#[cfg(test)]
mod commander_reach_tests;

#[cfg(test)]
pub(crate) mod flashback_reach_tests;

#[cfg(test)]
mod owed_tests;

#[cfg(test)]
mod activation_payment_tests;
#[cfg(test)]
mod reachable_tests;

#[cfg(test)]
mod schedule_order_tests;

#[cfg(test)]
mod cue_feed_tests;

#[cfg(test)]
mod curtain_tests;

#[cfg(test)]
mod log_feed_tests;

/// Leaving a table despawns what it drew once each (#321).
#[cfg(test)]
mod teardown_tests;

/// A [`baylee_view::PublicObject`] carrying the registry card of that name.
///
/// [`baylee_client_core::test_support::printed`] takes the index as a plain
/// number, and that is only ever right by luck: an index is assigned over the
/// whole card corpus, of which this pool holds 1365 rows, so the literal that
/// was Command Tower is now a card nobody implemented — and a test that reads
/// what the card *says* finds nothing at all. A test that needs only an
/// identity keeps the numbered helper; this is for the ones that ask the
/// registry a question.
#[cfg(test)]
pub(crate) fn registry_printed(slot: u32, controller: u8, name: &str) -> baylee_view::PublicObject {
    let index = baylee_cards::decks::by_name(name).expect("a card of that name in the pool");
    let mut object = baylee_client_core::test_support::printed(slot, controller, name, 0);
    if let Some(card) = object.card.as_mut() {
        card.index = index;
    }
    object.rules = object.card.map(baylee_view::RulesFace::from);
    object
}

#[cfg(test)]
mod stack_tests;

/// Whether a question asks this seat to declare attackers or blockers.
fn my_combat_question(interaction: &Interaction) -> bool {
    interaction.is_mine()
        && matches!(
            interaction.pending(),
            Pending::ChooseAttackers { .. } | Pending::ChooseBlockers { .. }
        )
}

/// The attacker a Raging River label is asked about, if that is the question.
fn river_label(interaction: &Interaction) -> Option<ObjectId> {
    match interaction.pending() {
        Pending::ChoosePile { label, .. } => *label,
        _ => None,
    }
}
