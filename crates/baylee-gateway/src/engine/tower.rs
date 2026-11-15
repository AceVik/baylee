//! Private native Tower routing; no rules, rewards or legacy replay writes here.
use super::{
    Envelope, Incoming, LobbyState, Shared, Uuid, WebSocket, auth, end_game, execution, mpsc,
    next_envelope, outbox_of, send_queued, v1,
};
use baylee_tower::v2::verification::Resources;
use baylee_tower_api::{
    engine::{Assembly, Control, Setup},
    host::{Prefix, Request, Response as HostResponse},
};
use std::sync::Arc;

type Refused = Box<dyn std::error::Error + Send + Sync>;

async fn send(socket: &mut WebSocket, envelope: &Envelope) -> Result<(), Refused> {
    super::send(socket, envelope)
        .await
        .map_err(|()| "Tower engine transport closed".into())
}

pub(super) async fn run(
    state: &Shared,
    mut socket: WebSocket,
    hello: &v1::EngineHello,
    authority: Arc<crate::tower::Tower>,
    game: Arc<crate::tower::Game>,
) {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let accepted = {
        let mut lobby = state.lobby.lock();
        lobby.games.get_mut(&hello.game_id).is_some_and(|table| {
            let authorized = table
                .engine_token_hash
                .as_ref()
                .is_some_and(|hash| auth::ct_eq(hash, &auth::token_hash(&hello.token)));
            if !authorized
                || table.state != LobbyState::Playing
                || table.engine.is_some()
                || hello.protocol_version != baylee_protocol::PROTOCOL_VERSION
                || !execution::matches(table.execution.as_ref(), hello)
            {
                return false;
            }
            table.engine = Some(tx);
            table.engine_pid = Some(hello.pid).filter(|pid| *pid != 0 && table.engine_local);
            true
        })
    };
    // An unproved socket must not stop the legitimate owner of this reservation.
    if !accepted {
        return;
    }
    let result = attached(
        state,
        &mut socket,
        &mut rx,
        &hello.game_id,
        &authority,
        &game,
    )
    .await;
    if result.is_err() {
        tracing::warn!(
            game_id = hello.game_id,
            "Tower execution interrupted; durable state retained"
        );
    }
    end_game(state, &hello.game_id);
}

async fn attached(
    state: &Shared,
    socket: &mut WebSocket,
    rx: &mut mpsc::UnboundedReceiver<Envelope>,
    id: &str,
    authority: &crate::tower::Tower,
    game: &crate::tower::Game,
) -> Result<(), Refused> {
    let names: [String; 2] = crate::seat_names(state, id)
        .await
        .try_into()
        .map_err(|_| "Tower roster refused")?;
    let claimed = &game.claimed;
    let setup = Setup {
        game: Uuid::parse_str(id)?,
        package: claimed.package.clone(),
        catalog: claimed.catalog.clone(),
        fence: claimed.lease.fence,
        prefix: claimed.prefix,
        byte_limit: claimed.byte_limit,
        names,
        resources: Resources {
            bytes: claimed.byte_limit,
            ..Resources::default()
        },
    };
    let digest = setup.package.digest(&setup.catalog)?;
    send(
        socket,
        &Control::Start {
            setup: Box::new(setup),
        }
        .envelope(Vec::new())?,
    )
    .await?;
    {
        let lobby = state.lobby.lock();
        if let Some(table) = lobby.games.get(id) {
            table.ready.send_replace(true);
        }
    }
    let mut session = Session {
        authority,
        game,
        assembly: Assembly::new(claimed.lease, digest, claimed.byte_limit, claimed.prefix)?,
        digest,
        read: Prefix::default(),
        sealed: false,
    };
    let mut outboxes = Vec::new();
    loop {
        tokio::select! {
            outbound = rx.recv() => {
                let outbound = outbound.ok_or("Tower game closed")?;
                send_queued(socket, &outbound, rx).await.map_err(|()| "Tower engine transport closed")?;
            }
            frame = socket.recv() => {
                match next_envelope(frame) {
                    Incoming::Msg(v1::envelope::Msg::TowerFrame(frame)) => {
                        let (control, bytes) = Control::decode(frame)?;
                        if session.control(socket, control, bytes).await? { return Ok(()); }
                    }
                    Incoming::Msg(v1::envelope::Msg::SeatFrame(frame)) if frame.seat == 0 => {
                        if let Some(outbox) = outbox_of(state, id, &mut outboxes, 0) { let _ = outbox.send(frame.envelope); }
                    }
                    Incoming::Msg(v1::envelope::Msg::SeatParticipationEnded(ended)) if session.sealed => {
                        let mut lobby = state.lobby.lock();
                        if let Some(table) = lobby.games.get_mut(id) { table.end_participation(&ended.seats); }
                    }
                    Incoming::Msg(v1::envelope::Msg::GameEnded(ended)) if session.sealed && ended.game_id == id => {},
                    Incoming::Ignored => {},
                    _ => return Err("Tower engine direction refused".into()),
                }
            }
        }
    }
}

struct Session<'a> {
    authority: &'a crate::tower::Tower,
    game: &'a crate::tower::Game,
    assembly: Assembly,
    digest: [u8; 32],
    read: Prefix,
    sealed: bool,
}

impl Session<'_> {
    async fn control(
        &mut self,
        socket: &mut WebSocket,
        control: Control,
        bytes: Vec<u8>,
    ) -> Result<bool, Refused> {
        let claimed = &self.game.claimed;
        match control {
            Control::Read { index }
                if !self.sealed && index == self.read.chunks && index < claimed.prefix.chunks =>
            {
                let HostResponse::Chunk { chunk } = self
                    .authority
                    .client
                    .control(&Request::Read {
                        lease: claimed.lease,
                        index,
                    })
                    .await?
                else {
                    return Err("Tower record unavailable".into());
                };
                let expected = self
                    .read
                    .extended(self.digest, claimed.byte_limit, &chunk.data)?;
                if chunk.previous != self.read
                    || chunk.head != expected
                    || (index + 1 == claimed.prefix.chunks && expected != claimed.prefix)
                {
                    return Err("Tower retained prefix refused".into());
                }
                self.read = expected;
                send(socket, &Control::Recovery { index }.envelope(chunk.data)?).await?;
            }
            Control::Append { .. } if !self.sealed && self.read == claimed.prefix => {
                if let Some(batch) = self.assembly.push(&control, bytes)? {
                    let acknowledgement = self.authority.client.append(&batch).await?;
                    self.assembly.acknowledge(acknowledgement)?;
                    send(
                        socket,
                        &Control::Acknowledged { acknowledgement }.envelope(Vec::new())?,
                    )
                    .await?;
                }
            }
            Control::Seal { expected }
                if !self.sealed
                    && self.read == claimed.prefix
                    && self.assembly.can_seal(expected) =>
            {
                // Stop renewal before sealing: an in-flight renew may observe
                // Verifying and must not race the final seal/view publication.
                self.game.sealed();
                let HostResponse::Acknowledged { acknowledgement } = self
                    .authority
                    .client
                    .control(&Request::Seal {
                        lease: claimed.lease,
                        expected,
                    })
                    .await?
                else {
                    return Err("Tower seal refused".into());
                };
                self.sealed = true;
                send(
                    socket,
                    &Control::Sealed { acknowledgement }.envelope(Vec::new())?,
                )
                .await?;
            }
            Control::Finished {} if self.sealed => return Ok(true),
            _ => return Err("Tower execution sequence refused".into()),
        }
        Ok(false)
    }
}
