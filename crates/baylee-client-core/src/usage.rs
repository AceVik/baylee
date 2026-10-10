//! The anonymous offline count (`docs/feedback.md` §"Offline now",
//! `docs/privacy.md`): while a game this client hosts itself runs, and the
//! player has left "Send anonymous usage count" on, it tells the feedback
//! service once a [`BEAT_SECS`] that the game is still going.
//!
//! The beat carries one field, a random value made for that one game and
//! held only in memory: no account, no device id, nothing about the game.
//! Off, or with no feedback service known, nothing is sent at all.

/// The path the feedback service counts offline games at.
pub const ALIVE_PATH: &str = "/client/alive";

/// How often a beat goes; the service's `baylee_feedback::alive::BEAT_SECS`
/// must agree (its TTL is two beats and a margin).
pub const BEAT_SECS: f64 = 60.0;

/// A random value for one offline game, as 32 hex digits: the only thing a
/// beat says. Made fresh per game and forgotten with it.
#[must_use]
pub fn game_value(random: [u8; 16]) -> String {
    random.iter().fold(String::with_capacity(32), |mut s, b| {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// The beat's body: the one field.
#[must_use]
pub fn body(game: &str) -> String {
    serde_json::json!({ "game": game }).to_string()
}

/// Where a beat goes now, or `None` when nothing is to be sent: the
/// setting off, no feedback service, or no offline game running.
#[must_use]
pub fn beat_url(enabled: bool, service: Option<&str>, offline_game: bool) -> Option<String> {
    if !enabled || !offline_game {
        return None;
    }
    service.map(|s| format!("{}{ALIVE_PATH}", s.trim_end_matches('/')))
}

/// When the next beat is due.
#[derive(Clone, Debug, Default)]
pub struct Beats {
    /// The game's value, while one runs.
    game: Option<String>,
    /// The time (seconds, any monotonic origin) of the last beat.
    last: Option<f64>,
}

impl Beats {
    /// What to send at `now`, if anything: `Some(body)` when an offline game
    /// runs, the beat is wanted and one is due. `fresh` makes a game's value
    /// when a game starts. A game ending forgets its value.
    pub fn due(
        &mut self,
        now: f64,
        wanted: bool,
        offline_game: bool,
        fresh: impl FnOnce() -> [u8; 16],
    ) -> Option<String> {
        if !offline_game {
            self.game = None;
            self.last = None;
            return None;
        }
        let game = self.game.get_or_insert_with(|| game_value(fresh())).clone();
        if !wanted {
            return None;
        }
        if self.last.is_some_and(|at| now - at < BEAT_SECS) {
            return None;
        }
        self.last = Some(now);
        Some(body(&game))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The body is exactly one field, a fresh random value, and it differs
    /// between games: no account, device or game content.
    #[test]
    fn a_beat_names_nobody() {
        let mut beats = Beats::default();
        let one = beats.due(0.0, true, true, || [1; 16]).expect("a beat");
        let value: serde_json::Value = serde_json::from_str(&one).unwrap();
        let keys: Vec<&String> = value.as_object().unwrap().keys().collect();
        assert_eq!(keys, ["game"]);
        assert_eq!(value["game"], "01".repeat(16));
        // The game ends; the next one has its own value.
        assert_eq!(beats.due(1.0, true, false, || [1; 16]), None);
        let two = beats.due(2.0, true, true, || [2; 16]).expect("a beat");
        assert_ne!(one, two);
    }

    /// Opted out, or with no service, nothing is sent.
    #[test]
    fn opting_out_sends_nothing() {
        assert_eq!(beat_url(false, Some("https://fb.example"), true), None);
        assert_eq!(beat_url(true, None, true), None);
        assert_eq!(beat_url(true, Some("https://fb.example"), false), None);
        assert_eq!(
            beat_url(true, Some("https://fb.example/"), true).as_deref(),
            Some("https://fb.example/client/alive")
        );
        let mut beats = Beats::default();
        for n in 0..10 {
            assert_eq!(
                beats.due(f64::from(n) * 100.0, false, true, || [3; 16]),
                None
            );
        }
    }

    /// One beat a minute, not one a frame.
    #[test]
    fn beats_are_a_minute_apart() {
        let mut beats = Beats::default();
        assert!(beats.due(0.0, true, true, || [4; 16]).is_some());
        assert!(beats.due(30.0, true, true, || [4; 16]).is_none());
        assert!(beats.due(60.0, true, true, || [4; 16]).is_some());
    }
}
