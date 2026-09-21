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
/// alive (CR 104.2c: a player who has left the game is still on the team
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
    /// All opponents lost (CR 104.2a).
    LastPlayerStanding,
    /// Every seat still in the game plays for one team (CR 104.2c).
    LastTeamStanding,
    /// A player won by effect (M2).
    EffectWin,
    /// All remaining players drew (e.g. mandatory loop with `CompRulesDraw`).
    Draw,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A team wins as a team, and the seat that happened to survive is not
    /// the winner — CR 104.2c keeps a player who has left the game on the
    /// team that wins it. So the question is asked of the *side* a seat
    /// plays for, and a seat playing for nobody can only win as itself.
    #[test]
    fn a_team_wins_for_everyone_on_it_and_a_seat_only_for_itself() {
        let me = PlayerId::new(0);
        let ally = PlayerId::new(2);
        let enemy = PlayerId::new(1);

        let team = Victor::Team(1);
        assert!(team.includes(me, Some(1)));
        assert!(
            team.includes(ally, Some(1)),
            "a teammate wins the same game, whether or not they are still in it"
        );
        assert!(!team.includes(enemy, Some(2)));
        assert!(
            !team.includes(me, None),
            "a seat on no team is on no winning team either"
        );

        let solo = Victor::Player(me);
        assert!(solo.includes(me, None));
        assert!(
            solo.includes(me, Some(1)),
            "a seat that won as itself won, whatever side it was on"
        );
        assert!(!solo.includes(enemy, None));
        assert!(
            !solo.includes(ally, Some(1)),
            "and a teammate of theirs did not — this victor is one seat"
        );
    }

    /// A draw is a result with no victor rather than a victor nobody
    /// matches, which is what lets a client ask `winner.is_none()` instead
    /// of comparing against every seat at the table.
    #[test]
    fn a_draw_has_no_winner_at_all() {
        let drawn = GameResult {
            winner: None,
            reason: EndReason::Draw,
        };
        assert!(drawn.winner.is_none());
        let won = GameResult {
            winner: Some(Victor::Player(PlayerId::new(0))),
            reason: EndReason::LastPlayerStanding,
        };
        assert_ne!(drawn, won);
        assert_eq!(
            won,
            GameResult {
                winner: Some(Victor::Player(PlayerId::new(0))),
                reason: EndReason::LastPlayerStanding,
            },
            "a result is its two fields and nothing else"
        );
    }
}
