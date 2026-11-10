//! The house as a mind: `HeuristicAgent::act`, on the view the house is
//! allowed to see.
//!
//! Two jobs. It is a mind in its own right (`--mind house`), which is how the
//! bridge is measured and tried before any model is attached; and it is the
//! bridge's fallback, the answer given when a mind is too slow, refused
//! twice, or down.
//!
//! Either way it is the house, and it answers as the table's own house chair
//! does: from the view a house chair may see, the decision context the
//! table's note names (`Request::note`, rebuilt by `baylee_ai::noted`), and
//! the report of its own deck a fair chair is handed, built from the deck
//! this seat brought (`HeuristicAgent::act_with_own_deck`). The house's
//! rules hold: it never reads the log, and it never reads anything a clock
//! made. A socket view carries both (a player is told the log and their
//! clock), so the view is stripped here as `Session::agent_view` builds one
//! for a house chair (`house_view`). The request's
//! [`Request::log`](crate::Request::log) is not read at all.

use crate::mind::{Answer, DeckList, Disclosure, GameContext, Mind, Request, Thinking};
use baylee_ai::intelligence::DeckIntel;

use baylee_ai::{AIProfile, HeuristicAgent, policy_seed};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::DecisionNote;
use baylee_view::PlayerView;
use std::time::Instant;

/// The house heuristic at one level.
#[derive(Clone, Debug)]
pub struct HouseMind {
    profile: AIProfile,
}

impl Default for HouseMind {
    fn default() -> Self {
        Self::new(AIProfile::default())
    }
}

impl HouseMind {
    /// The house at this profile: its level.
    #[must_use]
    pub const fn new(profile: AIProfile) -> Self {
        Self { profile }
    }

    /// The house at a level the lobby names (`steady`, `sharp`, …) or a
    /// frozen reference (`sharp-2026-11`); `None` for a name it does not
    /// know or a level a seat cannot play ([`level`]).
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        level(name).ok().map(Self::new)
    }
}

/// The level `name` names, as a seat plays it.
///
/// A seat answers from what its socket is sent, which is its own view and
/// nothing hidden, so a level that peeks (`seer`) is one it cannot play:
/// it would sit down under that name and play as `expert`. It is refused in
/// one sentence rather than played under a name that does not tell the
/// truth.
///
/// # Errors
/// The sentence, for a name no level has or for a level that peeks.
pub fn level(name: &str) -> Result<AIProfile, String> {
    let profile =
        AIProfile::resolve(name).ok_or_else(|| format!("no house level called «{name}»"))?;
    if profile.peeks() {
        return Err(format!(
            "«{name}» reads hidden cards, which only a table's own house can; a seat \
             plays from its own view, so choose «expert»"
        ));
    }
    Ok(profile)
}

impl HouseMind {
    /// The house's answer, synchronously, with no note.
    #[must_use]
    pub fn answer(
        &self,
        context: &GameContext,
        view: &PlayerView,
        pending: &Pending,
    ) -> PlayerAction {
        self.answer_noted(context, view, pending, None)
    }

    /// The house's answer, synchronously, as a house chair at the table
    /// gives it: from the view the house may see, the decision context the
    /// table's note names (`baylee_ai::noted`), and the report of its own
    /// deck a fair chair is handed, built from the deck this seat brought.
    ///
    /// Seeded as the host seeds a house chair at a named table
    /// (`policy_seed(game, seat)`), and told the sides, so a partner is
    /// never taken for an enemy. A seat that knows no deck (a bare test
    /// context) answers without the report rather than from an empty list.
    #[must_use]
    pub fn answer_noted(
        &self,
        context: &GameContext,
        view: &PlayerView,
        pending: &Pending,
        note: Option<&DecisionNote>,
    ) -> PlayerAction {
        let agent = HeuristicAgent::new(self.profile)
            .with_teams(context.teams.clone())
            .with_seed(policy_seed(&context.game_id, context.seat.get()));
        let deck = own_deck(&context.deck);
        baylee_ai::noted::answer_as_seated(&agent, view.clone(), pending, note, deck.as_ref())
    }
}

