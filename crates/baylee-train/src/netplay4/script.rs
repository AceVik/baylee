//! A cast or an activation as one answer: the planner's taps, then the
//! action they pay for.
//!
//! At priority a spell is on the engine's offer only with its mana floating,
//! so "cast this" is a chain that starts at one of several equivalent taps.
//! A [`Script4`] is that chain held as one thing: `eval4 --menu-payable`
//! follows one for a spell the net chose off the wider menu
//! ([`super::menu`]), and a search with macro candidates tries one per
//! payable spell or ability ([`super::search`]).

use std::collections::VecDeque;

use baylee_client_core::manaplan::{Plan, Step, Tap};
use baylee_core::mana::ManaColor;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::{Phase, PlayerView, Step as TurnStep};

/// A planned cast or activation on its way: the taps still to make and the
/// action they pay for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Script4 {
    steps: VecDeque<Step>,
    then: PlayerAction,
    color: Option<ManaColor>,
    second: Option<ManaColor>,
    at: (u32, Phase, TurnStep),
}

/// What a script says to one of its seat's questions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScriptAnswer4 {
    /// The script's next tap; it goes on.
    Tap(PlayerAction),
    /// The colour a tap just made asks for; it goes on.
    Color(PlayerAction),
    /// Every tap is made and the table offers the action: the script's last
    /// answer.
    Then(PlayerAction),
    /// The table would not take the next tap, or the moment has passed: the
    /// script is over and the question is the player's.
    Broken,
    /// Every tap is made and the table does not offer the action: the script
    /// is over and the question is the player's.
    NotOffered,
    /// Not a question of the script's; it goes on.
    Other,
}

impl ScriptAnswer4 {
    /// The answer to give, when the script gives one.
    #[must_use]
    pub const fn action(&self) -> Option<&PlayerAction> {
        match self {
            Self::Tap(action) | Self::Color(action) | Self::Then(action) => Some(action),
            Self::Broken | Self::NotOffered | Self::Other => None,
        }
    }

    /// Whether the script is over after this answer.
    #[must_use]
    pub const fn ends(&self) -> bool {
        matches!(self, Self::Then(_) | Self::Broken | Self::NotOffered)
    }
}

/// The action that makes one tap of a plan.
const fn tap(step: &Step) -> PlayerAction {
    match step.tap {
        Tap::Intrinsic => PlayerAction::ActivateManaAbility {
            source: step.source,
        },
        Tap::Ability(ability_index) => PlayerAction::ActivateAbility {
            source: step.source,
            ability_index,
        },
    }
}

/// Whether the table offers `then` at `pending` with its mana floating.
fn offered(pending: &Pending, then: &PlayerAction) -> bool {
    let Pending::Priority { legal, .. } = pending else {
        return false;
    };
    match then {
        PlayerAction::CastSpell { card } => legal.castable.contains(card),
        PlayerAction::ActivateAbility {
            source,
            ability_index,
        } => legal.abilities.contains(&(*source, *ability_index)),
        _ => false,
    }
}

impl Script4 {
    /// The script that makes `plan`'s taps at `view`'s moment and then
    /// answers `then`, with its first tap; `None` for a plan with nothing
    /// to tap, which is no chain.
    #[must_use]
    pub fn begin(
        view: &PlayerView,
        plan: &Plan,
        then: PlayerAction,
    ) -> Option<(PlayerAction, Self)> {
        let mut steps: VecDeque<Step> = plan.steps.iter().copied().collect();
        let first = steps.pop_front()?;
        let script = Self {
            steps,
            then,
            color: first.color,
            second: first.second,
            at: (view.turn, view.phase, view.step),
        };
        Some((tap(&first), script))
    }

    /// The action the taps pay for.
    #[must_use]
    pub const fn then(&self) -> &PlayerAction {
        &self.then
    }

    /// The script's answer to its seat's question `pending` (`view` is the
    /// seat's view of it). Any question but a colour a tap asks for and the
    /// seat's priority stays the player's.
    pub fn next(&mut self, view: &PlayerView, pending: &Pending) -> ScriptAnswer4 {
        let same = (view.turn, view.phase, view.step) == self.at;
        match pending {
            Pending::ChooseColor { options, .. } if same => {
                let Some(color) = self.color.take().filter(|color| options.contains(color)) else {
                    return ScriptAnswer4::Other;
                };
                self.color = self.second.take();
                ScriptAnswer4::Color(PlayerAction::ChooseColor(color))
            }
            Pending::Priority { .. } if same && view.owed.is_none() => {
                if let Some(step) = self.steps.front().copied() {
                    let action = tap(&step);
                    if pending.answer_fault(&action).is_some() {
                        return ScriptAnswer4::Broken;
                    }
                    self.steps.pop_front();
                    self.color = step.color;
                    self.second = step.second;
                    return ScriptAnswer4::Tap(action);
                }
                if offered(pending, &self.then) {
                    ScriptAnswer4::Then(self.then.clone())
                } else {
                    ScriptAnswer4::NotOffered
                }
            }
            Pending::Priority { .. } => ScriptAnswer4::Broken,
            _ => ScriptAnswer4::Other,
        }
    }
}
