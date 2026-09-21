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
pub mod arrowmat;
pub mod buildui;
mod card_loading;
pub mod cardart;
pub mod cardmat;
pub mod cardtext;
pub mod castmodes;
pub mod choices;
mod combatfx;
pub mod combatlines;
pub mod compass;
pub mod depart;
/// The dev-control harness. Native dev builds only; see the module docs for
/// why it is a compile-time feature rather than a runtime switch.
#[cfg(all(feature = "dev-control", not(target_arch = "wasm32")))]
pub mod devctl;
pub mod face;
pub mod feltmat;
pub mod filterui;
pub mod flip;
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
pub mod matmat;
pub mod net;
pub mod prefs;
pub mod settings;
pub mod settingsui;
pub mod sheen;
pub mod sky;
pub mod softkeys;
pub mod sound;
pub mod standalone;
pub mod table;
pub mod targeting;
pub mod textures;
pub mod tokenart;
pub mod touch;

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
    /// A duel is being set up: the static payload has arrived, the first view
    /// has not.
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
/// The run exists because the engine offers a spell only when its mana is
/// already floating, and there are exactly two things in this client that a
/// seat pays mana for out of its hand. They are told apart here rather than
/// guessed at the end, because the engine's answer at that moment is a
/// `LegalActions` in which both lists are populated and a run that picked the
/// wrong one would cast a card the player meant to suspend — which is not an
/// action anything can take back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunEnd {
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

/// The client's own state for one duel.
#[derive(Resource, Default)]
// This is the client's bag of screen state, and each bool is a different
// thing standing open or not: a menu, a preview, a run of taps. They are not
// a mode — most pairs of them occur together — so an enum would have to
// enumerate the product and would be read back through the same number of
// comparisons.
#[allow(clippy::struct_excessive_bools)]
pub struct Duel {
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
    /// The opponent whose board is being inspected.
    pub focus: Option<PlayerId>,
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
    /// The engaged autopilot, if any ("next phase" / "end turn").
    pub autopilot: Option<AutoPilot>,
    /// Stack entry chosen as the next manual response boundary.
    pub stack_selected: Option<ObjectId>,
    /// A requested boundary must also suppress the client's phase autopilot.
    pub stack_stop_requested: bool,
    /// Ability policies last installed in this host connection.
    pub ability_orders_applied: Option<Vec<automation::AbilityOrder>>,
    /// Whether the player has aimed the camera themselves.
    ///
    /// While this is false the table frames itself ([`table::frame_table`]),
    /// so every resize and every seat joining is re-framed. It is set by the
    /// gestures that can only mean the camera — a wheel over the felt, a
    /// right- or middle-drag, a pinch, the arrows — and by
    /// [`input::navigate_to_player`]; it is cleared by
    /// [`input::navigate_home`] and by the table changing size.
    ///
    /// A flag and not a comparison, because it *was* a comparison: the
    /// framing stopped following the moment the rig differed from the
    /// computed one by anything at all, which the since-deleted left-drag
    /// orbit did on every click that travelled a pixel.
    pub camera_held: bool,
    /// Hand bar scroll offset in pixels.
    pub hand_scroll: f32,
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
    /// Permanents the engine listed at least one activatable ability for.
    ///
    /// The engine's own answer, unlike [`Self::reachable`] — `LegalActions`
    /// names every source whose ability may be activated right now, mana
    /// abilities included. Kept here so the table can draw it and the board
    /// model does not have to recompute it per frame.
    pub activatable: std::collections::HashSet<ObjectId>,
    /// The permanent whose abilities the prompt bar is offering.
    ///
    /// Only ever set for one with more than one thing to do: a single
    /// ability activates on the click that found it, because a menu of one is
    /// a menu that only ever wastes a tap.
    pub ability_menu: Option<ObjectId>,
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
    /// Strikes waiting for their visual presentation, read at snapshot edges.
    pub strikes: Vec<baylee_client_core::strike::Strike>,
    /// What has been typed into the creature-type filter.
    ///
    /// It lives here and not on the `Interaction` because the interaction is
    /// rebuilt from scratch on every `HostMessage::Choice`, and a re-sent
    /// snapshot (a print table earned, a seat reattaching) would empty the
    /// box under the player's fingers. It is cleared when an action is sent
    /// and when a choice arrives that is not asking for a type.
    pub subtype_filter: String,
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
        // The third reading on the same edge, and the one that is a *reset*
        // rather than a difference: the clock is restarted from what this
        // view says, and whether the sounds are re-armed is its own question
        // — see `DecisionClock::RESTART`, which tells a new question from a
        // correction by size, because the view carries no question identity.
        self.clock
            .sync(view.decision_remaining_ms, view.awaiting == Some(view.seat));
        self.view = Some(view);
        if let Some(v) = self.view.as_ref() {
            self.browser.saw_reveal(v);
        }
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
        let cost = view.owed?;
        let legal = self.interaction.as_ref()?.legal_actions()?;
        let pool = view.seat(view.seat)?.mana_pool;
        baylee_client_core::manaplan::plan(&cost, &pool, &manasources::sources(view, legal))
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

