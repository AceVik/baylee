//! `/reveal {"count":3}`: stands up a reveal by another seat, for the reveal
//! sheet's checks.
//!
//! A real one needs an opponent to reveal something, which the house AI does
//! when it pleases and a harness cannot ask for. So this writes the log line
//! a host would have sent — `Revealed` by the first other seat, naming the
//! first `count` cards of this seat's own hand, whose printings the seat
//! already holds — and hands it to [`Reveals::heard`], the same door
//! `poll_host` takes. Nothing reaches the host, the log book or the game: the
//! sheet is the only thing that sees it.
//!
//! [`Reveals::heard`]: baylee_client_core::reveals::Reveals::heard

use super::field;
use crate::Duel;
use baylee_view::{LogEntry, LogEvent, LogObject, LogTail};

/// Answers one `/reveal`.
pub(super) fn stage(body: &str, duel: &mut Duel) -> String {
    let count = field(body, "count")
        .and_then(|n| n.parse::<usize>().ok())
        .unwrap_or(1)
        .clamp(1, 12);
    let Some(view) = duel.view.as_ref() else {
        return "{\"error\":\"no game\"}".to_string();
    };
    let Some(player) = view
        .seats
        .iter()
        .map(|s| s.player)
        .find(|p| *p != view.seat)
    else {
        return "{\"error\":\"no other seat\"}".to_string();
    };
    let cards: Vec<LogObject> = view
        .hand
        .iter()
        .take(count)
        .map(|card| LogObject::Known {
            id: card.id,
            card: Some(card.card),
            token: None,
            name: card.name.clone(),
        })
        .collect();
    if cards.is_empty() {
        return "{\"error\":\"no card in hand to reveal\"}".to_string();
    }
    let shown = cards.len();
    let entry = LogEntry {
        turn: view.turn,
        repeat: 1,
        at: 0,
        event: LogEvent::Revealed { player, cards },
        phase: None,
    };
    // A live tail (not from line 0), so it is heard as news.
    let tail = LogTail {
        from: u32::MAX,
        entries: vec![entry.clone()],
    };
    let view = view.clone();
    if duel.reveals.heard(&tail, &view, &[entry]) {
        format!("{{\"ok\":true,\"cards\":{shown}}}")
    } else {
        "{\"error\":\"not heard (a retold frame); try again after the next view\"}".to_string()
    }
}
