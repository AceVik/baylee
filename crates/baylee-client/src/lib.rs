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

#![forbid(unsafe_code)]
#![warn(missing_docs)]
// The client converts small counts (seats, cards in a lane, list indices) to
// floats for layout. All are bounded by what fits on a table.
#![allow(clippy::cast_precision_loss)]
// Bevy's system-param contract takes `Res`, `Query` and friends by value —
// they *are* the parameter, and a reference to one is not a system param at
// all. The lint cannot see that, and firing it on every system would bury the
// cases where it is right.
#![allow(clippy::needless_pass_by_value)]

pub mod host;
pub mod hud;
pub mod input;
pub mod settings;
pub mod table;
pub mod textures;

use baylee_client_core::automation::{self, AutoPilot, PhaseOrders};
use baylee_client_core::board::BoardModel;
use baylee_client_core::interaction::{CombatCandidates, Interaction};
use baylee_client_core::layout::TableLayout;
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::{GameStatic, PlayerView};
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use host::{DuelHost, HostMessage};

pub use host::LocalHost;

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
    /// Per-phase standing orders (green = take priority, red = skip).
    pub orders: PhaseOrders,
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
        app.init_state::<DuelPhase>()
            .insert_resource(self.config.clone())
            .insert_resource(settings::ClientSettings::load())
            .init_resource::<Duel>()
            // Both are written by systems that run every frame; a missing
            // resource here is a panic at the table, not a compile error.
            .init_resource::<table::SceneIndex>()
            .init_resource::<table::CameraRig>()
            .init_resource::<hud::HudRevision>()
            .add_message::<DuelCommand>()
            .add_message::<DuelReport>()
            .configure_sets(
                Update,
                (DuelSet::Sync, DuelSet::Input, DuelSet::Present).chain(),
            )
            .add_systems(Startup, (textures::setup, hud::setup_fonts))
            .add_systems(
                Update,
                (handle_commands, poll_host, run_autopilot, flush_outbox)
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
                )
                    .in_set(DuelSet::Input)
                    .run_if(in_state(DuelPhase::Playing)),
            )
            .add_systems(
                Update,
                (
                    table::sync_scene,
                    table::apply_camera_rig,
                    hud::sync_overlay,
                    hud::apply_hand_scroll,
                    hud::animate_overlay,
                )
                    .in_set(DuelSet::Present)
                    .run_if(not(in_state(DuelPhase::Closed))),
            )
            .add_systems(OnEnter(DuelPhase::Opening), table::spawn_stage)
            .add_systems(OnEnter(DuelPhase::Closed), table::despawn_stage);
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
                let candidates = combat_candidates(&duel, &pending);
                duel.interaction = Some(Interaction::new(*pending, seat, &candidates));
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
fn run_autopilot(mut duel: ResMut<Duel>) {
    let Some((phase, step, turn)) = duel.view.as_ref().map(|v| (v.phase, v.step, v.turn)) else {
        return;
    };
    if let Some(pilot) = duel.autopilot
        && pilot.reached(phase, turn)
    {
        duel.autopilot = None;
    }
    let answer = {
        let Some(interaction) = duel.interaction.as_ref() else {
            return;
        };
        automation::auto_answer(
            interaction.pending(),
            interaction.is_mine(),
            phase,
            step,
            &duel.orders,
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

    duel.board = Some(BoardModel::from_view(view, &playable, pod_width));
    duel.layout = Some(layout);
}

/// The attack and block candidates the engine's choice does not enumerate.
///
/// Derived from the board model, which already knows which creatures are
/// untapped and unsick. It is an affordance only: the engine rejects an
/// illegal declaration regardless of what the client offered.
fn combat_candidates(duel: &Duel, pending: &Pending) -> CombatCandidates {
    let (Some(view), Some(board)) = (duel.view.as_ref(), duel.board.as_ref()) else {
        return CombatCandidates::default();
    };
    let seat = view.seat;
    let mut candidates = CombatCandidates::default();

    match pending {
        Pending::ChooseAttackers { .. } => {
            candidates.defenders = view
                .seats
                .iter()
                .filter(|s| s.player != seat && !s.has_lost)
                .map(|s| s.player)
                .collect();
            if let Some(pod) = board.pod(seat) {
                for lane in &pod.lanes {
                    for group in &lane.groups {
                        if group.power.is_some()
                            && !group.status.is_tapped()
                            && !group.summoning_sick
                        {
                            candidates.attackers.extend(group.members.iter().copied());
                        }
                    }
                }
            }
        }
        Pending::ChooseBlockers { .. } => {
            candidates.attacking = view.combat.attackers.iter().map(|a| a.creature).collect();
            if let Some(pod) = board.pod(seat) {
                for lane in &pod.lanes {
                    for group in &lane.groups {
                        if group.power.is_some() && !group.status.is_tapped() {
                            candidates.blockers.extend(group.members.iter().copied());
                        }
                    }
                }
            }
        }
        _ => {}
    }
    candidates
}
