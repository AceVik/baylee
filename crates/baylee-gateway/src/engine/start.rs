//! Reserve and launch exactly one compatible engine.
use super::{Envelope, LobbyState, Shared, auth, execution, lifecycle, v1};

/// Asks an agent to start the engine for a game.
///
/// Called once, when a game's seats are decided. The seat sockets do not wait
/// on this: they wait on the engine actually attaching, which is a different
/// and later thing.
///
/// # Errors
/// When no agent has room, or when the game is not one that needs an engine;
/// [`crate::admission::HELD`] while a deploy holds admission. The callers
/// mark the game playing before they call this, so a deploy that reads
/// `/health` after placing the hold counts every game this let through.
pub fn start_engine(state: &Shared, game_id: &str) -> Result<(), &'static str> {
    if state.admission.held() {
        return Err(crate::admission::HELD);
    }
    let engine_token = auth::new_token();
    let agent_id = {
        // One short reservation critical section. No await and no hidden setup data.
        // All other users release either lock before acquiring the other one.
        let mut lobby = state.lobby.lock();
        let game = lobby.games.get_mut(game_id).ok_or("no such game")?;
        if game.state != LobbyState::Playing || game.agent_id.is_some() {
            return Err("game does not need an engine");
        }
        let preset = game.preset.as_ref().ok_or("no game preset")?;
        let variant = execution::standard_variant(preset.house_rules.loop_policy);
        let mut agents = state.agents.lock();
        let (id, execution) = if let Some(required) = game
            .execution
            .as_ref()
            .filter(|identity| identity.variant == baylee_protocol::execution::Variant::Tower)
        {
            agents.pick_exact(required)
        } else {
            agents.pick_variant(variant, game.challenge_executor.as_deref())
        }
        .ok_or("no compatible engine capacity")?;
        let agent = agents.connected.get_mut(&id).ok_or("the agent went away")?;
        let order = Envelope {
            msg: Some(v1::envelope::Msg::StartEngine(v1::StartEngine {
                game_id: game_id.to_string(),
                engine_token: engine_token.clone(),
                gateway_url: state.engine_url.clone(),
                execution: execution.to_metadata(),
            })),
        };
        if agent.tx.send(order).is_err() {
            return Err("the agent went away");
        }
        agent.games.push(game_id.to_string());
        game.engine_token_hash = Some(auth::token_hash(&engine_token));
        game.agent_id = Some(id.clone());
        game.engine_local = agent.local;
        game.execution = Some(execution);
        tracing::info!(game_id, agent_id = id, agent = agent.name, "engine ordered");
        id
    };
    lifecycle::watch_start(state.clone(), game_id.to_string(), agent_id);
    Ok(())
}
