//! Game results and win/lose evaluation.

use baylee_core::ids::PlayerId;
use serde::{Deserialize, Serialize};

/// The final result of a game.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct GameResult {
    /// Who won (`None` = draw).
    pub winner: Option<Victor>,
    /// Why the game ended.
    pub reason: EndReason,
}

/// Who a game was won by.
///
/// A team wins as a team, including when only one of its members is still
/// alive (CR 104.2b: a player who has left the game is still on the team
/// that wins), so the winner of a team game is the team and not the seat
/// that happened to survive.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Victor {
    /// One seat, playing for nobody else.
    Player(PlayerId),
    /// A team, by its index.
    Team(u8),
}

impl Victor {
    /// Whether the seat `player`, playing for `team`, is on the winning side.
    #[must_use]
    pub fn includes(self, player: PlayerId, team: Option<u8>) -> bool {
        match self {
            Self::Player(id) => id == player,
            Self::Team(t) => team == Some(t),
        }
    }
}

/// Why the game ended.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum EndReason {
    /// All opponents lost (CR 104.1).
    LastPlayerStanding,
    /// Every seat still in the game plays for one team (CR 104.2b).
    LastTeamStanding,
    /// A player won by effect (M2).
    EffectWin,
    /// All remaining players drew (e.g. mandatory loop with `CompRulesDraw`).
    Draw,
}
