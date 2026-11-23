//! The authenticated participant's local presentation of a ranked duel. The
//! original `GameStatic` stays untouched for gameplay, diagnostics and records.
use crate::Duel;
use baylee_view::GameStatic;

type Handles = [Option<String>; 2];

#[derive(Default)]
pub(super) struct Names {
    source: Option<(String, Handles)>,
    display: Option<GameStatic>,
    revision: u64,
}

impl Names {
    /// Only host static messages and a new Central answer refresh this snapshot.
    /// All UI frames borrow it. Reports and replays continue to read the raw one.
    pub(super) fn refresh(&mut self, raw: &GameStatic) {
        let Some((_, handles)) = self
            .source
            .as_ref()
            .filter(|(game, _)| *game == raw.game_id)
        else {
            self.forget_display();
            return;
        };
        if raw.seats.len() != 2
            || ![0, 1]
                .iter()
                .all(|n| raw.seats.iter().any(|s| s.player.get() == *n))
        {
            self.forget_display();
            return;
        }
        let mut shown = raw.clone();
        for s in &mut shown.seats {
            if let Some(handle) = &handles[usize::from(s.player.get())] {
                s.display_name.clone_from(handle);
            }
        }
        self.display = Some(shown);
    }

    fn forget_display(&mut self) {
        if self.display.take().is_some() {
            self.revision = self.revision.wrapping_add(1);
        }
    }
}

impl Duel {
    /// Game state used solely for local presentation. An unbound or watching
    /// session sees the original host roster, including on replay.
    pub(crate) fn display_statics(&self) -> Option<&GameStatic> {
        let raw = self.statics.as_ref()?;
        if !self.watching
            && self.replay.is_none()
            && let Some(shown) = self
                .participant_names
                .display
                .as_ref()
                .filter(|s| s.game_id == raw.game_id)
        {
            return Some(shown);
        }
        Some(raw)
    }

    pub(crate) fn roster_revision(&self) -> u64 {
        self.participant_names.revision
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn participant_labels(&mut self, source: Option<(String, Handles)>) {
        let names = &mut self.participant_names;
        if names.source == source {
            return;
        }
        names.source = source;
        names.display = None;
        names.revision = names.revision.wrapping_add(1);
        if let Some(raw) = self.statics.as_ref() {
            names.refresh(raw);
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use baylee_core::ids::PlayerId;
    use baylee_view::SeatIdentity;

    fn roster(game: &str) -> GameStatic {
        GameStatic {
            view_version: baylee_view::VIEW_VERSION,
            game_id: game.into(),
            your_seat: PlayerId::new(1),
            seats: [1u8, 0]
                .map(|n| SeatIdentity {
                    player: PlayerId::new(n),
                    display_name: format!("Seat {n}"),
                    is_ai: false,
                    away: false,
                    team: None,
                    guest: false,
                })
                .into(),
            prints: Vec::new(),
            decision_secs: None,
            reconnect_secs: None,
            table_seed: 0,
        }
    }

    #[test]
    fn ranked_participant_names_stay_off_the_wire_and_follow_the_authenticated_game() {
        let game = uuid::Uuid::now_v7().to_string();
        let player = PlayerId::new(0);
        let mut duel = Duel {
            statics: Some(roster(&game)),
            ..Duel::default()
        };
        duel.participant_labels(Some((game.clone(), [Some("Laurel#1234".into()), None])));
        assert_eq!(
            duel.display_statics().unwrap().seat_name(player),
            "Laurel#1234"
        );
        assert_eq!(
            duel.display_statics().unwrap().seat_name(PlayerId::new(1)),
            "Seat 1"
        );
        let raw = duel.statics.as_ref().unwrap();
        assert_eq!(raw.seat_name(player), "Seat 0");
        assert!(!serde_json::to_string(raw).unwrap().contains("Laurel"));
        // A reconnect can resend GameStatic with refreshed card/away information.
        let mut refreshed = roster(&game);
        refreshed.seats[1].away = true;
        duel.participant_names.refresh(&refreshed);
        duel.statics = Some(refreshed);
        let shown = duel.display_statics().unwrap();
        assert!(shown.seats[1].away);
        assert_eq!(shown.seat_name(player), "Laurel#1234");
        duel.watching = true;
        assert_eq!(duel.display_statics().unwrap().seat_name(player), "Seat 0");
        duel.watching = false;
        duel.statics = Some(roster(&uuid::Uuid::now_v7().to_string()));
        assert_eq!(duel.display_statics().unwrap().seat_name(player), "Seat 0");
        duel.participant_labels(None);
        duel.statics = Some(roster(&game));
        assert_eq!(duel.display_statics().unwrap().seat_name(player), "Seat 0");
        duel.participant_labels(Some((game, [Some("Laurel#1234".into()), None])));
        duel.rewind();
        assert_eq!(duel.display_statics().unwrap().seat_name(player), "Seat 0");
    }
}
