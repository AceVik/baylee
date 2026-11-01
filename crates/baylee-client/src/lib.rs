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
/// `baylee:` links from the website: how they reach the client.
pub mod links;
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
pub mod replay;
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
pub mod social;
pub mod softkeys;
pub mod sound;
pub mod standalone;
pub mod table;
pub(crate) mod tablellm;
pub(crate) mod tableseats;
pub mod targeting;
#[cfg(test)]
pub(crate) mod test_pointer;
pub mod textures;
pub mod tokenart;
pub mod touch;
pub mod tour;
pub(crate) mod transport;
pub(crate) mod ui_button;
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
use baylee_client_core::reconnect::{LinkNote, Retry, Window, gave_up};
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
    /// An open payment window: the taps staged in it, and the taps "Pay
    /// now" would add to them.
    Owed {
        /// What "Pay now" would stage, when anything plans the rest.
        plan: Option<&'a baylee_client_core::manaplan::Plan>,
        /// What is staged, in staging order.
        staged: &'a [baylee_client_core::manaplan::Step],
    },
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
    ///
    /// # Panics
    /// If more than `u32::MAX` casting options were constructed.
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
                    index: u32::try_from(i).expect("option index fits the protocol"),
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
    /// "Keep all remaining" on a copy's new-target questions (report 10).
    pub(crate) keep_targets: Option<baylee_client_core::keep_targets::KeepTargets>,
    /// Page of the current legal target list.
    pub target_page: usize,
    /// Whether the attack chooser's rows are folded away (report 25): with
    /// many defenders and creatures they stand over the player's own board.
    /// The player's, for the whole game: the fold row toggles it.
    pub attack_list_folded: bool,
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
    /// Mana colours contributed by the visible battlefield, derived at the
    /// snapshot boundary in `receive_view`. The firewheel animates towards
    /// these values without rereading card abilities every rendered frame.
    pub(crate) table_mana: [f32; 5],
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
    /// Every other player whose connection is lost, counted between views:
    /// what the banner over the table draws (`PlayerView::lost`).
    pub lost_seats: baylee_client_core::lostbanner::LostSeats,
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
    /// The game menu's Settings was pressed: the lobby stands its settings
    /// screen over the table on the next frame (`lobby::table_settings`),
    /// for the reason [`Self::report_asked`] is a flag.
    pub settings_asked: bool,
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
    /// Whether *Leave the table* is waiting for its second press, disarmed by
    /// everything that disarms [`Self::concede_armed`].
    pub leave_armed: bool,
    /// The player left a running table from the game menu (report 33): the
    /// lobby tells the gateway on the way back (`POST /lobby/depart`), so
    /// the house takes the chair at once rather than after the reconnect
    /// window.
    pub departing: bool,
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
    /// A door to the log's panel pressed this frame, for the chat (the
    /// beta.8 chat window design §4): with a social session the panel is the
    /// chat window and the log one of its tabs, so the chat decides what a
    /// door does. `crate::social::window` takes it.
    pub chat_door: Option<ChatDoor>,
    /// Whether the panel is the chat window this frame (a social session):
    /// `Esc` then never closes it (§2.3).
    pub chat_panel: bool,
    /// Whether the AI log's panel is open.
    pub ai_log_open: bool,
    /// Whether an AI seat's mind has said anything this game: the AI log's
    /// door stands in the tray only from then on. Never in a release build,
    /// which has no AI log (`docs/protocol.md` §"An AI seat's reasoning").
    pub ai_log_heard: bool,
    /// Whether this client only watches the table (`docs/protocol.md`
    /// §"Spectators"), as its host says.
    pub watching: bool,
    /// How many watch the table, as the engine last said.
    pub spectators: u32,
    /// An admin ended the game, and why (`docs/protocol.md` §"Acting on a
    /// game"): the end screen says so instead of who won.
    pub closed: Option<String>,
    /// A spectator asked to stop watching, or a player to leave the table;
    /// `hud::spectators::leave_watching` closes it.
    pub leave_asked: bool,
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
    /// The taps staged in this seat's open payment window, in the order
    /// they will be sent (`client-core::paystage`). Nothing in it has
    /// reached the engine: Go sends it whole as one
    /// `PlayerAction::PayWindow`, and until then every tap can be taken
    /// back. Kept across a refused payment — the engine hands the same
    /// window back, and the player corrects the staging rather than
    /// building it again.
    pub pay_staged: baylee_client_core::paystage::Staging,
    /// The window [`Self::pay_staged`] belongs to, by what it owes: a new
    /// window starts with nothing staged.
    pay_window: Option<baylee_core::mana::ManaCost>,
    /// What this seat said yes to just before its window opened — a pact's
    /// debt, a tax — so "Don't pay" can say what declining costs. The window
    /// itself is a bare priority and carries none of it.
    pay_reason: Option<baylee_engine::choice::YesNoPrompt>,
    /// What this seat could tap the last time it held priority, kept past
    /// it for a cast's X question: that question carries no mana abilities
    /// of its own (only priority does), and the mana it is paid with is made
    /// after it is answered (CR 601.2b, 601.2g). See [`Self::cap_x`].
    priority_legal: Option<baylee_engine::choice::LegalActions>,
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
    pub link_note: Option<LinkNote>,
    /// Where the replay this table plays stands (`docs/client.md`
    /// §"Replays"), as its host last said; `None` at a live table. Written
    /// by `poll_host` only when it differs: the transport bar reads it.
    pub replay: Option<replay::ReplayStatus>,
}

