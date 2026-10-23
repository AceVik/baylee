//! The end of a game as one seat reads it: the verdict, the outcome, why
//! it ended and who lost how.

#[allow(clippy::wildcard_imports)] // the interaction's own vocabulary
use super::*;

/// The line a finished game gets, as this seat reads it.
///
/// [`outcome`] under a language. The split is not decoration: a sound plays
/// at the end of a game too, in no language at all.
#[must_use]
pub fn verdict(lang: Lang, result: &GameResult, seat: PlayerId, team: Option<u8>) -> String {
    match outcome(result, seat, team) {
        Outcome::Draw => Phrase::TheGameIsADraw.text(lang).to_string(),
        Outcome::YouWon => Phrase::YouWon.text(lang).to_string(),
        Outcome::YouLost => Phrase::YouLost.text(lang).to_string(),
        Outcome::YourTeamWon(n) => Phrase::YourTeamWon.fill(lang, &[&n.to_string()]),
        Outcome::TheirTeamWon(n) => Phrase::TheirTeamWon.fill(lang, &[&n.to_string()]),
    }
}

/// How a finished game reads from one chair, before it is put into words.
///
/// [`verdict`] is this plus a language, and it was this and nothing else
/// until something other than the sheet wanted the same answer: a sound at
/// the end of a game is the same fact and must not be a second reading of
/// `GameResult`, or the day a team's win starts counting differently the two
/// would disagree about who lost.
///
/// The five arms are the five *sentences*, which is why a team's number is in
/// here at all — the sheet has to name the team. Anything that only wants to
/// know which way it went asks [`Outcome::won`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Outcome {
    /// Nobody won.
    Draw,
    /// This seat won, on its own.
    YouWon,
    /// Somebody else won, on their own.
    YouLost,
    /// This seat's team won, and which.
    YourTeamWon(u8),
    /// Another team won, and which.
    TheirTeamWon(u8),
}

impl Outcome {
    /// Which way it went, `None` for a draw.
    #[must_use]
    pub fn won(self) -> Option<bool> {
        match self {
            Self::Draw => None,
            Self::YouWon | Self::YourTeamWon(_) => Some(true),
            Self::YouLost | Self::TheirTeamWon(_) => Some(false),
        }
    }
}

/// Reads a [`GameResult`] from one chair.
///
/// Takes the seat's own team rather than the whole roster because that is the
/// only thing about the table it needs: a `Victor::Team` is answered by one
/// comparison, whether the winning team is yours. A seat with no team can
/// only be its own winner, which is every seat in a game with no teams in it.
///
/// It sits here rather than in the renderer for the reason every other line
/// does — who won is a decision about the game, and the shell only draws it.
#[must_use]
pub fn outcome(result: &GameResult, seat: PlayerId, team: Option<u8>) -> Outcome {
    match result.winner {
        None => Outcome::Draw,
        Some(Victor::Player(winner)) => {
            if winner == seat {
                Outcome::YouWon
            } else {
                Outcome::YouLost
            }
        }
        Some(Victor::Team(won)) => {
            if team == Some(won) {
                Outcome::YourTeamWon(won)
            } else {
                Outcome::TheirTeamWon(won)
            }
        }
    }
}

/// The line under the verdict: *how* the game was decided.
///
/// `None` for a draw, and that refusal is the point. The four reasons are a
/// rule apiece, and three of them say something the verdict does not — a
/// table that emptied, a team that outlasted another, a card that declared a
/// winner. [`EndReason::Draw`] says only "nobody won", which is the verdict's
/// own sentence written a second time, and a screen that repeats itself
/// teaches a player to stop reading the second line.
///
/// It says nothing about *this seat*, deliberately. The same words are read
/// by the winner and by everyone who lost, so "every opponent has left" is
/// true from exactly one chair at the table and false from the others.
///
/// What a player actually wants after a loss — zero life, an empty library,
/// ten poison — is not here because it is one seat's and not the table's: the
/// view carries it per seat as `SeatView::loss` (#83), and [`loss_lines`]
/// says it. Inventing it from the life totals would be the client deciding a
/// rules fact.
#[must_use]
pub fn ending_reason(lang: Lang, result: &GameResult) -> Option<String> {
    let phrase = match result.reason {
        EndReason::LastPlayerStanding => Phrase::EndedLastPlayer,
        EndReason::LastTeamStanding => Phrase::EndedLastTeam,
        EndReason::EffectWin => Phrase::EndedByEffect,
        EndReason::Draw => return None,
    };
    Some(phrase.text(lang).to_string())
}

