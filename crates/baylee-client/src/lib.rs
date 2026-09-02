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
pub mod buildui;
pub mod cardmat;
pub mod cardtext;
pub mod face;
pub mod host;
pub mod hud;
pub mod input;
pub mod keys;
pub mod lobby;
pub mod manasources;
pub mod manaui;
pub mod net;
pub mod prefs;
pub mod settings;
pub mod settingsui;
pub mod softkeys;
pub mod table;
pub mod textures;

use baylee_client_core::automation::{self, AutoPilot, Situation};
use baylee_client_core::board::BoardModel;
use baylee_client_core::interaction::Interaction;
use baylee_client_core::layout::TableLayout;
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::{GameStatic, PlayerView};
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use host::{DuelHost, HostMessage};

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
    Failed(String),
}

/// The installed source of duel state.
#[derive(Resource)]
pub struct InstalledHost(pub Box<dyn DuelHost>);

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
    /// The engaged autopilot, if any ("next phase" / "end turn").
    pub autopilot: Option<AutoPilot>,
    /// Hand bar scroll offset in pixels.
    pub hand_scroll: f32,
    /// Whether the own-board overlay is slid down (hidden). Defaults to
    /// false — the overlay starts raised.
    pub overlay_closed: bool,
    /// Slide position of the own-board overlay: 0.0 = raised (open),
    /// 1.0 = down (closed). Animated towards `overlay_closed`.
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
    /// The permanent whose abilities the prompt bar is offering.
    ///
    /// Only ever set for one with more than one thing to do: a single
    /// ability activates on the click that found it, because a menu of one is
    /// a menu that only ever wastes a tap.
    pub ability_menu: Option<ObjectId>,
    /// Actions waiting to be sent.
    outbox: Vec<PlayerAction>,
    /// The last thing that went wrong, shown in the prompt bar.
    pub last_error: Option<String>,
}

impl Duel {
    /// Queues an action for the host.
    ///
    /// Queuing rather than sending directly keeps every mutation of the game on
    /// one system boundary, which is what lets input handlers stay plain
    /// functions of the board model.
    pub fn submit(&mut self, action: PlayerAction) {
        self.outbox.push(action);
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
        app.add_plugins(cardmat::CardMaterialPlugin)
            .init_state::<DuelPhase>()
            .insert_resource(self.config.clone())
            .insert_resource(settings::ClientSettings::load())
            .init_resource::<Duel>()
            // Both are written by systems that run every frame; a missing
            // resource here is a panic at the table, not a compile error.
            .init_resource::<table::SceneIndex>()
            .init_resource::<table::CameraRig>()
            .init_resource::<table::ShownRig>()
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
                    table::sync_scene,
                    table::sync_zones,
                    table::glide,
                    table::apply_camera_rig,
                    hud::sync_overlay,
                    hud::apply_hand_scroll,
                    hud::animate_overlay,
                    textures::drive_preloads,
                    textures::note_failed_loads,
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
                let seat = duel.seat().unwrap_or(PlayerId::new(0));
                duel.interaction = Some(Interaction::new(*pending, seat));
                // A chooser belongs to the choice it was opened under. It
                // would heal itself anyway — the options are rebuilt from the
                // current `LegalActions` — but a menu that outlives its
                // question is a menu a player has to dismiss.
                duel.ability_menu = None;
                rebuild_board(&mut duel);
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
    let layout = TableLayout::new(&seats, 16.0 / 9.0, duel.focus);
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
        },
        pod_width,
    ));
    duel.layout = Some(layout);
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
