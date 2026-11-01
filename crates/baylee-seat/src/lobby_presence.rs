//! Session presence uses a one-use lobby ticket, never a session in a URL.
use super::{Lobby, Session};
use crate::link::{Socket, ws_base};
use anyhow::{anyhow, bail};
use baylee_client_core::wsticket::{self, Socket as TicketSocket, TicketAnswer};

impl Lobby {
    /// Opens the session's lobby presence feed with a single-use socket ticket.
    /// The caller must keep reading/ponging until it leaves the gateway.
    ///
    /// # Errors
    /// Returns a credential-free error if ticket issuance or the upgrade fails.
    pub async fn connect_lobby(&self, session: &Session) -> anyhow::Result<Socket> {
        let base = ws_base(self.base()).map_err(|_| anyhow!("invalid lobby address"))?;
        let (status, body) = self
            .raw(
                "POST",
                baylee_protocol::WS_TICKET_PATH,
                Some(session.token.clone()),
                Some(TicketSocket::Lobby.request_body().into_bytes()),
            )
            .await
            .map_err(|_| anyhow!("lobby presence ticket request failed"))?;
        let TicketAnswer::Ticket(ticket) = TicketAnswer::read(status, &body) else {
            bail!("gateway refused the lobby presence ticket");
        };
        let path = wsticket::lobby_socket_path(&ticket, "");
        let (socket, _) =
            tokio_tungstenite::connect_async_with_config(format!("{base}{path}"), None, true)
                .await
                .map_err(|_| anyhow!("lobby presence socket did not open"))?;
        Ok(socket)
    }
}