    pub(crate) fn receive_choice(&mut self, pending: Pending) {
        let seat = self.seat().unwrap_or(PlayerId::new(0));
        if !matches!(pending, Pending::ChooseSubtype { .. }) {
            self.subtype_filter.clear();
        }
        self.interaction = Some(Interaction::new(pending, seat));
        self.refresh_owed_plan();
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
        if let Some(v) = self.view.as_ref() {
            self.browser.follow(v, self.interaction.as_ref());
        }
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

/// How the duel is configured when it opens.
#[derive(Resource, Clone, Debug)]
pub struct DuelConfig {
    /// Texture budget in bytes.
    pub texture_budget: usize,
    /// Whether to draw the debug overlay.
    pub debug_overlay: bool,
}

impl Default for DuelConfig {
    fn default() -> Self {
        Self {
            texture_budget: textures::default_budget_bytes(),
            debug_overlay: false,
        }
    }
}

/// System sets, so an embedding application can order its own work around the
/// duel's without depending on individual system names.
#[derive(SystemSet, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum DuelSet {
    /// Draining the host and rebuilding the board model.
    Sync,
    /// Turning input into actions.
    Input,
    /// Updating the scene and the overlay.
    Present,
}

/// The duel client, as a plugin.
#[derive(Default)]
pub struct DuelPlugin {
    /// Configuration applied on insert.
    pub config: DuelConfig,
}

/// Everything the duel *draws*: the scene, the sky, the overlay, and the
/// three animations that live above the retained tree.
///
/// A function of its own rather than another link in `build`'s chain,
/// because it is the longest of the four sets and it grows every time the
/// client learns to animate something. The inner tuple is the overlay's own
/// animations — a system tuple holds twenty and the outer list has already
/// outgrown one. `ease_the_stack_in` runs after the rebuild deliberately: a
/// stack row spawned this frame is spawned at rest, so without the ordering
/// it is drawn once at full strength before its arrival is ever applied.
fn add_present_systems(app: &mut App) {
    app.init_resource::<hud::StackFold>()
        .init_resource::<hud::TrayReveal>()
        .init_resource::<hud::MenuRevision>()
        .init_resource::<input::TrayGlide>();
    app.add_systems(
        Update,
        (
            table::track_canvas,
            // Ahead of both things that draw a card, so a card arriving is
            // placed with its sheen already decided rather than a frame late.
            sheen::watch_for_arrivals
                .before(table::sync_scene)
                .before(hud::sync_overlay),
            table::sync_scene,
            (table::sync_zones, table::sync_library_fan).chain(),
            (table::sync_table, compass::rotate).chain(),
            sky::hang_sky,
            sky::sync_sky,
            sky::light_the_table.after(sky::sync_sky),
            // One entry and not three: a system tuple holds twenty and this
            // list is at its limit. Chained rather than merely ordered
            // because that is what the first pair is — a card that left this
            // frame is moved once before it is counted against its own clock,
            // so a table running at ten frames a second still shows the exit
            // instead of skipping it. The third reads where the glide left
            // each card and puts a flying one's shadow back on the felt, so
            // it comes after both.
            (
                combatfx::animate.before(table::sync_scene),
                combatfx::age.before(table::sync_scene),
                table::glide.after(table::sync_scene),
                table::retire,
                table::ground_the_shadows,
            )
                .chain(),
            // After the glide, and deliberately: a line is welded to where
            // its two cards *are* this frame, so it has to be computed once
            // they have moved.
            combatlines::sync_combat_lines.after(table::glide),
            combatlines::sync_focus_ring.after(table::glide),
            table::frame_table,
            table::apply_camera_rig,
            // One entry and not two, because the tuple is at its twenty: a card
            // that has left the hand is taken out of the row before the row
            // is rebuilt, and chaining is what says so. The commands of the
            // first are queued before the commands of the second, so the
            // node is out of the hand zone when the hand zone is despawned.
            (depart::send_off, hud::sync_overlay).chain(),
            hud::apply_hand_scroll,
            (
                hud::light_the_current_step,
                hud::flash_the_designation,
                hud::ease_the_stack_in.after(hud::sync_overlay),
                hud::fold_the_stack.after(hud::sync_overlay),
                hud::reveal_tray.after(hud::sync_tray),
                // The same ordering, and the same reason: the slip under
                // the hover preview is spawned written, and this is what
                // takes the ink back off and washes it on.
                hud::wash_the_slip_in.after(hud::sync_overlay),
                // After the rebuild for the reason `ease_the_stack_in` is:
                // a hand card spawned this frame is spawned where the card
                // already was, and this is what moves it from there.
                touch::settle.after(hud::sync_overlay),
                // After the rebuild too, and for the mirror of that reason:
                // a card taken out of the row this frame is written to its
                // window position by `send_off` and moved from there by
                // this, and a flight advanced before the hand had let go of
                // it would spend its first frame twice.
                depart::fly.after(hud::sync_overlay),
                // The seat bars are ink pinned to a rectangle of felt, so
                // they are measured from the rig the camera was just set
                // from and placed in the same schedule. `bevy_ui` runs its
                // layout *before* transform propagation, so a placer reading
                // the camera's propagated `GlobalTransform` in `PostUpdate`
                // would write a position the layout had already read past,
                // and the ink would swim a frame behind the felt.
                (
                    hud::measure_shelves,
                    hud::sync_seat_bars,
                    hud::place_seat_bars,
                    hud::stretch_step_tiles,
                    hud::describe_phase,
                    hud::highlight_player,
                )
                    .chain()
                    .after(table::apply_camera_rig),
                // The ability sheet is pinned to a *card* rather than to a
                // rectangle of felt, and to where that card **stands** rather
                // than the pose it is drawn in: `table::CardRest` is written
                // by `sync_scene` before a hover or an arming lifts the card,
                // so the paper is not dragged about by the hand crossing the
                // thing it is describing. After that write, and after the rig
                // for the reason the bars are.
                //
                // `zoom_the_sheet` is between the two on purpose. It is what
                // despawns a closing sheet, so it has to run after the system
                // that hands one over; and the scale it writes is one the
                // placer never reads — that one is about *where* the paper
                // is, this one about how much of it has arrived.
                (
                    hud::sync_ability_sheet,
                    hud::zoom_the_sheet,
                    hud::place_ability_sheet,
                )
                    .chain()
                    .after(table::apply_camera_rig)
                    .after(table::sync_scene),
                // A life total changing is drawn over the cell that carries
                // it, so this runs once the bar holding that cell has been
                // rebuilt and placed. It reads `bevy_ui`'s own layout for
                // where the cell is, which is a frame old for the reason
                // above — and a frame is nothing to a number that hangs for
                // a second, where guessing the position from the shelf and
                // the tilt would be the bar's layout written out twice.
                lifeflash::flash_life_changes.after(hud::place_seat_bars),
                // The veil behind a dialog that holds the whole answer. After
                // the rebuild, because the veil *is* part of the retained tree
                // and is spawned clear: how far the fade has risen lives in
                // `hud::Veil`, where a rebuild cannot reach it, and this is
                // what paints it on. See `tray::spawn_veil`.
                // The shelf is filled after the tree that holds it is built, and
                // has a revision of its own for the reason `LedgeRevision`
                // gives: `HudRevision` counts the hover, and a question
                // rebuilt on every pointer move would lose the warmth under
                // the pointer that is about to press it.
                hud::sync_ledge.after(hud::sync_overlay),
                // And the drawer after the shelf, because the centre it stands
                // over is what the shelf has just worked out.
                hud::sync_drawer.after(hud::sync_ledge),
                // And the movement after the reading, on the same frame: a
                // panel `sync_drawer` has just sent away has to be able to
                // leave, and a panel it has just spawned is drawn small on
                // the frame it first appears rather than a frame later.
                hud::zoom_the_drawer.after(hud::sync_drawer),
                // The pool's own row, on its own revision, after the shelf it
                // hangs beside — and its two movements after that, for the
                // drawer's reason: a spent mana has to be able to leave, and
                // so does the strip it was the last thing on.
                // Nested, and it has to stay nested: `add_systems` takes a
                // tuple and a tuple of systems is implemented up to twenty.
                // This set was at twenty, so the pair goes in together rather
                // than the next system to be added failing to compile for a
                // reason that has nothing to do with it.
                (
                    hud::sync_pool.after(hud::sync_ledge),
                    // After the shelf, because the cell it writes into is one
                    // `sync_ledge` spawns: ordered the other way, a countdown
                    // appearing would show an empty cell for its first frame.
                    hud::count_down_the_decision.after(hud::sync_ledge),
                ),
                // The two of them nested as one element on purpose: bevy
                // implements `IntoScheduleConfigs` for tuples up to twenty,
                // and this tuple was at nineteen. A nested tuple is a tuple
                // of systems like any other, so the grouping costs nothing at
                // run time and the pair that belongs together is the one that
                // pays for the ceiling.
                (
                    hud::zoom_the_pool.after(hud::sync_pool),
                    hud::grow_the_pool.after(hud::sync_pool),
                    // The game menu's panel and its movement, in the pool's
                    // nest for the pool's reason — the tuple above is at
                    // bevy's twenty — and ordered after the shelf because the
                    // burger that opens it is drawn there. It hangs off
                    // `HudRoot` rather than off the shelf, so it needs
                    // nothing the shelf worked out; what it must not do is
                    // stand over a shelf that is not there yet.
                    hud::sync_menu.after(hud::sync_ledge),
                    hud::grow_the_menu.after(hud::sync_menu),
                ),
                // The tray hangs off the shelf's edge and not out of its
                // layout, so it needs nothing the shelf worked out — but it
                // is ordered after it anyway, because it is spawned by the
                // same rebuild the shelf is and a frame where the strip
                // exists and the shelf does not would draw a button standing
                // on air.
                hud::sync_tray_strip.after(hud::sync_ledge),
                // The zone dialog, on a revision of its own for the same
                // reason as the shelf and with a louder symptom: the dialog
                // is a hundred rows, and a tree rebuilt on every pointer move
                // despawned the row under the pointer *as the pointer reached
                // it* — the replacement starting at `warmth: 0` and waiting a
                // frame for picking to say `Over` again. The owner reported
                // it as the dialog being unstable.
                //
                // After the overlay, because it hangs its two nodes off that
                // system's root; `sync_overlay` passes them over by marker
                // the way it passes over the shelf and the drawer.
                hud::sync_tray.after(hud::sync_overlay),
                hud::dim_the_table.after(hud::sync_overlay),
                // The end screen settles as that veil rises, off the very
                // number `dim_the_table` has just written: one movement, one
                // rate, one `reduce_motion`.
                hud::settle_the_sheet.after(hud::dim_the_table),
            ),
            textures::drive_preloads,
            textures::load_the_card_back,
            textures::note_load_states,
            textures::retry_failed_loads,
        )
            .in_set(DuelSet::Present)
            .run_if(not(in_state(DuelPhase::Closed))),
    );
    // Its own call because the tuple above is at its twenty, and its own
    // *system* because it belongs to none of them: it is where a frame stops
    // deciding what is worth hearing and hands it over. In `Present` rather
    // than `Sync` for the one reason that matters — that is what lets a cue
    // be taken back, since every source of one and everything that answers a
    // question have run by the time this does.
    app.add_systems(
        Update,
        sound::play_the_cues
            .after(combatfx::age)
            .in_set(DuelSet::Present)
            .run_if(not(in_state(DuelPhase::Closed))),
    );
}

/// Everything a hand does, in the order the frame has to read it in.
///
/// Lifted out of [`DuelPlugin::build`] for [`add_present_systems`]'s
/// reason and no other: this one tuple carries five ordering constraints
/// and about as many paragraphs saying why, and a `build` that holds it
/// inline is a function nobody can read the shape of. Registration order
/// says nothing in Bevy — every order that matters here is written as a
/// `before` or an `after` — so where the call stands does not either.
fn add_input_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            // Before the key path, and for the reason the lobby's
            // sits there too: on a platform that owns the typing the
            // client must not also read raw keys, or a character is
            // entered twice.
            input::browser_softkeys,
            // Before the key path, so the frame the sheet opens on is
            // already one the filter box owns. A letter that reached
            // `look_around` instead is a display toggle or an engine
            // answer fired out of somebody's search term.
            input::browser_takes_the_keyboard.before(input::keyboard),
            input::keyboard,
            // Before the click, and it has to be: a press and the
            // click it turns into arrive on the same frame, so a
            // finger put down *after* its own tap had been answered
            // would leave the card pressed with nothing to lift it.
            touch::watch_the_finger.before(input::pointer),
            // Before `pointer`, and on the *press* rather than the
            // click it becomes: a click on another card has to close
            // this sheet and then open that one, which is two things
            // in that order and not one thing twice.
            input::close_the_sheet_on_a_press_outside_it.before(input::pointer),
            // And the game menu, on the same mechanics and the same
            // ordering: a press outside it shuts it, and a press on its
            // own button is spared so the click can toggle.
            input::close_the_menu_on_a_press_outside_it.before(input::pointer),
            input::pointer,
            input::pointer_hover,
            // `input::camera_controls` used to stand here, and its
            // absence is the point: the owner asked for the general
            // camera movement to go on 14.09.2026, keyboard included,
            // so nothing a hand does reaches the rig any more.
            // `hud::scrolls` no longer shares the wheel with anything
            // and there is no order left to get wrong.
            hud::scrolls,
            input::preview_resize,
            input::tray_drag,
            // After the drag, for the reason `glide_the_sheet`'s own
            // doc gives: both write the sheet's `Node` outside the
            // revision, and a frame in which the two disagreed would
            // be a frame the later one wins by accident rather than
            // by a stated order. They cannot both be running — the
            // maximise button is excluded from `tray_drag`'s press —
            // so the order costs nothing and says so.
            input::glide_the_sheet.after(input::tray_drag),
            face::track_modifier,
        )
            .in_set(DuelSet::Input)
            .run_if(in_state(DuelPhase::Playing)),
    );
}

