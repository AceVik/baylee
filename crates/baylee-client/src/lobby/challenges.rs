//! Native global challenges: one account worker, bounded replies, ordinary network seats.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_client_core::challenges::{Action, State};

mod draw;
pub(super) use draw::draw;
#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub(super) use native::ask;

#[derive(Default)]
pub(crate) struct Ui {
    pub(super) model: State,
    /// The task opened here, retained until its table has closed.
    playing: Option<([u8; 16], String)>,
    /// An ended table awaiting Central's durable result delivery.
    pub(super) pending_result: bool,
    #[cfg(not(target_arch = "wasm32"))]
    session: Option<central::Handle>,
    #[cfg(not(target_arch = "wasm32"))]
    inbox: Arc<Mutex<Option<native::Finished>>>,
}

pub(super) fn install(app: &mut App) {
    #[cfg(not(target_arch = "wasm32"))]
    app.add_systems(
        Update,
        native::tick
            .after(poll)
            .before(ui)
            .run_if(in_state(DuelPhase::Closed)),
    );
    #[cfg(not(target_arch = "wasm32"))]
    app.add_systems(
        OnEnter(DuelPhase::Closed),
        native::returned.after(super::systems::came_back),
    );
    #[cfg(target_arch = "wasm32")]
    let _ = app;
}

pub(super) fn current(state: &LobbyState) -> bool {
    matches!(state.lobby.screen(), Screen::Seated(seat)
        if state.challenges.playing.as_ref().is_some_and(|(_, game)| *game == seat.game_id))
}

pub(super) fn press(action: Action, cx: super::press::Cx<'_, '_, '_, '_, '_>) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if action == Action::Play && cx.state.lobby.awaiting().is_some() {
            cx.state
                .lobby
                .tell_refusal(Phrase::ChallengeLeaveTable, &[]);
            return;
        }
        ask(cx.state, action);
        if matches!(action, Action::Select(_) | Action::Back | Action::Page(_)) {
            cx.scrolled.set(List::Table, 0.0);
        }
    }
    #[cfg(target_arch = "wasm32")]
    let _ = (action, cx);
}
