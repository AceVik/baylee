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
pub mod buildui;
pub mod cardmat;
pub mod cardtext;
pub mod choices;
/// The dev-control harness. Native dev builds only; see the module docs for
/// why it is a compile-time feature rather than a runtime switch.
#[cfg(all(feature = "dev-control", not(target_arch = "wasm32")))]
pub mod devctl;
pub mod face;
pub mod flip;
pub mod host;
pub mod hud;
pub mod input;
pub mod keys;
pub mod loading;
pub mod lobby;
pub mod manasources;
pub mod manaui;
pub mod net;
pub mod prefs;
pub mod rivermat;
pub mod settings;
pub mod settingsui;
pub mod softkeys;
pub mod table;
pub mod textures;

use baylee_client_core::automation::{self, AutoPilot, Situation};
use baylee_client_core::board::BoardModel;
use baylee_client_core::i18n::Phrase;
use baylee_client_core::interaction::Interaction;
use baylee_client_core::layout::TableLayout;
use baylee_client_core::reconnect::Retry;
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::{GameStatic, PlayerView};
use bevy::platform::collections::HashSet;
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
/// closes, and a schedule that reset there would forget how long it had been
/// trying every time anything else about the duel changed.
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
    /// Tap the sources of this plan, then cast — see [`ManaRun`].
    Run(baylee_client_core::manaplan::Plan),
}