impl Plugin for DuelPlugin {
    fn build(&self, app: &mut App) {
        add_present_systems(app);
        add_input_systems(app);
        // Shared with the lobby, which is a separate plugin and may already
        // have installed it.
        prefs::install(app);
        ambience::install(app);
        loading::install(app);
        flip::install(app);
        app.add_plugins(cardmat::CardMaterialPlugin)
            .add_plugins(markatlas::MarkAtlasPlugin)
            .add_plugins(feltmat::FeltMaterialPlugin)
            .add_plugins(frontal::FrontalPlugin)
            .add_plugins(matmat::MatMaterialPlugin)
            .add_plugins(arrowmat::ArrowMaterialPlugin)
            .add_plugins(sky::SkyPlugin)
            // Without this nothing on the 3D table can be pointed at, ever.
            //
            // Bevy's UI picking backend is on by default and its *mesh* one is
            // not, so the hand zone — which is UI nodes — answered the pointer
            // while the battlefield, the stack of a hovered permanent and
            // every pile beside a mat did not: `Pointer<Over>` and
            // `Pointer<Click>` simply never fired for a `Mesh3d`. That is why
            // `Interaction::activate` could be written, wired to `input.rs`,
            // and still leave a Forest inert under the cursor, and why the
            // preview only ever appeared for cards in hand. Measured rather
            // than guessed: hovering a hand card reports its object, hovering
            // an opponent's land at the pixel the card is drawn on reports
            // nothing at all.
            //
            // `require_markers` stays `false` — the default, and the one the
            // `Pickable::IGNORE` already on the contact shadows was written
            // against. Everything on the table that is not a card carries
            // that marker, so the felt itself never answers a click, and a
            // card needs no marker of its own. Measured that way round too:
            // with a `Pickable::default()` added to every card the hover was
            // no different, so it is not there.
            //
            // The `mesh_picking` cargo feature this needs cannot be dropped
            // by a later `default-features = false` audit without the build
            // saying so — the path below names the module the feature gates.
            .add_plugins(bevy::picking::mesh_picking::MeshPickingPlugin)
            .init_state::<DuelPhase>()
            .insert_resource(self.config.clone())
            .insert_resource(settings::ClientSettings::load())
            .init_resource::<Duel>()
            // Both are written by systems that run every frame; a missing
            // resource here is a panic at the table, not a compile error.
            .init_resource::<table::SceneIndex>()
            .init_resource::<table::ZoneWatch>()
            .init_resource::<table::CameraRig>()
            .init_resource::<table::ShownRig>()
            .init_resource::<Reconnect>()
            .init_resource::<sheen::Sheen>()
            .init_resource::<touch::Touched>()
            .init_resource::<hud::HudRevision>()
            .init_resource::<hud::StackMotion>()
            .init_resource::<hud::SlipWash>()
            .init_resource::<hud::DesignationFlash>()
            .init_resource::<hud::Shelves>()
            .init_resource::<hud::BarRevision>()
            .init_resource::<hud::LedgeRevision>()
            .init_resource::<hud::LedgeLayout>()
            .init_resource::<hud::DrawerRevision>()
            .init_resource::<hud::PoolRevision>()
            .init_resource::<hud::StripRevision>()
            .init_resource::<hud::TrayRevision>()
            .init_resource::<hud::SheetRevision>()
            .init_resource::<hud::Veil>()
            .init_resource::<textures::Preload>()
            .init_resource::<cardtext::CardTexts>()
            .init_resource::<face::FaceMode>()
            .init_resource::<combatlines::LineAssets>()
            .init_resource::<combatlines::FocusAssets>()
            // Shared with the lobby, which may already have installed it: the
            // table has one text field of its own, the browser's filter box.
            .init_resource::<softkeys::SoftKeyboard>()
            .add_message::<DuelCommand>()
            .add_message::<DuelReport>()
            .configure_sets(
                Update,
                (DuelSet::Sync, DuelSet::Input, DuelSet::Present).chain(),
            )
            .add_systems(
                Startup,
                (
                    textures::setup,
                    hud::setup_fonts,
                    hud::setup_sheets,
                    // Once, on the frame the app opens: thirty-seven
                    // buffers of arithmetic, and thereafter thirty-seven
                    // handles. See `sound`'s header for why they are
                    // computed and not shipped, and what the count buys.
                    sound::voice_the_cues,
                ),
            )
            .add_systems(
                Update,
                (
                    handle_commands,
                    poll_host,
                    keep_the_table_connected.run_if(duel_is_live),
                    run_mana_plan,
                    // After the run and before the HUD is built, which is
                    // both halves of where it belongs: the run is what puts
                    // the mana up and sends the cast, and a frame drawn
                    // between the engine's question and this answer would
                    // flash the engine's own chooser over a choice the player
                    // already made.
                    answer_the_chosen_cast_mode,
                    run_autopilot,
                    flush_outbox,
                    cardtext::request,
                    cardtext::poll,
                )
                    .chain()
                    .in_set(DuelSet::Sync),
            )
            .add_systems(OnEnter(DuelPhase::Opening), table::spawn_stage)
            // The end screen. Built on the edge because a result never
            // changes, and taken down on the way out of `Finished` — which
            // covers the way to `Closed` too, so it needs no line in the
            // teardown below.
            .add_systems(OnEnter(DuelPhase::Finished), hud::spawn_finish)
            .add_systems(OnExit(DuelPhase::Finished), hud::despawn_finish)
            .add_systems(
                OnEnter(DuelPhase::Closed),
                (
                    table::despawn_stage,
                    hud::despawn_overlay,
                    textures::reset_game,
                ),
            );
    }
}

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
fn poll_host(
    host: Option<ResMut<InstalledHost>>,
    mut duel: ResMut<Duel>,
    mut textures: ResMut<textures::CardTextures>,
    phase: Res<State<DuelPhase>>,
    mut next: ResMut<NextState<DuelPhase>>,
    mut reports: MessageWriter<DuelReport>,
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
                duel.statics = Some(*statics);
                duel.ability_orders_applied = None;
                // A print table arriving is the one event that can turn an
                // unresolvable printing into a resolvable one: this payload is
                // re-sent, before the view that needs it, whenever the seat
                // earns an entry it did not have. Without this the first ask
                // decided the answer for the whole game, and a permanent whose
                // entry arrived a frame late never drew its art again.
                textures.forget_unresolved();
            }
            HostMessage::View(view) => {
                duel.receive_view(*view);
                rebuild_board(&mut duel);
                if *phase.get() == DuelPhase::Opening {
                    next.set(DuelPhase::Playing);
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
    if duel.view.is_some()
        && duel.ability_orders_applied.as_ref() != Some(&prefs.all().ability_orders)
    {
        let old = duel.ability_orders_applied.take().unwrap_or_default();
        for order in &old {
            if !prefs
                .all()
                .ability_orders
                .iter()
                .any(|new| new.ability == order.ability)
            {
                duel.submit(automation::AbilityOrder::manual(order.ability).action());
            }
        }
        for order in &prefs.all().ability_orders {
            if !old.contains(order) {
                duel.submit(order.action());
            }
        }
        duel.ability_orders_applied = Some(prefs.all().ability_orders.clone());
        if !duel.outbox.is_empty() {
            return;
        }
    }
    if duel.stack_stop_requested {
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
                offering: !duel.reachable.is_empty() || !duel.suspend_reach.is_empty(),
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
        }
    }

    /// The spell being paid for — the HUD says so while it happens.
    #[must_use]
    pub const fn card(&self) -> ObjectId {
        self.card
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
fn run_mana_plan(mut duel: ResMut<Duel>) {
    advance_mana_run(&mut duel);
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
fn answer_the_chosen_cast_mode(mut duel: ResMut<Duel>) {
    take_the_chosen_cast_mode(&mut duel);
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
    // A hand card, or a commander standing in the command zone. The two are
    // the only places this client offers to tap lands *for*, and they have to
    // be the same two [`reachable`] admits — a card in one set and not the
    // other is a card that lights up and then does nothing when it is
    // clicked.
    let cost = if let Some(hand_card) = view.hand.iter().find(|c| c.id == card) {
        manasources::hand_cost(hand_card)?
    } else {
        let commander = view
            .seat(view.seat)?
            .commanders
            .iter()
            .find(|c| c.object == card)?;
        commander_cost(commander)?.with_more_generic(2 * commander.casts)
    };
    let pool = view.seat(view.seat)?.mana_pool;
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
    let hand_card = view.hand.iter().find(|c| c.id == card)?;
    let cost = suspend_cost(hand_card.card)?;
    let pool = view.seat(view.seat)?.mana_pool;
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
        LinkState::Local | LinkState::Up => {
            retry.schedule.settle();
            retry.told = false;
            duel.link_note = None;
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
    if duel.outbox.is_empty() {
        return;
    }
    for action in std::mem::take(&mut duel.outbox) {
        host.0.submit(action);
    }
    // The answer has been sent; the next choice replaces this one.
    duel.interaction = None;
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
    let mut layout =
        TableLayout::seated(&seats, duel.canvas_aspect.unwrap_or(16.0 / 9.0), duel.focus);
    for slot in &mut layout.slots {
        if view
            .seats
            .iter()
            .any(|seat| seat.player == slot.player && seat.commanders.is_empty())
        {
            slot.reclaim_command_strip();
        }
    }

    duel.board = Some(BoardModel::from_view(
        view,
        baylee_client_core::board::Openings {
            playable: &playable,
            reachable: &reach,
            activatable: &duel.activatable,
        },
        // Each pod is measured against its own row. This used to be one
        // number taken off the first opponent, which is only ever right on a
        // table nobody has focused: a focus widens the seat it is on and
        // shrinks the rest, so the local board was being gated against a
        // 23.7-unit row while standing on a 14.7-unit one.
        |player| {
            layout
                .slot(player)
                .map_or(12.0, baylee_client_core::layout::SeatSlot::lane_width)
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

/// Which permanents have an ability that can be activated right now.
///
/// Straight off `LegalActions`, both halves of it: `mana_abilities` names a
/// source once however many mana abilities it has, `abilities` names a
/// `(source, index)` pair per ability. The table only needs "does this card
/// have anything to do", so both collapse to the same set of sources — the
/// menu that asks *which* one is built later, from the same list, by
/// `abilities::options`.
fn activatable(duel: &Duel) -> std::collections::HashSet<ObjectId> {
    duel.interaction
        .as_ref()
        .and_then(Interaction::legal_actions)
        .map(|legal| {
            legal
                .mana_abilities
                .iter()
                .copied()
                .chain(legal.abilities.iter().map(|(source, _)| *source))
                .collect()
        })
        .unwrap_or_default()
}

/// Which cards in hand a tap or two would make castable.
///
/// The engine answers "castable" against the mana already floating, which is
/// the correct rules answer and a hand that looks empty to a player with five
/// untapped lands. This is the other half of that question, and the board
/// model draws it differently from `playable` on purpose: one is what the
/// game says, the other is what this client is offering to do about it.
///
/// Two questions, both of which have to be answered yes. "Can the lands pay
/// for it" is [`baylee_client_core::manaplan`]; "could it be cast at all right
/// now" is [`baylee_client_core::timing`], and it was missing — so a sorcery
/// was lit and clickable on an opponent's turn and over a stack that had not
/// resolved. The offer is not free: taking it taps the lands *first*, and the
/// engine then refuses the cast, which spends the turn's mana on nothing.
fn reachable(duel: &Duel) -> std::collections::HashSet<ObjectId> {
    let Some(view) = duel.view.as_ref() else {
        return std::collections::HashSet::new();
    };
    let Some(legal) = duel
        .interaction
        .as_ref()
        .and_then(Interaction::legal_actions)
    else {
        return std::collections::HashSet::new();
    };
    // Nothing to reach for while there is nothing to tap.
    let sources = manasources::sources(view, legal);
    if sources.is_empty() {
        return std::collections::HashSet::new();
    }
    let Some(pool) = view.seat(view.seat).map(|s| s.mana_pool) else {
        return std::collections::HashSet::new();
    };
    let affordable = |cost: baylee_core::mana::ManaCost| {
        baylee_client_core::manaplan::plan(&cost, &pool, &sources).is_some()
    };
    view.hand
        .iter()
        .filter(|card| !legal.castable.contains(&card.id) && !legal.lands.contains(&card.id))
        // Types off the view, because those are the *projected* ones; flash
        // off the printed card, because a `HandObject` carries no keywords.
        .filter(|card| baylee_client_core::timing::allows(view, card.types, has_flash(card.card)))
        // And it has to print a cost at all (CR 202.1b). A blank cost is
        // payable by an empty pool, so a suspend-only card was "reachable"
        // with a plan of no taps at all: the click armed a run, the run
        // finished at once and asked the engine to cast a card it will never
        // offer. The engine has the same rule in `casting::has_a_printed_cost`
        // — this is the client not offering what that will refuse.
        .filter(|card| {
            manasources::hand_cost(card).is_some_and(|cost| cost.symbols().next().is_some())
        })
        .filter(|card| manasources::hand_cost(card).is_some_and(&affordable))
        // And there has to be something to point it at (CR 601.2c). Last in
        // the chain because it is the only filter here that walks the
        // battlefield, so it is asked only about a card the rest already
        // agreed on. `targeting` withholds the offer solely when it can
        // *prove* no legal target exists and offers whenever it cannot tell,
        // which is the opposite default from `castmodes::parts_payable` and
        // deliberately so — the reasoning is in that module's header.
        .filter(|card| !targeting::provably_targetless(view, card))
        .map(|card| card.id)
        // The command zone is castable from too (CR 903.8), and leaving it
        // out is why a commander could not be played. A card the engine has
        // not offered yet — because the mana is not floating — is reached
        // for by tapping lands, and this set is the whole of what the client
        // will offer to do that for. It only ever read the hand, so the one
        // card a commander deck is built around answered no click at all:
        // not `castable`, not `reachable`, no ability to activate, so the
        // tap fell through to the last branch and opened the zone browser.
        //
        // The tax is part of the cost and has to be added here rather than
        // read off the card: CR 903.8 is `{2}` more generic for each previous
        // cast *of that commander*, which is why `CommanderView::casts` is
        // per commander and not per seat.
        .chain(
            view.seat(view.seat)
                .into_iter()
                .flat_map(|seat| seat.commanders.iter())
                .filter(|c| !legal.castable.contains(&c.object))
                // Still *in* the zone. A commander on the battlefield or in a
                // graveyard is named by the same `CommanderView` — the handle
                // follows the card through every move (CR 400.7) — and only
                // the one standing in the command zone is castable from it.
                .filter(|c| {
                    view.command
                        .get(view.seat.get() as usize)
                        .is_some_and(|zone| zone.iter().any(|o| o.id == c.object))
                })
                // And it is cast at the timing it would have from a hand
                // (CR 903.8) — which for the legend most commander decks are
                // built around is sorcery speed, so the same gate.
                .filter(|c| {
                    c.card.is_some_and(|card| {
                        baylee_client_core::timing::allows(
                            view,
                            printed_face(card)
                                .map_or(baylee_core::types::TypeSet::EMPTY, |f| f.types),
                            has_flash(card),
                        )
                    })
                })
                .filter(|c| {
                    commander_cost(c)
                        .map(|cost| cost.with_more_generic(2 * c.casts))
                        .is_some_and(&affordable)
                })
                .map(|c| c.object),
        )
        .collect()
}

/// Which cards in hand a tap or two would make **suspendable**.
///
/// [`reachable`]'s counterpart, and it needs one because suspend is paid for
/// like anything else: the engine offers `legal.suspendable` only once the
/// cost is floating, so a suspend card over untapped lands is a card with
/// nothing to do — which is precisely the report this answers, *"so first tap
/// and pay mana, then suspend"*.
///
/// The timing gate is [`baylee_client_core::timing::sorcery_window`] and not
/// `allows`: suspend's ability says "activate only as a sorcery" (CR 702.62a)
/// whatever the card's own type is, and the engine gates its offer on exactly
/// that. Flash on the card would be the wrong question — an instant with
/// suspend still may not suspend at instant speed.
fn suspend_reach(duel: &Duel) -> std::collections::HashSet<ObjectId> {
    let Some(view) = duel.view.as_ref() else {
        return std::collections::HashSet::new();
    };
    let Some(legal) = duel
        .interaction
        .as_ref()
        .and_then(Interaction::legal_actions)
    else {
        return std::collections::HashSet::new();
    };
    if !baylee_client_core::timing::sorcery_window(view) {
        return std::collections::HashSet::new();
    }
    let sources = manasources::sources(view, legal);
    if sources.is_empty() {
        return std::collections::HashSet::new();
    }
    let Some(pool) = view.seat(view.seat).map(|s| s.mana_pool) else {
        return std::collections::HashSet::new();
    };
    view.hand
        .iter()
        .filter(|card| !legal.suspendable.contains(&card.id))
        .filter(|card| {
            suspend_cost(card.card).is_some_and(|cost| {
                baylee_client_core::manaplan::plan(&cost, &pool, &sources).is_some()
            })
        })
        .map(|card| card.id)
        .collect()
}

/// What a card's suspend ability costs, or `None` when it has none.
///
/// Off the card and not off the view: `AbilityDef::Suspend` is printed data,
/// and a `HandObject` carries the projected characteristics rather than the
/// abilities. The card's own list rather than the face's, for the reason
/// `keywords_for_face` exists — a single-faced card keeps its abilities on
/// the `CardDef` and leaves the face's list empty.
fn suspend_cost(card: baylee_view::CardIdentity) -> Option<baylee_core::mana::ManaCost> {
    let def = baylee_cards::by_index(card.index)?;
    def.abilities.iter().find_map(|a| match a {
        baylee_cards_dsl::AbilityDef::Suspend { cost, .. } => Some(*cost),
        _ => None,
    })
}

/// A commander's printed cost, before CR 903.8's tax.
///
/// The card is named on the [`baylee_view::CommanderView`] itself rather than
/// looked up through the object, because a commander is public in every zone
/// (CR 903.3) and the view says so there whatever zone it is sitting in.
/// The printed face a `CardIdentity` names, out of the compiled registry.
///
/// A card the registry does not have answers `None`, and both callers read
/// that as the cautious thing: no types and no flash, which
/// [`baylee_client_core::timing::allows`] treats as sorcery speed. That is
/// the same reading `manasources::hand_cost` already gives such a card by
/// offering no cost at all.
fn printed_face(card: baylee_view::CardIdentity) -> Option<&'static baylee_cards_dsl::FaceDef> {
    let def = baylee_cards::by_index(card.index)?;
    def.faces.get(card.face as usize).or(def.faces.first())
}

/// Whether the card behind a view's handle prints flash (CR 702.8a).
///
/// `keywords_for_face` and not `face.keywords`, because a single-faced card
/// keeps its keywords on the `CardDef` and leaves the face's list empty.
fn has_flash(card: baylee_view::CardIdentity) -> bool {
    baylee_cards::by_index(card.index).is_some_and(|def| {
        def.keywords_for_face(card.face as usize)
            .contains(baylee_cards_dsl::KeywordSet::FLASH)
    })
}

fn commander_cost(c: &baylee_view::CommanderView) -> Option<baylee_core::mana::ManaCost> {
    let card = c.card?;
    let def = baylee_cards::by_index(card.index)?;
    let face = def.faces.get(card.face as usize).or(def.faces.first())?;
    Some(face.mana_cost)
}

#[cfg(test)]
mod reconnect_tests;

#[cfg(test)]
mod commander_reach_tests;

#[cfg(test)]
mod owed_tests;

#[cfg(test)]
mod reachable_tests;

#[cfg(test)]
mod schedule_order_tests;

#[cfg(test)]
mod cue_feed_tests;

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
    object
}

#[cfg(test)]
mod stack_tests;