mod duel;

mod duel_plugin;

/// Which door to the log's panel was pressed ([`Duel::chat_door`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChatDoor {
    /// The log's scroll, its cross, or `L`.
    Log,
    /// The chat's bubble.
    Chat,
}
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

/// An admin ended the game (`docs/protocol.md` §"Acting on a game"): nobody
/// won or lost, and the end screen says why ([`Duel::closed`]). The game is
/// over as any other is, so its question becomes `GameOver` with no winner.
/// Whether it ended now; a game already over stays as it ended.
fn close_by_admin(duel: &mut Duel, reason: String) -> bool {
    if duel.ending().is_some() {
        return false;
    }
    duel.closed = Some(reason);
    duel.receive_choice(Pending::GameOver(baylee_engine::win::GameResult {
        winner: None,
        reason: baylee_engine::win::EndReason::Draw,
    }));
    true
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
    mut placing: Option<ResMut<table::Placing>>,
) {
    let Some(mut host) = host else {
        return;
    };
    if *phase.get() == DuelPhase::Closed {
        return;
    }
    let host = &mut host.bypass_change_detection().0;
    let watching = host.watching();
    if duel.watching != watching {
        duel.watching = watching;
    }
    let mut jumped = false;
    for message in host.poll() {
        match message {
            HostMessage::Jump => jumped = jump(&mut duel, &phase, &mut next, &mut placing),
            HostMessage::Spectators(count) => {
                if duel.spectators != count {
                    duel.spectators = count;
                }
            }
            HostMessage::Static(statics) => {
                if let Some(journey) = journey.as_mut() {
                    journey.changed_scene();
                }
                duel.statics = Some(*statics);
                // The seat's own decklist, where this host knows it.
                if duel.own_deck.is_empty() {
                    duel.own_deck = host.own_deck();
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
                    host.ready();
                    duel.ready_sent = true;
                }
            }
            HostMessage::Curtain => duel.curtain_up = true,
            // An admin ended the game: nobody won or lost, and the end
            // screen says why. The game is over as any other is, so the
            // way out is the end screen's.
            HostMessage::Closed(reason) => {
                if close_by_admin(&mut duel, reason) {
                    next.set(DuelPhase::Finished);
                    reports.write(DuelReport::Finished);
                }
            }
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
    follow_the_replay(&mut duel, host.replay(), jumped);
}

/// A replay moved other than one step on (`HostMessage::Jump`): the next
/// frames are another moment, told from the log's first line
/// (`docs/client.md` §"Replays"). The duel forgets what the frames told it,
/// a finished table plays again, and the next board is placed, not glided.
fn jump(
    duel: &mut Duel,
    phase: &State<DuelPhase>,
    next: &mut NextState<DuelPhase>,
    placing: &mut Option<ResMut<table::Placing>>,
) -> bool {
    duel.rewind();
    if *phase.get() == DuelPhase::Finished {
        next.set(DuelPhase::Playing);
    }
    if let Some(placing) = placing.as_mut() {
        placing.arm();
    }
    true
}

/// A replay's place, for the transport bar, written only when it moved; and
/// no cue across a jump, or at 4× and 8×, where they would run into each
/// other (§7.3).
fn follow_the_replay(duel: &mut ResMut<Duel>, status: Option<&replay::ReplayStatus>, jumped: bool) {
    if duel.replay.as_ref() != status {
        duel.replay = status.cloned();
    }
    let quiet = jumped || status.is_some_and(|s| !s.speed.cues());
    if quiet && !duel.cues.pending().is_empty() {
        duel.cues.take();
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
    if (duel.combat_auto.is_some() || duel.keep_targets.is_some() || !duel.yes_batch.is_idle())
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
        let active_is_mine = view.active == view.seat;
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
    if let Some(keep) = duel.keep_targets.as_mut() {
        if !keep.accepts(view, interaction.pending()) {
            duel.keep_targets = None;
        } else if let Some(answer) = keep.answer(view, interaction.pending()) {
            return Some(answer);
        }
    }
    duel.yes_batch.answer(view, interaction.pending())
}

mod mana_run;
pub use mana_run::{
    ManaRun, advance_mana_run, mana_for, suspend_mana_for, take_the_chosen_cast_mode,
};
use mana_run::{answer_the_chosen_cast_mode, run_mana_plan, spell_cost};

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
    // Whether the engine waits for this seat (another player is here) or
    // pauses the game: read off the last roster and view, like the window.
    let here = baylee_client_core::reconnect::others_here(
        duel.statics.as_ref(),
        duel.view.as_ref(),
        host.0.seat(),
    );
    match host.0.link() {
        // `Local` is a host with no socket to lose, and the schedule must
        // never start on one: an in-process engine would otherwise be
        // "reconnected" to twelve times and then declared unreachable.
        // Every frame of a live table comes here, so the notes are cleared
        // only where they stand: an assignment through the `ResMut` marks
        // the whole duel changed.
        LinkState::Local | LinkState::Up => {
            if !retry.schedule.is_settled() {
                retry.schedule.settle();
            }
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
            retry.schedule.hold_for(table);
            retry.schedule.refusals(host.0.ticket_refusals());
            retry.schedule.stayed_down(time.delta_secs());
            duel.link_note = Some(link_note(&retry.schedule, table, here));
        }
        // The table said this client speaks another protocol (#271). Only an
        // update on one side changes that, so there is no schedule: the
        // player is told which side is behind, once, and nothing redials.
        LinkState::Refused { table } => {
            duel.link_note = Some(LinkNote::just(refusal_note(table)));
            if !retry.told {
                retry.told = true;
                reports.write(DuelReport::Unreachable);
            }
        }
        LinkState::Down => {
            // The table's window holds the dials on until it has passed; the
            // refusals say, when they run out, whether it was the sign-in.
            retry.schedule.hold_for(table);
            retry.schedule.refusals(host.0.ticket_refusals());
            retry.schedule.stayed_down(time.delta_secs());
            if retry.schedule.exhausted() {
                duel.link_note = Some(gave_up(&retry.schedule));
                if !retry.told {
                    retry.told = true;
                    reports.write(DuelReport::Unreachable);
                }
            } else {
                duel.link_note = Some(link_note(&retry.schedule, table, here));
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
///
/// `here` is whether another player was at the table: without one the
/// engine pauses the game rather than handing the chair over, and the bar
/// says so (`Phrase::LinkPaused`). The rule is
/// [`baylee_client_core::reconnect::link_note`]'s.
fn link_note(schedule: &Retry, table: Window, here: Option<bool>) -> LinkNote {
    baylee_client_core::reconnect::link_note(schedule, table, here)
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
    let mut out = std::collections::HashMap::new();
    proposals_into(duel, &mut out);
    out
}

/// Writes [`proposals`] into `out`, reusing its allocation.
pub(crate) fn proposals_into(
    duel: &Duel,
    out: &mut std::collections::HashMap<ObjectId, baylee_client_core::board::Proposal>,
) {
    use baylee_client_core::board::Proposal;
    out.clear();
    let (plan, staged): (_, &[baylee_client_core::manaplan::Step]) = match duel.proposing() {
        Proposing::Armed(Armed {
            deed: Deed::Run { plan, .. },
            ..
        }) => (Some(plan), &[]),
        Proposing::Owed { plan, staged } => (plan, staged),
        Proposing::Armed(_) | Proposing::Nothing => (None, &[]),
    };
    let spent = plan
        .into_iter()
        .flat_map(|plan| plan.steps.iter().map(|step| (step.source, Proposal::Spent)))
        .chain(staged.iter().map(|step| (step.source, Proposal::Staged)));
    let answer = duel.interaction.as_ref().into_iter().flat_map(|i| {
        i.assignments()
            .into_iter()
            .map(|(id, focus)| (id, Proposal::Combat(focus)))
            .chain(i.selected().map(|id| (id, Proposal::Picked)))
    });
    out.extend(spent.chain(answer));
}

/// A tear under way ([`Duel::tear`]).
#[derive(Clone, Debug)]
pub struct Tear {
    /// The pieces and the two tables (`layout::transition::Tear`).
    pub plan: baylee_client_core::layout::transition::Tear,
    /// Seconds into it.
    pub t: f32,
    /// The seed of this tear's jagged line (`felt.wesl`'s `tear_line`): each
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
        board.take_modes(&duel.log);
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
