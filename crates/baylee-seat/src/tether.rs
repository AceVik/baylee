//! Stdin held by the program that started the bridge (`--tethered`).
use baylee_client_core::llmseat::seating::{LIVE_CHANGES, ORDER_BYTES};
use baylee_client_core::say;
use tokio::io::{AsyncBufRead, AsyncBufReadExt as _};

/// Bounds credentials before allocating or parsing their first stdin line.
pub const FIRST_LINE_BYTES: usize = 16384;
use tokio::sync::{mpsc, oneshot};

/// What the program that started the bridge hands it on stdin.
pub struct Held {
    /// Fires when stdin closes: the program let go.
    pub let_go: oneshot::Receiver<()>,
    /// Each later line, an order.
    pub orders: mpsc::UnboundedReceiver<String>,
    /// The first line, when the bridge was told credentials come
    /// there (`--chair-ticket` or `--host-chair`): handed over as read and to
    /// nothing else, never an order, never printed.
    pub ticket: Option<oneshot::Receiver<String>>,
}

/// Reads stdin until it closes: first the credentials when `ticket`
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
        let mut input = tokio::io::BufReader::new(tokio::io::stdin());
        if let Some(ticket_in) = ticket_in {
            // Taken whatever it is: an empty or closed stdin hands over
            // an empty line, which is refused as no ticket.
            let Ok(first) = first_line(&mut input).await else {
                let _ = ticket_in.send(String::new());
                let _ = gone.send(());
                return;
            };
            let _ = ticket_in.send(first);
        }
        let mut lines = input.lines();
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

/// Reads exactly the credential line, preserving buffered tether bytes.
async fn first_line(reader: &mut (impl AsyncBufRead + Unpin)) -> std::io::Result<String> {
    let mut bytes = Vec::new();
    loop {
        let available = reader.fill_buf().await?;
        if available.is_empty() {
            break;
        }
        let end = available.iter().position(|b| *b == b'\n').map(|n| n + 1);
        let count = end.unwrap_or(available.len());
        if bytes.len() + count > FIRST_LINE_BYTES {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "stdin credential line too long",
            ));
        }
        bytes.extend_from_slice(&available[..count]);
        reader.consume(count);
        if end.is_some() {
            break;
        }
    }
    String::from_utf8(bytes).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid stdin credential encoding",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn first_line_keeps_buffered_commands_and_rejects_oversize() {
        let mut input = tokio::io::BufReader::new(&b"{\"credentials\":true}\nnext-order\n"[..]);
        assert_eq!(
            first_line(&mut input).await.unwrap(),
            "{\"credentials\":true}\n"
        );
        assert_eq!(
            input.lines().next_line().await.unwrap().unwrap(),
            "next-order"
        );
        let data = vec![b'x'; FIRST_LINE_BYTES + 1];
        assert!(
            first_line(&mut tokio::io::BufReader::new(data.as_slice()))
                .await
                .is_err()
        );
    }
}
