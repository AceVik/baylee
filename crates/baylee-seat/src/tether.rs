//! Stdin held by the program that started the bridge (`--tethered`).
use baylee_client_core::llmseat::seating::{LIVE_CHANGES, ORDER_BYTES};
use baylee_client_core::say;
use tokio::io::AsyncBufReadExt as _;
use tokio::sync::{mpsc, oneshot};

/// What the program that started the bridge hands it on stdin.
pub struct Held {
    /// Fires when stdin closes: the program let go.
    pub let_go: oneshot::Receiver<()>,
    /// Each later line, an order.
    pub orders: mpsc::UnboundedReceiver<String>,
    /// The first line, when the bridge was told a chair ticket comes
    /// there (`--chair-ticket`): handed over as it was read and to
    /// nothing else, never an order, never printed.
    pub ticket: Option<oneshot::Receiver<String>>,
}

/// Reads stdin until it closes: first the chair ticket when `ticket`
/// says one comes, then each line an order (a debug build's only; a
/// release build reads them and does nothing with them), and its end
/// the program letting go.
pub fn hold(ticket: bool) -> Held {
    let (gone, let_go) = oneshot::channel();
    let (send, orders) = mpsc::unbounded_channel();
    let (ticket_in, ticket_out) = if ticket {
        let (tx, rx) = oneshot::channel();
        (Some(tx), Some(rx))
    } else {
        (None, None)
    };
    tokio::spawn(async move {
        let mut lines = tokio::io::BufReader::new(tokio::io::stdin()).lines();
        if let Some(ticket_in) = ticket_in {
            // Taken whatever it is: an empty or closed stdin hands over
            // an empty line, which is refused as no ticket.
            let first = lines.next_line().await.ok().flatten().unwrap_or_default();
            let _ = ticket_in.send(first);
        }
        let mut said = false;
        while let Ok(Some(line)) = lines.next_line().await {
            if line.trim().is_empty() {
                continue;
            }
            if !LIVE_CHANGES {
                if !said {
                    say!("order refused: a release build changes no mind during a game");
                    said = true;
                }
                continue;
            }
            if line.len() > ORDER_BYTES {
                say!("order refused: an order is one line of at most {ORDER_BYTES} bytes");
                continue;
            }
            let _ = send.send(line);
        }
        let _ = gone.send(());
    });
    Held {
        let_go,
        orders,
        ticket: ticket_out,
    }
}

/// Waits for the program to let go; forever when nothing holds the
/// bridge.
pub async fn let_go(held: Option<&mut oneshot::Receiver<()>>) {
    match held {
        Some(held) => {
            let _ = held.await;
        }
        None => std::future::pending().await,
    }
}
