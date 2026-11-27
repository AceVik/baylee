//! The authenticated participant's local presentation of a ranked duel. The
//! original `GameStatic` stays untouched for gameplay, diagnostics and records.
//!
//! Central says two things of a ranked game's seats that the host does not:
//! whose published handle each is, and which is an AI player and what kind
//! (`Roster`). Both are drawn from here only, and never written into the
//! roster the game itself reads.
use crate::Duel;
use baylee_client_core::aiplayers::AiBadge;
use baylee_core::ids::PlayerId;
use baylee_view::GameStatic;

type Handles = [Option<String>; 2];
/// Central's AI badges for the two seats, in seat order.
pub(crate) type Badges = [Option<AiBadge>; 2];

/// What Central says of one ranked game's two seats.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Roster {
    /// The game it is about.
    pub(crate) game: String,
    /// Each seat's published handle.
    pub(crate) handles: Handles,
    /// Each seat's AI badge, where it is an AI.
    pub(crate) ai: Badges,
}

#[derive(Default)]
pub(super) struct Names {
    source: Option<Roster>,
    display: Option<GameStatic>,
    revision: u64,
}

impl Names {
    /// Only host static messages and a new Central answer refresh this snapshot.
    /// All UI frames borrow it. Reports and replays continue to read the raw one.
    pub(super) fn refresh(&mut self, raw: &GameStatic) {
        let Some(roster) = self.source.as_ref().filter(|r| r.game == raw.game_id) else {
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
        // An AI player is a full player with its own handle, named as a
        // person is; its badge's mark stands beside the name, never in it.
        for s in &mut shown.seats {
            if let Some(handle) = &roster.handles[usize::from(s.player.get())] {
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

    /// The AI badge Central gave `player`'s seat in this ranked game, for
    /// the reader's own table; none for a person, a watcher or a replay.
    pub(crate) fn ai_badge(&self, player: PlayerId) -> Option<&AiBadge> {
        let raw = self.statics.as_ref()?;
        if self.watching || self.replay.is_some() {
            return None;
        }
        self.participant_names.display.as_ref()?;
        let roster = self
            .participant_names
            .source
            .as_ref()
            .filter(|r| r.game == raw.game_id)?;
        roster.ai.get(usize::from(player.get()))?.as_ref()
    }

    pub(crate) fn roster_revision(&self) -> u64 {
        self.participant_names.revision
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn participant_labels(&mut self, source: Option<Roster>) {
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

    fn named(game: &str) -> Roster {
        Roster {
            game: game.into(),
            handles: [Some("Laurel#1234".into()), None],
            ai: [None, None],
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
        duel.participant_labels(Some(named(&game)));
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
        duel.participant_labels(Some(named(&game)));
        duel.rewind();
        assert_eq!(duel.display_statics().unwrap().seat_name(player), "Seat 0");
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod badge_tests {
    use super::*;
    use baylee_client_core::aiplayers::AiKind;
    use baylee_view::SeatIdentity;

    fn statics(game: &str) -> GameStatic {
        GameStatic {
            view_version: baylee_view::VIEW_VERSION,
            game_id: game.into(),
            your_seat: PlayerId::new(0),
            seats: [0u8, 1]
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

    fn house() -> AiBadge {
        AiBadge {
            kind: AiKind::House,
            label: "Haus-KI · Expert".into(),
        }
    }

    #[test]
    fn an_ai_seat_wears_its_badge_only_at_its_own_game_and_never_on_the_wire() {
        let game = uuid::Uuid::now_v7().to_string();
        let mut duel = Duel {
            statics: Some(statics(&game)),
            ..Duel::default()
        };
        let before = duel.roster_revision();
        duel.participant_labels(Some(Roster {
            game: game.clone(),
            handles: [None, Some("KI-Haus-Expert#0002".into())],
            ai: [None, Some(house())],
        }));
        assert_ne!(duel.roster_revision(), before, "the bars redraw for it");
        assert_eq!(duel.ai_badge(PlayerId::new(1)), Some(&house()));
        assert_eq!(duel.ai_badge(PlayerId::new(0)), None);
        // The AI is named by its own handle, as a person is; the badge
        // stands beside it.
        assert_eq!(
            duel.display_statics().unwrap().seat_name(PlayerId::new(1)),
            "KI-Haus-Expert#0002"
        );
        // The roster the game reads is untouched: the seat stays a remote
        // seat, and no badge is written into what a report or record keeps.
        let raw = duel.statics.as_ref().unwrap();
        assert!(!raw.seats.iter().any(|s| s.is_ai));
        assert!(!serde_json::to_string(raw).unwrap().contains("Haus-KI"));
        // The same answer again changes nothing.
        let settled = duel.roster_revision();
        duel.participant_labels(Some(Roster {
            game: game.clone(),
            handles: [None, Some("KI-Haus-Expert#0002".into())],
            ai: [None, Some(house())],
        }));
        assert_eq!(duel.roster_revision(), settled);
        duel.watching = true;
        assert_eq!(duel.ai_badge(PlayerId::new(1)), None);
        duel.watching = false;
        duel.statics = Some(statics(&uuid::Uuid::now_v7().to_string()));
        assert_eq!(duel.ai_badge(PlayerId::new(1)), None, "another game");
    }
}