/// The sentence for a seat that lost for `cause`: the reading seat's, then
/// any other seat's, whose `{0}` is its name.
///
/// One table for the end screen ([`loss_lines`]) and the game log
/// (`crate::gamelog`), so a loss is told in the same words in both.
#[must_use]
pub const fn loss_phrases(cause: LossCause) -> (Phrase, Phrase) {
    match cause {
        LossCause::Life => (Phrase::LostLifeYou, Phrase::LostLifeOther),
        LossCause::EmptyDraw => (Phrase::LostEmptyDrawYou, Phrase::LostEmptyDrawOther),
        LossCause::Poison => (Phrase::LostPoisonYou, Phrase::LostPoisonOther),
        LossCause::CommanderDamage => (
            Phrase::LostCommanderDamageYou,
            Phrase::LostCommanderDamageOther,
        ),
        LossCause::Conceded => (Phrase::LostConcededYou, Phrase::LostConcededOther),
        LossCause::Effect => (Phrase::LostEffectYou, Phrase::LostEffectOther),
    }
}

/// Why one seat lost, and who answered its last decision, as the lines the
/// end screen writes under the verdict (#83).
///
/// `name` is `None` for the reading seat, which is told in the second
/// person, and the seat's name for any other. Nothing for a seat still in
/// the game: the house answering a decision of a seat that plays on is not
/// what this screen is for.
///
/// The house line follows the loss only where the view records one
/// (`SeatView::house_answered`), and together they are a game lost to the
/// clock: `loss.is_some() && house_answered == Some(HouseAnswer::Clock)`,
/// the reading `docs/protocol.md` §"Why a seat lost, and who answered for
/// it (#83)" gives. The field freezes at the loss, so it is the last
/// decision *before* the seat went out.
#[must_use]
pub fn loss_lines(lang: Lang, seat: &SeatView, name: Option<&str>) -> Vec<String> {
    let Some(loss) = seat.loss else {
        return Vec::new();
    };
    let (you, other) = loss_phrases(loss);
    let house = seat.house_answered.map(|answer| match answer {
        HouseAnswer::Clock => (Phrase::HouseClockYou, Phrase::HouseClockOther),
        HouseAnswer::StandIn => (Phrase::HouseStandInYou, Phrase::HouseStandInOther),
    });
    std::iter::once((you, other))
        .chain(house)
        .map(|(you, other)| match name {
            None => you.text(lang).to_string(),
            Some(name) => other.fill(lang, &[name]),
        })
        .collect()
}

/// [`loss_lines`] for every seat that is out: the reading seat's first,
/// then the others' in seat order, each named as the rest of the interface
/// names it ([`seat_name`]).
///
/// Every seat's and not only the reader's, because a loss is public (a seat
/// going out is announced at a real table, and so is why), and the winner
/// of a duel has as much reason to read "Bob conceded" as the loser has to
/// read their own line.
#[must_use]
pub fn table_losses(
    lang: Lang,
    view: &PlayerView,
    statics: Option<&GameStatic>,
    reader: PlayerId,
) -> Vec<String> {
    let own = view
        .seat(reader)
        .map(|seat| loss_lines(lang, seat, None))
        .unwrap_or_default();
    let others = view
        .seats
        .iter()
        .filter(|seat| seat.player != reader)
        .flat_map(|seat| {
            let name = seat_name(lang, statics, seat.player);
            loss_lines(lang, seat, Some(&name))
        });
    own.into_iter().chain(others).collect()
}