/// The deck this seat brought, analysed as the host analyses a submitted
/// list (`DeckIntel::new`); `None` for a seat that knows none.
fn own_deck(deck: &DeckList) -> Option<DeckIntel> {
    let cards = |list: &[crate::mind::DeckCard]| -> Vec<_> {
        list.iter()
            .flat_map(|entry| {
                std::iter::repeat_n(entry.card, usize::try_from(entry.count).unwrap_or(0))
            })
            .collect()
    };
    let (main, commanders) = (cards(&deck.main), cards(&deck.commanders));
    (!main.is_empty() || !commanders.is_empty()).then(|| DeckIntel::new(main, commanders))
}

impl Mind for HouseMind {
    fn decide(&self, request: Request) -> Thinking<'_> {
        let house = self.clone();
        Box::pin(async move {
            // Off the async threads: a sharp profile searches, and a search
            // on a socket's thread is a socket that stops reading.
            let answered = tokio::task::spawn_blocking(move || {
                let started = Instant::now();
                let action = house.answer_noted(
                    &request.context,
                    &request.view,
                    &request.pending,
                    request.note.as_ref(),
                );
                Answer::new(action).took(started.elapsed())
            })
            .await;
            answered.map_err(|e| crate::MindError::Unavailable(format!("the house failed: {e}")))
        })
    }

    fn disclosure(&self) -> Disclosure {
        Disclosure::House
    }
}

/// The view the house answers from, made from a seat's socket view
/// (`baylee_ai::noted::seated_view`, which `HouseMind` answers through).
///
/// The fields `Session::agent_view` leaves out of a house chair's view,
/// left out again: the clock's remainder (wall time is not an input to a
/// house decision, #87), what the seat's standing policies answered (told to
/// the player, not to a decision), and who asked whom for a hand. The hands
/// teammates show this seat stay where its level plays as a team, as the
/// table's own house reads them (`AIProfile::TEAM_PLAY`); the frozen
/// reference reads none.
#[must_use]
pub fn house_view(view: PlayerView, profile: AIProfile) -> PlayerView {
    baylee_ai::noted::seated_view(view, profile)
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::test_support::ViewBuilder;
    use baylee_core::ids::PlayerId;
    /// A seat plays every fair level and every frozen reference, and refuses
    /// the level that peeks: its socket is sent nothing hidden, so it would
    /// play `expert` under the `seer`'s name.
    #[test]
    fn a_seat_plays_every_fair_level_and_refuses_the_one_that_peeks() {
        for (name, profile) in AIProfile::NAMED.iter().chain(&AIProfile::REFERENCES) {
            match level(name) {
                Ok(played) => {
                    assert!(!profile.peeks(), "{name} peeks and was taken");
                    assert_eq!(played, *profile);
                    assert!(HouseMind::named(name).is_some());
                }
                Err(why) => {
                    assert!(profile.peeks(), "{name} was refused: {why}");
                    assert!(why.contains("expert"), "{why}");
                    assert!(HouseMind::named(name).is_none());
                }
            }
        }
        assert!(level("seer").is_err());
        assert!(level("oracle").is_err(), "no such level");
    }

    /// A level that plays as a team keeps the hands a teammate shows the
    /// seat, as the table's own house chair reads them; the frozen reference
    /// keeps only what a rule reveals to everyone. Neither keeps who asked
    /// whom.
    #[test]
    fn house_keeps_shown_hands_where_it_plays_as_a_team() {
        let mut view = ViewBuilder::new(3).build();
        for (player, public) in [(1, true), (2, false)] {
            view.shared_hands.push(baylee_view::SharedHand {
                player: PlayerId::new(player),
                public,
                cards: ViewBuilder::new(3)
                    .with_hand(vec![("Forest", 1, u32::from(player))])
                    .build()
                    .hand,
            });
        }
        view.hand_requests = [PlayerId::new(2)].into_iter().collect();
        let live = house_view(view.clone(), AIProfile::SHARP);
        assert_eq!(live.shared_hands.len(), 2);
        assert!(live.hand_requests.is_empty());
        let frozen = house_view(view, AIProfile::SHARP_2026_11);
        assert_eq!(frozen.shared_hands.len(), 1);
        assert_eq!(frozen.shared_hands[0].player, PlayerId::new(1));
    }
}