/// The client's own state for one duel.
#[derive(Resource, Default)]
pub struct Duel {
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
    /// Hand bar scroll offset in pixels.
    pub hand_scroll: f32,
    /// Whether the own-board overlay is raised over the table.
    ///
    /// Default `false`, and the polarity is the point: the overlay is an
    /// opaque panel the width of the canvas, so a default of "open" hides
    /// the table, the mats, the cards and every animation on them behind a
    /// sheet of `palette::PANEL`. It is opt-in (the `X` action, or the knob
    /// on its edge), and the derived `Default` has to land on the table.
    pub overlay_open: bool,
    /// Slide position of the own-board overlay: 0.0 = down (closed),
    /// 1.0 = raised (open). Animated towards `overlay_open`.
    pub overlay_t: f32,
    /// Whether the preview resize handle is being dragged.
    pub resize_drag: bool,
    /// The taps the client is making on the player's behalf, if any.
    pub mana_run: Option<ManaRun>,
    /// Cards in hand that are not castable yet and would be after tapping.
    ///
    /// Kept beside the board model rather than in it: it is a *client*
    /// judgement, not something the engine said, and the difference is worth
    /// keeping visible at the type level.
    pub reachable: std::collections::HashSet<ObjectId>,
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
    /// Which entry of that menu the keyboard is on.
    ///
    /// A menu the pointer can answer and the keyboard cannot is not a menu,
    /// it is a trap — and it was one: the chooser used to swallow every key
    /// except cancel. Reset whenever the menu opens, and clamped to the list
    /// as it is rebuilt, because the engine may withdraw an ability while
    /// the menu stands.
    pub ability_pick: usize,
    /// The zone browser: every zone a choice can reach that the table
    /// cannot draw.
    ///
    /// Beside the interaction rather than inside it, and holding no
    /// selection of its own: the interaction is the one truth about the
    /// answer being assembled, and two copies of a selection are two things
    /// that can disagree. What lives here is only what the *player* said
    /// about the panel — open, which tab, what is typed.
    pub browser: baylee_client_core::browser::Browser,
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
    /// Actions waiting to be sent.
    outbox: Vec<PlayerAction>,
    /// The last thing that went wrong, shown in the prompt bar.
    pub last_error: Option<String>,
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
    /// Queues an action for the host.
    ///
    /// Queuing rather than sending directly keeps every mutation of the game on
    /// one system boundary, which is what lets input handlers stay plain
    /// functions of the board model.
    pub fn submit(&mut self, action: PlayerAction) {
        // Whatever was typed belonged to the question just answered.
        self.subtype_filter.clear();
        // And so did the last refusal. Cleared here rather than when a new
        // question arrives, because the acting seat is re-sent its own
        // question every time anybody says anything — a refusal would have
        // flashed and been gone before it was read. It stands until this
        // player tries something else.
        self.last_error = None;
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
    pub(crate) fn receive_choice(&mut self, pending: Pending) {
        let seat = self.seat().unwrap_or(PlayerId::new(0));
        if !matches!(pending, Pending::ChooseSubtype { .. }) {
            self.subtype_filter.clear();
        }
        self.interaction = Some(Interaction::new(pending, seat));
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

    /// The local seat, once the static payload has arrived.
    #[must_use]
    pub fn seat(&self) -> Option<PlayerId> {
        self.statics.as_ref().map(|s| s.your_seat)
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
        } else {
            baylee_engine::choice::PriorityHold::UntilStackEmpty {
                depth: u16::try_from(view.stack.len()).unwrap_or(u16::MAX),
            }
        };
        Some(PlayerAction::SetPriorityHold(hold))
    }

    /// Whether the engine would take a draw offer right now.
    ///
    /// `Engine::offer_draw` refuses anything but the offerer's own priority,
    /// because the offer suspends a decision that has to be handed back
    /// untouched if anyone refuses (CR 104.4a). The button was drawn live
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

impl Plugin for DuelPlugin {
    fn build(&self, app: &mut App) {
        // Shared with the lobby, which is a separate plugin and may already
        // have installed it.
        prefs::install(app);
        ambience::install(app);
        loading::install(app);
        flip::install(app);
        app.add_plugins(cardmat::CardMaterialPlugin)
            .add_plugins(rivermat::RiverMaterialPlugin)
            // Without this nothing on the 3D table can be pointed at, ever.
            //
            // Bevy's UI picking backend is on by default and its *mesh* one is
            // not, so the hand bar — which is UI nodes — answered the pointer
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
            .init_resource::<table::CameraRig>()
            .init_resource::<table::ShownRig>()
            .init_resource::<table::HomeRig>()
            .init_resource::<Reconnect>()
            .init_resource::<hud::HudRevision>()
            .init_resource::<textures::Preload>()
            .init_resource::<cardtext::CardTexts>()
            .init_resource::<face::FaceMode>()
            .add_message::<DuelCommand>()
            .add_message::<DuelReport>()
            .configure_sets(
                Update,
                (DuelSet::Sync, DuelSet::Input, DuelSet::Present).chain(),
            )
            .add_systems(Startup, (textures::setup, hud::setup_fonts))
            .add_systems(
                Update,
                (
                    handle_commands,
                    poll_host,
                    keep_the_table_connected.run_if(duel_is_live),
                    run_mana_plan,
                    run_autopilot,
                    flush_outbox,
                    cardtext::request,
                    cardtext::poll,
                )
                    .chain()
                    .in_set(DuelSet::Sync),
            )
            .add_systems(
                Update,
                (
                    input::keyboard,
                    input::pointer,
                    input::pointer_hover,
                    input::camera_controls,
                    input::preview_resize,
                    face::track_modifier,
                )
                    .in_set(DuelSet::Input)
                    .run_if(in_state(DuelPhase::Playing)),
            )
            .add_systems(
                Update,
                (
                    table::track_canvas,
                    table::sync_scene,
                    table::sync_zones,
                    table::sync_river,
                    table::glide,
                    table::frame_table,
                    table::apply_camera_rig,
                    hud::sync_overlay,
                    hud::apply_hand_scroll,
                    hud::animate_overlay,
                    textures::drive_preloads,
                    textures::note_load_states,
                )
                    .in_set(DuelSet::Present)
                    .run_if(not(in_state(DuelPhase::Closed))),
            )
            .add_systems(OnEnter(DuelPhase::Opening), table::spawn_stage)
            .add_systems(
                OnEnter(DuelPhase::Closed),
                (table::despawn_stage, hud::despawn_overlay),
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
            HostMessage::Static(statics) => duel.statics = Some(*statics),
            HostMessage::View(view) => {
                duel.view = Some(*view);
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
                duel.last_error = Some(reason.clone());
                reports.write(DuelReport::Failed(reason));
            }
        }
    }
}

/// Applies the standing orders and the autopilot: hands control back at
/// the boundary, and never makes a real decision for the player.
fn run_autopilot(mut duel: ResMut<Duel>, prefs: Res<prefs::Prefs>) {
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

/// Tapping lands for a spell, one action at a time.
///
/// The plan is decided in one go (`manaplan::plan`) and then spent one step
/// per engine round trip, because that is how the engine works: every
/// activation is an action, and each one comes back as a fresh `Pending` with
/// a fresh `LegalActions`. That round trip is also the safety property — each
/// step is re-checked against what the engine is offering *now*, so a plan
/// that has gone stale stops instead of guessing.
#[derive(Debug)]
pub struct ManaRun {
    /// Taps still to make.
    steps: std::collections::VecDeque<baylee_client_core::manaplan::Step>,
    /// The colour to answer with while an ability is asking for one.
    asking: Option<baylee_core::mana::ManaColor>,
    /// The spell all of this is for.
    card: ObjectId,
}

impl ManaRun {
    /// Starts a run for `card`.
    #[must_use]
    pub fn new(plan: baylee_client_core::manaplan::Plan, card: ObjectId) -> Self {
        Self {
            steps: plan.steps.into(),
            asking: None,
            card,
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
                None => abort = Some("that source cannot make the colour the plan wanted"),
            }
        }
        Pending::Priority { player, legal } if *player == seat => {
            let step = duel.mana_run.as_mut().and_then(|r| r.steps.pop_front());
            if let Some(step) = step {
                action = tap_action(&step, legal);
                if action.is_none() {
                    abort = Some("a land the plan counted on can no longer be tapped");
                } else if let Some(run) = duel.mana_run.as_mut() {
                    run.asking = step.color;
                }
            } else {
                // Every tap is made; the mana is floating and the engine is
                // offering the spell it was floated for.
                let card = duel.mana_run.as_ref().map(ManaRun::card);
                match card.filter(|c| legal.castable.contains(c)) {
                    Some(card) => action = Some(PlayerAction::CastSpell { card }),
                    None => abort = Some("the mana is up but the spell is not castable"),
                }
                finished = true;
            }
        }
        // Mana abilities do not use the stack, so priority never leaves the
        // seat in the middle of a plan. Anything else means the game moved on
        // without us and the plan is void.
        _ => abort = Some("the game asked something else"),
    }

    if let Some(reason) = abort {
        duel.last_error = Some(reason.to_string());
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

/// The taps that would make `card` castable, if any.
///
/// `None` both when the spell needs no help and when nothing here can pay for
/// it — the caller has already asked the engine the first question.
#[must_use]
pub fn mana_for(duel: &Duel, card: ObjectId) -> Option<baylee_client_core::manaplan::Plan> {
    let view = duel.view.as_ref()?;
    let legal = duel.interaction.as_ref()?.legal_actions()?;
    let hand_card = view.hand.iter().find(|c| c.id == card)?;
    let cost = manasources::hand_cost(hand_card)?;
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
        // not information anyone can act on.
        LinkState::Connecting => duel.link_note = Some(Phrase::LinkLost),
        LinkState::Down => {
            if retry.schedule.exhausted() {
                duel.link_note = Some(Phrase::LinkGaveUp);
                if !retry.told {
                    retry.told = true;
                    reports.write(DuelReport::Unreachable);
                }
            } else {
                duel.link_note = Some(Phrase::LinkLost);
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
pub(crate) fn rebuild_board(duel: &mut Duel) {
    duel.reachable = reachable(duel);
    duel.activatable = activatable(duel);

    let Some(view) = duel.view.as_ref() else {
        return;
    };
    let playable: HashSet<ObjectId> = duel
        .interaction
        .as_ref()
        .and_then(Interaction::legal_actions)
        .map(|legal| {
            legal
                .lands
                .iter()
                .chain(legal.castable.iter())
                .copied()
                .collect()
        })
        .unwrap_or_default();
    let playable: std::collections::HashSet<ObjectId> = playable.into_iter().collect();

    let seats: Vec<PlayerId> = std::iter::once(view.seat)
        .chain(view.opponents_in_turn_order())
        .collect();
    let layout = TableLayout::new(&seats, duel.canvas_aspect.unwrap_or(16.0 / 9.0), duel.focus);
    let pod_width = layout
        .slots
        .iter()
        .find(|s| !s.is_local)
        .map_or(12.0, baylee_client_core::layout::SeatSlot::lane_width);

    duel.board = Some(BoardModel::from_view(
        view,
        baylee_client_core::board::Openings {
            playable: &playable,
            reachable: &duel.reachable,
            activatable: &duel.activatable,
        },
        pod_width,
    ));
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
    view.hand
        .iter()
        .filter(|card| !legal.castable.contains(&card.id) && !legal.lands.contains(&card.id))
        .filter(|card| {
            manasources::hand_cost(card)
                .and_then(|cost| baylee_client_core::manaplan::plan(&cost, &pool, &sources))
                .is_some()
        })
        .map(|card| card.id)
        .collect()
}

#[cfg(test)]
mod reconnect_tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    /// A host whose link state the test holds the other end of.
    ///
    /// It never actually comes back on its own: `reconnect` counts the dial
    /// and leaves the state alone, so the schedule can be watched running out
    /// rather than being cut short by a lucky reconnection.
    struct FakeHost {
        state: Arc<Mutex<LinkState>>,
        dials: Arc<Mutex<usize>>,
    }

    impl DuelHost for FakeHost {
        fn poll(&mut self) -> Vec<HostMessage> {
            Vec::new()
        }
        fn submit(&mut self, _: PlayerAction) {}
        fn seat(&self) -> PlayerId {
            PlayerId::new(0)
        }
        fn link(&self) -> LinkState {
            *self.state.lock().unwrap()
        }
        fn reconnect(&mut self) -> Result<(), String> {
            *self.dials.lock().unwrap() += 1;
            Ok(())
        }
    }

    /// How many times the app said the table could not be reached.
    ///
    /// Counted by a real reader rather than by inspecting the buffer, which
    /// is also what proves the report is deliverable at all.
    #[derive(Resource, Default)]
    struct Unreachables(usize);

    /// Drains the reports the way an embedding shell would.
    fn count_unreachable(mut reader: MessageReader<DuelReport>, mut seen: ResMut<Unreachables>) {
        seen.0 += reader
            .read()
            .filter(|r| matches!(r, DuelReport::Unreachable))
            .count();
    }

    /// An app with just enough in it to run the one system.
    fn app_with(state: LinkState) -> (App, Arc<Mutex<LinkState>>, Arc<Mutex<usize>>) {
        let link = Arc::new(Mutex::new(state));
        let dials = Arc::new(Mutex::new(0));
        let host = FakeHost {
            state: Arc::clone(&link),
            dials: Arc::clone(&dials),
        };
        let mut app = App::new();
        app.init_resource::<Duel>()
            .init_resource::<Reconnect>()
            .insert_resource(Time::<()>::default())
            .insert_resource(InstalledHost(Box::new(host)))
            .add_message::<DuelReport>()
            .init_resource::<Unreachables>()
            .add_systems(
                Update,
                (super::keep_the_table_connected, count_unreachable).chain(),
            );
        (app, link, dials)
    }

    /// Moves the clock on and runs one frame.
    fn advance(app: &mut App, seconds: f32) {
        app.world_mut()
            .resource_mut::<Time<()>>()
            .advance_by(Duration::from_secs_f32(seconds));
        app.update();
    }

    /// The whole point: nothing outside the client asks for this. A socket
    /// that goes away is dialled again on the client's own initiative, which
    /// is what `NetworkHost::redial` could always do and what nothing ever
    /// called — a dropped connection simply ended the game.
    #[test]
    fn a_table_that_drops_is_dialled_again_without_anyone_asking() {
        let (mut app, _link, dials) = app_with(LinkState::Down);

        advance(&mut app, 0.1);
        assert_eq!(*dials.lock().unwrap(), 0, "not instantly");
        assert_eq!(
            app.world().resource::<Duel>().link_note,
            Some(Phrase::LinkLost),
            "but the player is told at once"
        );

        advance(&mut app, 0.5);
        assert_eq!(*dials.lock().unwrap(), 1, "half a second in");
    }

    /// A host with no socket must never enter the schedule. An in-process
    /// engine cannot be disconnected from, so a client that treated it as a
    /// dead link would "reconnect" to it twelve times and then tell a solo
    /// player their table was unreachable.
    #[test]
    fn a_local_host_is_never_dialled() {
        let (mut app, _link, dials) = app_with(LinkState::Local);
        for _ in 0..40 {
            advance(&mut app, 5.0);
        }
        assert_eq!(*dials.lock().unwrap(), 0);
        assert_eq!(app.world().resource::<Duel>().link_note, None);
    }

    /// A dial in flight is not a reason to dial again. Without this the
    /// system would fire once per frame for as long as the socket took to
    /// open, which is every frame of the two seconds a bad network needs.
    #[test]
    fn a_dial_in_flight_is_left_alone() {
        let (mut app, _link, dials) = app_with(LinkState::Connecting);
        for _ in 0..120 {
            advance(&mut app, 0.5);
        }
        assert_eq!(*dials.lock().unwrap(), 0, "it is already dialling");
        assert_eq!(
            app.world().resource::<Duel>().link_note,
            Some(Phrase::LinkLost)
        );
    }

    /// The schedule ends, and says so once rather than once a frame. An
    /// unbounded retry against a game the gateway has already finished would
    /// spin until the player closed the window — and "that game is over"
    /// arrives as a refusal string, not as something a client can match on.
    #[test]
    fn a_table_that_cannot_be_reached_stops_and_says_so() {
        let (mut app, _link, dials) = app_with(LinkState::Down);
        for _ in 0..80 {
            advance(&mut app, 20.0);
        }
        assert_eq!(
            *dials.lock().unwrap(),
            baylee_client_core::reconnect::Retry::GIVE_UP as usize,
            "it stopped where the schedule said it would"
        );
        assert_eq!(
            app.world().resource::<Duel>().link_note,
            Some(Phrase::LinkGaveUp)
        );

        // Counted as they were written rather than read off the buffer at the
        // end: `Messages` is double-buffered and drops what nobody read
        // within two frames, so a test that looked afterwards would find
        // nothing however many had been sent.
        assert_eq!(
            app.world().resource::<Unreachables>().0,
            1,
            "told once, not once a frame"
        );
    }

    /// A table that comes back takes the notice off the bar and resets the
    /// schedule, so the *next* drop is dialled promptly rather than at the
    /// cap the last one ended on.
    #[test]
    fn a_table_that_comes_back_clears_the_notice_and_the_schedule() {
        let (mut app, link, dials) = app_with(LinkState::Down);
        for _ in 0..4 {
            advance(&mut app, 20.0);
        }
        let during = *dials.lock().unwrap();
        assert!(during >= 4, "it was dialling: {during}");

        *link.lock().unwrap() = LinkState::Up;
        advance(&mut app, 0.1);
        assert_eq!(app.world().resource::<Duel>().link_note, None);
        assert_eq!(*dials.lock().unwrap(), during, "and stopped dialling");

        // Down again: prompt, not at the cap the last outage ended on.
        *link.lock().unwrap() = LinkState::Down;
        advance(&mut app, 0.6);
        assert_eq!(
            *dials.lock().unwrap(),
            during + 1,
            "the next drop starts the schedule over"
        );
    }
}
