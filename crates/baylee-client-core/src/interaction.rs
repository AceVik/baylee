//! Turning a pending choice into something a player can operate, and turning
//! what they did back into a [`PlayerAction`].
//!
//! # Two independent jobs
//!
//! **Affordance.** Given a [`Pending`], say what is clickable, how many things
//! must be picked, and what the prompt reads. A renderer should never have to
//! `match` on the engine's choice taxonomy to decide whether a card glows.
//!
//! **Refusal.** Never let a player build an answer the engine will reject.
//! Every selection is checked against the options the engine actually offered,
//! and out-of-range values are unreachable rather than merely discouraged.
//!
//! The second job is a user-experience feature, not a security control: a host
//! must validate everything it receives regardless of what the client believes.
//! It exists here so that an illegal action is impossible to *express*, which
//! is a much better experience than a round trip that ends in a rejection.
//!
//! # A note on combat
//!
//! `Pending::ChooseAttackers` and `Pending::ChooseBlockers` are the two choices
//! that do **not** carry their candidate list, unlike every other variant. The
//! client therefore has to be told which creatures to offer, and the caller
//! passes that in from the board model. It is an affordance hint only — the
//! engine remains the authority and rejects an illegal declaration.

use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::mana::ManaColor;
use baylee_engine::choice::{
    CastModeDesc, ChoicePrompt, LegalActions, Pending, PlayerAction, YesNoPrompt,
};

/// What the player is being asked, in renderer-friendly terms.
// Not `PartialEq`: two of the variants carry engine types that are plain
// data without an equality impl, and a prompt is displayed rather than
// compared.
#[derive(Clone, Debug)]
pub enum Prompt {
    /// Nothing is being asked of this seat.
    Waiting {
        /// Who the game is waiting for, when it is waiting for a player.
        on: Option<PlayerId>,
    },
    /// Keep or mulligan.
    Mulligan {
        /// Mulligans already taken.
        taken: u8,
        /// Whether the next one costs nothing.
        free: bool,
    },
    /// Put cards on the bottom after keeping.
    BottomCards {
        /// How many.
        count: u8,
    },
    /// The seat holds priority.
    Priority {
        /// Everything the engine says is legal right now.
        legal: Box<LegalActions>,
    },
    /// Declare attackers.
    DeclareAttackers,
    /// Declare blockers.
    DeclareBlockers {
        /// The attacking player.
        attacker: PlayerId,
    },
    /// Discard down to maximum hand size.
    Discard {
        /// How many cards.
        count: u8,
    },
    /// Legend rule: keep one.
    LegendRule,
    /// Choose cards from an offered set.
    ChooseCards {
        /// Minimum.
        min: u8,
        /// Maximum.
        max: u8,
        /// Why.
        reason: ChoicePrompt,
    },
    /// Choose targets.
    ChooseTargets {
        /// Minimum.
        min: u8,
        /// Maximum.
        max: u8,
    },
    /// Choose a creature type.
    ChooseSubtype,
    /// Choose a colour.
    ChooseColor {
        /// The allowed colours.
        options: Vec<ManaColor>,
    },
    /// Choose a number, typically X.
    ChooseNumber {
        /// Lowest legal value.
        min: u32,
        /// Highest legal value.
        max: u32,
    },
    /// Choose a player.
    ChoosePlayer {
        /// The candidate seats.
        options: Vec<PlayerId>,
    },
    /// Choose how to cast a spell.
    CastMode {
        /// The offered options.
        options: Vec<CastModeDesc>,
    },
    /// Put objects in an order.
    OrderObjects,
    /// A yes-or-no question.
    YesNo {
        /// What is being decided.
        question: YesNoPrompt,
    },
    /// The game is over.
    GameOver,
}

impl Prompt {
    /// A short line for the prompt bar.
    #[must_use]
    pub fn headline(&self) -> String {
        match self {
            Self::Waiting { on: Some(p) } => format!("Waiting for seat {p}"),
            Self::Waiting { on: None } => "Waiting".to_string(),
            Self::Mulligan { taken, free } => {
                if *free {
                    "Keep this hand? (the next mulligan is free)".to_string()
                } else {
                    format!("Keep this hand? ({taken} taken)")
                }
            }
            Self::BottomCards { count } => format!("Put {count} card(s) on the bottom"),
            Self::Priority { .. } => "You have priority".to_string(),
            Self::DeclareAttackers => "Declare attackers".to_string(),
            Self::DeclareBlockers { .. } => "Declare blockers".to_string(),
            Self::Discard { count } => format!("Discard {count} card(s)"),
            Self::LegendRule => "Legend rule: keep one".to_string(),
            Self::ChooseCards { min, max, .. } => choose_line("card", *min, *max),
            Self::ChooseTargets { min, max } => choose_line("target", *min, *max),
            Self::ChooseSubtype => "Choose a creature type".to_string(),
            Self::ChooseColor { .. } => "Choose a colour".to_string(),
            Self::ChooseNumber { min, max } => format!("Choose a number ({min}–{max})"),
            Self::ChoosePlayer { .. } => "Choose a player".to_string(),
            Self::CastMode { .. } => "Choose how to cast".to_string(),
            Self::OrderObjects => "Put these in order".to_string(),
            Self::YesNo { question } => yes_no_line(*question),
            Self::GameOver => "The game is over".to_string(),
        }
    }
}

fn choose_line(noun: &str, min: u8, max: u8) -> String {
    match (min, max) {
        (0, m) => format!("Choose up to {m} {noun}(s)"),
        (a, b) if a == b => format!("Choose {a} {noun}(s)"),
        (a, b) => format!("Choose {a}–{b} {noun}(s)"),
    }
}

fn yes_no_line(question: YesNoPrompt) -> String {
    match question {
        YesNoPrompt::PayLifeOrEnterTapped { amount } => {
            format!("Pay {amount} life? Otherwise it enters tapped")
        }
        YesNoPrompt::Kicker => "Pay the additional cost?".to_string(),
        YesNoPrompt::PayTax { mana } => format!("Pay {{{mana}}}?"),
        YesNoPrompt::Miracle { .. } => "Cast it for its miracle cost?".to_string(),
        YesNoPrompt::Generic => "Yes or no?".to_string(),
    }
}

/// What happened when the player touched something.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SelectionOutcome {
    /// The object joined the selection.
    Added,
    /// The object left the selection.
    Removed,
    /// The object is not selectable for this choice.
    Rejected,
    /// The selection is already at its maximum.
    Full,
}

/// Extra affordance the engine does not enumerate, supplied by the caller.
#[derive(Clone, Default, Debug)]
pub struct CombatCandidates {
    /// Creatures that may be declared as attackers.
    pub attackers: Vec<ObjectId>,
    /// Creatures that may be declared as blockers.
    pub blockers: Vec<ObjectId>,
    /// Attacking creatures a blocker may be assigned to.
    pub attacking: Vec<ObjectId>,
    /// Players an attacker may be sent at.
    pub defenders: Vec<PlayerId>,
}

/// Internal shape of the answer being assembled.
#[derive(Clone, Debug)]
enum Mode {
    /// Nothing to answer.
    Idle,
    /// A set of objects, bounded by `min` and `max`.
    Objects {
        options: Vec<ObjectId>,
        min: usize,
        max: usize,
    },
    /// An ordered list; every offered object must appear exactly once.
    Order { options: Vec<ObjectId> },
    /// Attacker declarations.
    Attackers {
        candidates: Vec<ObjectId>,
        defenders: Vec<PlayerId>,
        pairs: Vec<(ObjectId, PlayerId)>,
    },
    /// Blocker declarations.
    Blockers {
        candidates: Vec<ObjectId>,
        attacking: Vec<ObjectId>,
        pairs: Vec<(ObjectId, ObjectId)>,
    },
    /// A bounded number.
    Number { min: u32, max: u32 },
    /// One of a fixed set of colours.
    Color { options: Vec<ManaColor> },
    /// One of a fixed set of seats.
    Player { options: Vec<PlayerId> },
    /// One of a fixed set of cast options.
    CastOption { count: usize },
    /// A yes-or-no answer.
    YesNo,
    /// Priority: an action menu rather than a selection.
    Priority { legal: Box<LegalActions> },
    /// Keep or mulligan.
    Mulligan,
    /// The game has ended.
    GameOver,
}

/// The interaction state for one pending choice.
pub struct Interaction {
    pending: Pending,
    seat: PlayerId,
    mode: Mode,
    selected: Vec<ObjectId>,
    number: u32,
    choice_index: Option<usize>,
}

impl Interaction {
    /// Builds the interaction for a pending choice as seen by `seat`.
    ///
    /// `combat` supplies the candidate lists for attack and block declarations,
    /// which the engine's choice does not carry.
    #[must_use]
    pub fn new(pending: Pending, seat: PlayerId, combat: &CombatCandidates) -> Self {
        let mode = Self::mode_for(&pending, seat, combat);
        let number = match &mode {
            Mode::Number { min, .. } => *min,
            _ => 0,
        };
        Self {
            pending,
            seat,
            mode,
            selected: Vec::new(),
            number,
            choice_index: None,
        }
    }

    fn mode_for(pending: &Pending, seat: PlayerId, combat: &CombatCandidates) -> Mode {
        if pending_player(pending) != Some(seat) {
            return if matches!(pending, Pending::GameOver(_)) {
                Mode::GameOver
            } else {
                Mode::Idle
            };
        }
        match pending {
            Pending::Mulligan { .. } => Mode::Mulligan,
            Pending::MulliganBottom { count, .. } | Pending::DiscardChoice { count, .. } => {
                Mode::Objects {
                    options: Vec::new(),
                    min: *count as usize,
                    max: *count as usize,
                }
            }
            Pending::Priority { legal, .. } => Mode::Priority {
                legal: legal.clone(),
            },
            Pending::ChooseAttackers { .. } => Mode::Attackers {
                candidates: combat.attackers.clone(),
                defenders: combat.defenders.clone(),
                pairs: Vec::new(),
            },
            Pending::ChooseBlockers { .. } => Mode::Blockers {
                candidates: combat.blockers.clone(),
                attacking: combat.attacking.clone(),
                pairs: Vec::new(),
            },
            Pending::LegendChoice { options, .. } => Mode::Objects {
                options: options.clone(),
                min: 1,
                max: 1,
            },
            Pending::ChooseCards {
                options, min, max, ..
            }
            | Pending::ChooseTargets {
                options, min, max, ..
            } => Mode::Objects {
                options: options.clone(),
                min: *min as usize,
                max: *max as usize,
            },
            Pending::OrderObjects { objects, .. } => Mode::Order {
                options: objects.clone(),
            },
            Pending::ChooseColor { options, .. } => Mode::Color {
                options: options.clone(),
            },
            Pending::ChoosePlayer { options, .. } => Mode::Player {
                options: options.clone(),
            },
            Pending::ChooseNumber { min, max, .. } => Mode::Number {
                min: *min,
                max: *max,
            },
            Pending::ChooseCastMode { options, .. } => Mode::CastOption {
                count: options.len(),
            },
            Pending::YesNo { .. } => Mode::YesNo,
            // Subtype selection is answered from a searchable list in the UI
            // rather than by clicking the board.
            Pending::ChooseSubtype { .. } => Mode::Idle,
            Pending::GameOver(_) => Mode::GameOver,
        }
    }

    /// The pending choice this interaction wraps.
    #[must_use]
    pub fn pending(&self) -> &Pending {
        &self.pending
    }

    /// Whether this seat is the one being asked.
    #[must_use]
    pub fn is_mine(&self) -> bool {
        pending_player(&self.pending) == Some(self.seat)
    }

    /// What the player is being asked.
    #[must_use]
    pub fn prompt(&self) -> Prompt {
        if !self.is_mine() {
            return match &self.pending {
                Pending::GameOver(_) => Prompt::GameOver,
                other => Prompt::Waiting {
                    on: pending_player(other),
                },
            };
        }
        match &self.pending {
            Pending::Mulligan {
                taken,
                next_is_free,
                ..
            } => Prompt::Mulligan {
                taken: *taken,
                free: *next_is_free,
            },
            Pending::MulliganBottom { count, .. } => Prompt::BottomCards { count: *count },
            Pending::Priority { legal, .. } => Prompt::Priority {
                legal: legal.clone(),
            },
            Pending::ChooseAttackers { .. } => Prompt::DeclareAttackers,
            Pending::ChooseBlockers { attacker, .. } => Prompt::DeclareBlockers {
                attacker: *attacker,
            },
            Pending::DiscardChoice { count, .. } => Prompt::Discard { count: *count },
            Pending::LegendChoice { .. } => Prompt::LegendRule,
            Pending::ChooseCards {
                min, max, prompt, ..
            } => Prompt::ChooseCards {
                min: *min,
                max: *max,
                reason: *prompt,
            },
            Pending::ChooseTargets { min, max, .. } => Prompt::ChooseTargets {
                min: *min,
                max: *max,
            },
            Pending::ChooseSubtype { .. } => Prompt::ChooseSubtype,
            Pending::ChooseColor { options, .. } => Prompt::ChooseColor {
                options: options.clone(),
            },
            Pending::ChooseNumber { min, max, .. } => Prompt::ChooseNumber {
                min: *min,
                max: *max,
            },
            Pending::ChoosePlayer { options, .. } => Prompt::ChoosePlayer {
                options: options.clone(),
            },
            Pending::ChooseCastMode { options, .. } => Prompt::CastMode {
                options: options.clone(),
            },
            Pending::OrderObjects { .. } => Prompt::OrderObjects,
            Pending::YesNo { prompt, .. } => Prompt::YesNo { question: *prompt },
            Pending::GameOver(_) => Prompt::GameOver,
        }
    }

    /// Objects the player may touch for this choice.
    ///
    /// An empty list with a non-idle mode means "any card in the relevant
    /// zone": mulligan bottoming and discarding operate on the seat's own hand,
    /// which the engine does not enumerate because it is already private.
    #[must_use]
    pub fn selectable(&self) -> &[ObjectId] {
        match &self.mode {
            Mode::Objects { options, .. } | Mode::Order { options } => options,
            Mode::Attackers { candidates, .. } | Mode::Blockers { candidates, .. } => candidates,
            _ => &[],
        }
    }

    /// Whether an object may be selected.
    ///
    /// Choices whose options the engine leaves implicit (the seat's own hand)
    /// accept anything; every enumerated choice accepts only what was offered.
    #[must_use]
    pub fn is_selectable(&self, id: ObjectId) -> bool {
        match &self.mode {
            Mode::Objects { options, .. } => options.is_empty() || options.contains(&id),
            Mode::Order { options } => options.contains(&id),
            Mode::Attackers { candidates, .. } | Mode::Blockers { candidates, .. } => {
                candidates.contains(&id)
            }
            _ => false,
        }
    }

    /// The current selection, in the order it was made.
    #[must_use]
    pub fn selected(&self) -> &[ObjectId] {
        &self.selected
    }

    /// Whether an object is currently selected.
    #[must_use]
    pub fn is_selected(&self, id: ObjectId) -> bool {
        self.selected.contains(&id)
    }

    /// Adds or removes an object from the selection.
    pub fn toggle(&mut self, id: ObjectId) -> SelectionOutcome {
        if !self.is_selectable(id) {
            return SelectionOutcome::Rejected;
        }
        if let Some(pos) = self.selected.iter().position(|o| *o == id) {
            self.selected.remove(pos);
            return SelectionOutcome::Removed;
        }
        let max = match &self.mode {
            Mode::Objects { max, .. } => *max,
            Mode::Order { options } => options.len(),
            Mode::Attackers { candidates, .. } | Mode::Blockers { candidates, .. } => {
                candidates.len()
            }
            _ => 0,
        };
        if self.selected.len() >= max {
            return SelectionOutcome::Full;
        }
        self.selected.push(id);
        SelectionOutcome::Added
    }

    /// Clears the selection without answering.
    pub fn cancel(&mut self) {
        self.selected.clear();
        self.choice_index = None;
        if let Mode::Attackers { pairs, .. } = &mut self.mode {
            pairs.clear();
        }
        if let Mode::Blockers { pairs, .. } = &mut self.mode {
            pairs.clear();
        }
    }

    /// Declares `attacker` as attacking `defender`.
    ///
    /// Returns `false` when either side is not a candidate, so the caller can
    /// play a rejection cue instead of sending an action that will bounce.
    pub fn declare_attacker(&mut self, attacker: ObjectId, defender: PlayerId) -> bool {
        let Mode::Attackers {
            candidates,
            defenders,
            pairs,
        } = &mut self.mode
        else {
            return false;
        };
        if !candidates.contains(&attacker) || !defenders.contains(&defender) {
            return false;
        }
        pairs.retain(|(a, _)| *a != attacker);
        pairs.push((attacker, defender));
        true
    }

    /// Declares `blocker` as blocking `attacker`.
    pub fn declare_blocker(&mut self, blocker: ObjectId, attacker: ObjectId) -> bool {
        let Mode::Blockers {
            candidates,
            attacking,
            pairs,
        } = &mut self.mode
        else {
            return false;
        };
        if !candidates.contains(&blocker) || !attacking.contains(&attacker) {
            return false;
        }
        pairs.retain(|(b, _)| *b != blocker);
        pairs.push((blocker, attacker));
        true
    }

    /// Sets the number for an X choice, clamped to the offered range.
    ///
    /// Clamping rather than rejecting keeps a keyboard or a slider usable: the
    /// player can hold a key and stop at the boundary. The engine's own bounds
    /// are the only ones that exist here, so an out-of-range X is not
    /// expressible.
    pub fn set_number(&mut self, value: u32) -> u32 {
        if let Mode::Number { min, max, .. } = &self.mode {
            self.number = value.clamp(*min, *max);
        }
        self.number
    }

    /// The currently chosen number.
    #[must_use]
    pub const fn number(&self) -> u32 {
        self.number
    }

    /// Picks an indexed option (a cast mode, a colour, or a seat).
    ///
    /// Returns `false` when the index is not one the engine offered.
    pub fn choose_index(&mut self, index: usize) -> bool {
        let count = match &self.mode {
            Mode::CastOption { count } => *count,
            Mode::Color { options } => options.len(),
            Mode::Player { options } => options.len(),
            _ => 0,
        };
        if index >= count {
            return false;
        }
        self.choice_index = Some(index);
        true
    }

    /// Whether the current answer is complete enough to submit.
    #[must_use]
    pub fn can_confirm(&self) -> bool {
        match &self.mode {
            Mode::Objects { min, .. } => self.selected.len() >= *min,
            Mode::Order { options } => self.selected.len() == options.len(),
            // Declaring nothing is always legal (no attacks, no blocks), a
            // number always has its clamped value, and priority can always be
            // passed — all four are answerable the moment they are asked.
            Mode::Attackers { .. }
            | Mode::Blockers { .. }
            | Mode::Number { .. }
            | Mode::Priority { .. } => true,
            Mode::Color { .. } | Mode::Player { .. } | Mode::CastOption { .. } => {
                self.choice_index.is_some()
            }
            Mode::Mulligan | Mode::YesNo | Mode::Idle | Mode::GameOver => false,
        }
    }

    /// Builds the action for the current answer, if it is complete.
    #[must_use]
    pub fn confirm(&self) -> Option<PlayerAction> {
        if !self.is_mine() {
            return None;
        }
        match &self.mode {
            Mode::Objects { min, .. } if self.selected.len() >= *min => {
                Some(PlayerAction::ChooseObjects {
                    objects: self.selected.clone(),
                })
            }
            Mode::Order { options } if self.selected.len() == options.len() => {
                Some(PlayerAction::OrderObjects {
                    objects: self.selected.clone(),
                })
            }
            Mode::Attackers { pairs, .. } => Some(PlayerAction::DeclareAttackers {
                attackers: pairs.clone(),
            }),
            Mode::Blockers { pairs, .. } => Some(PlayerAction::DeclareBlockers {
                blockers: pairs.clone(),
            }),
            Mode::Number { .. } => Some(PlayerAction::ChooseNumber(self.number)),
            Mode::Priority { .. } => Some(PlayerAction::PassPriority),
            Mode::Color { options } => options
                .get(self.choice_index?)
                .copied()
                .map(PlayerAction::ChooseColor),
            Mode::Player { options } => options
                .get(self.choice_index?)
                .copied()
                .map(PlayerAction::ChoosePlayer),
            Mode::CastOption { count } => {
                let index = self.choice_index?;
                (index < *count).then_some(PlayerAction::ChooseMode(index))
            }
            _ => None,
        }
    }

    /// Answers a yes-or-no question.
    #[must_use]
    pub fn answer_yes_no(&self, yes: bool) -> Option<PlayerAction> {
        matches!(self.mode, Mode::YesNo).then_some(PlayerAction::YesNo(yes))
    }

    /// Answers a mulligan decision.
    #[must_use]
    pub fn answer_mulligan(&self, keep: bool) -> Option<PlayerAction> {
        matches!(self.mode, Mode::Mulligan).then(|| {
            if keep {
                PlayerAction::MulliganKeep
            } else {
                PlayerAction::MulliganTake
            }
        })
    }

    /// The legal actions offered with priority, if this is a priority choice.
    #[must_use]
    pub fn legal_actions(&self) -> Option<&LegalActions> {
        match &self.mode {
            Mode::Priority { legal } => Some(legal),
            _ => None,
        }
    }

    /// Builds the action for playing a card while holding priority, rejecting
    /// anything the engine did not list as legal.
    #[must_use]
    pub fn play_card(&self, card: ObjectId) -> Option<PlayerAction> {
        let legal = self.legal_actions()?;
        if legal.lands.contains(&card) {
            Some(PlayerAction::PlayLand { card })
        } else if legal.castable.contains(&card) {
            Some(PlayerAction::CastSpell { card })
        } else {
            None
        }
    }

    /// Builds the action for activating an ability, rejecting anything not
    /// offered.
    #[must_use]
    pub fn activate(&self, source: ObjectId, ability_index: u32) -> Option<PlayerAction> {
        let legal = self.legal_actions()?;
        if legal.mana_abilities.contains(&source) && ability_index == 0 {
            return Some(PlayerAction::ActivateManaAbility { source });
        }
        legal
            .abilities
            .contains(&(source, ability_index))
            .then_some(PlayerAction::ActivateAbility {
                source,
                ability_index,
            })
    }
}

/// The player a pending choice is addressed to.
#[must_use]
pub fn pending_player(pending: &Pending) -> Option<PlayerId> {
    match pending {
        Pending::Mulligan { player, .. }
        | Pending::MulliganBottom { player, .. }
        | Pending::Priority { player, .. }
        | Pending::ChooseAttackers { player }
        | Pending::ChooseBlockers { player, .. }
        | Pending::DiscardChoice { player, .. }
        | Pending::LegendChoice { player, .. }
        | Pending::ChooseCards { player, .. }
        | Pending::ChooseTargets { player, .. }
        | Pending::ChooseSubtype { player, .. }
        | Pending::ChooseColor { player, .. }
        | Pending::ChooseNumber { player, .. }
        | Pending::ChoosePlayer { player, .. }
        | Pending::ChooseCastMode { player, .. }
        | Pending::OrderObjects { player, .. }
        | Pending::YesNo { player, .. } => Some(*player),
        Pending::GameOver(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn me() -> PlayerId {
        PlayerId::new(0)
    }

    fn obj(slot: u32) -> ObjectId {
        ObjectId::new(slot, 0)
    }

    fn no_combat() -> CombatCandidates {
        CombatCandidates::default()
    }

    fn interaction(pending: Pending) -> Interaction {
        Interaction::new(pending, me(), &no_combat())
    }

    #[test]
    fn a_choice_addressed_to_another_seat_is_not_actionable() {
        let mut i = interaction(Pending::ChooseTargets {
            player: PlayerId::new(1),
            options: vec![obj(1)],
            min: 1,
            max: 1,
        });
        assert!(!i.is_mine());
        assert_eq!(i.toggle(obj(1)), SelectionOutcome::Rejected);
        assert!(i.confirm().is_none());
        assert!(matches!(i.prompt(), Prompt::Waiting { on: Some(_) }));
    }

    #[test]
    fn only_offered_targets_can_be_selected() {
        let mut i = interaction(Pending::ChooseTargets {
            player: me(),
            options: vec![obj(1), obj(2)],
            min: 1,
            max: 1,
        });
        assert_eq!(i.toggle(obj(1)), SelectionOutcome::Added);
        // Not in the offered set: the client refuses to even express it.
        assert_eq!(i.toggle(obj(99)), SelectionOutcome::Rejected);
        assert_eq!(i.selected(), &[obj(1)]);
    }

    #[test]
    fn the_maximum_is_enforced_and_toggling_off_frees_a_slot() {
        let mut i = interaction(Pending::ChooseCards {
            player: me(),
            options: vec![obj(1), obj(2), obj(3)],
            min: 1,
            max: 2,
            prompt: ChoicePrompt::Generic,
        });
        assert_eq!(i.toggle(obj(1)), SelectionOutcome::Added);
        assert_eq!(i.toggle(obj(2)), SelectionOutcome::Added);
        assert_eq!(i.toggle(obj(3)), SelectionOutcome::Full);
        assert_eq!(i.toggle(obj(1)), SelectionOutcome::Removed);
        assert_eq!(i.toggle(obj(3)), SelectionOutcome::Added);
    }

    #[test]
    fn a_minimum_blocks_confirmation_until_it_is_met() {
        let mut i = interaction(Pending::ChooseCards {
            player: me(),
            options: vec![obj(1), obj(2)],
            min: 2,
            max: 2,
            prompt: ChoicePrompt::Generic,
        });
        assert!(!i.can_confirm());
        i.toggle(obj(1));
        assert!(!i.can_confirm());
        i.toggle(obj(2));
        assert!(i.can_confirm());
        assert_eq!(
            i.confirm(),
            Some(PlayerAction::ChooseObjects {
                objects: vec![obj(1), obj(2)]
            })
        );
    }

    #[test]
    fn an_up_to_choice_can_be_confirmed_with_nothing_selected() {
        let i = interaction(Pending::ChooseTargets {
            player: me(),
            options: vec![obj(1)],
            min: 0,
            max: 1,
        });
        assert!(i.can_confirm());
        assert_eq!(
            i.confirm(),
            Some(PlayerAction::ChooseObjects { objects: vec![] })
        );
    }

    #[test]
    fn x_is_clamped_to_the_range_the_engine_offered() {
        let mut i = interaction(Pending::ChooseNumber {
            player: me(),
            min: 0,
            max: 50,
        });
        assert_eq!(i.set_number(7), 7);
        // The client cannot express a value outside the offered range, so the
        // usual overflow tricks are simply unavailable to a player.
        assert_eq!(i.set_number(u32::MAX), 50);
        assert_eq!(i.set_number(4_000_000_000), 50);
        assert_eq!(i.confirm(), Some(PlayerAction::ChooseNumber(50)));
    }

    #[test]
    fn x_starts_at_the_minimum() {
        let i = interaction(Pending::ChooseNumber {
            player: me(),
            min: 3,
            max: 9,
        });
        assert_eq!(i.number(), 3);
    }

    #[test]
    fn ordering_requires_every_offered_object_exactly_once() {
        let mut i = interaction(Pending::OrderObjects {
            player: me(),
            objects: vec![obj(1), obj(2), obj(3)],
        });
        i.toggle(obj(2));
        i.toggle(obj(3));
        assert!(!i.can_confirm(), "an incomplete order is not submittable");
        i.toggle(obj(1));
        assert_eq!(
            i.confirm(),
            Some(PlayerAction::OrderObjects {
                objects: vec![obj(2), obj(3), obj(1)]
            })
        );
    }

    #[test]
    fn ordering_rejects_objects_that_were_not_offered() {
        let mut i = interaction(Pending::OrderObjects {
            player: me(),
            objects: vec![obj(1)],
        });
        assert_eq!(i.toggle(obj(42)), SelectionOutcome::Rejected);
    }

    #[test]
    fn a_colour_choice_only_accepts_offered_colours() {
        let mut i = interaction(Pending::ChooseColor {
            player: me(),
            options: vec![ManaColor::White, ManaColor::Blue],
        });
        assert!(!i.can_confirm());
        assert!(!i.choose_index(2), "index beyond the offered options");
        assert!(i.choose_index(1));
        assert_eq!(
            i.confirm(),
            Some(PlayerAction::ChooseColor(ManaColor::Blue))
        );
    }

    #[test]
    fn a_player_choice_only_accepts_offered_seats() {
        let mut i = interaction(Pending::ChoosePlayer {
            player: me(),
            options: vec![PlayerId::new(2), PlayerId::new(3)],
        });
        assert!(!i.choose_index(5));
        assert!(i.choose_index(0));
        assert_eq!(
            i.confirm(),
            Some(PlayerAction::ChoosePlayer(PlayerId::new(2)))
        );
    }

    #[test]
    fn declaring_attackers_checks_both_the_creature_and_the_defender() {
        let combat = CombatCandidates {
            attackers: vec![obj(1), obj(2)],
            defenders: vec![PlayerId::new(1)],
            ..CombatCandidates::default()
        };
        let mut i = Interaction::new(Pending::ChooseAttackers { player: me() }, me(), &combat);

        assert!(
            !i.declare_attacker(obj(9), PlayerId::new(1)),
            "not a candidate"
        );
        assert!(
            !i.declare_attacker(obj(1), PlayerId::new(7)),
            "not a defender"
        );
        assert!(i.declare_attacker(obj(1), PlayerId::new(1)));

        assert_eq!(
            i.confirm(),
            Some(PlayerAction::DeclareAttackers {
                attackers: vec![(obj(1), PlayerId::new(1))]
            })
        );
    }

    #[test]
    fn re_declaring_an_attacker_replaces_its_defender_rather_than_duplicating() {
        let combat = CombatCandidates {
            attackers: vec![obj(1)],
            defenders: vec![PlayerId::new(1), PlayerId::new(2)],
            ..CombatCandidates::default()
        };
        let mut i = Interaction::new(Pending::ChooseAttackers { player: me() }, me(), &combat);
        i.declare_attacker(obj(1), PlayerId::new(1));
        i.declare_attacker(obj(1), PlayerId::new(2));
        assert_eq!(
            i.confirm(),
            Some(PlayerAction::DeclareAttackers {
                attackers: vec![(obj(1), PlayerId::new(2))]
            })
        );
    }

    #[test]
    fn declaring_no_attackers_is_a_valid_answer() {
        let combat = CombatCandidates {
            attackers: vec![obj(1)],
            defenders: vec![PlayerId::new(1)],
            ..CombatCandidates::default()
        };
        let i = Interaction::new(Pending::ChooseAttackers { player: me() }, me(), &combat);
        assert!(i.can_confirm());
        assert_eq!(
            i.confirm(),
            Some(PlayerAction::DeclareAttackers { attackers: vec![] })
        );
    }

    #[test]
    fn blockers_must_block_an_actual_attacker() {
        let combat = CombatCandidates {
            blockers: vec![obj(10)],
            attacking: vec![obj(1)],
            ..CombatCandidates::default()
        };
        let mut i = Interaction::new(
            Pending::ChooseBlockers {
                player: me(),
                attacker: PlayerId::new(1),
            },
            me(),
            &combat,
        );
        assert!(!i.declare_blocker(obj(10), obj(99)));
        assert!(i.declare_blocker(obj(10), obj(1)));
        assert_eq!(
            i.confirm(),
            Some(PlayerAction::DeclareBlockers {
                blockers: vec![(obj(10), obj(1))]
            })
        );
    }

    #[test]
    fn priority_confirms_as_a_pass_and_exposes_the_legal_actions() {
        let legal = LegalActions {
            can_pass: true,
            lands: vec![obj(1)],
            castable: vec![obj(2)],
            mana_abilities: vec![obj(3)],
            abilities: vec![(obj(4), 1)],
            suspendable: vec![],
        };
        let i = interaction(Pending::Priority {
            player: me(),
            legal: Box::new(legal),
        });
        assert_eq!(i.confirm(), Some(PlayerAction::PassPriority));
        assert!(i.legal_actions().is_some());
    }

    #[test]
    fn playing_a_card_maps_to_the_right_action_and_refuses_illegal_ones() {
        let legal = LegalActions {
            can_pass: true,
            lands: vec![obj(1)],
            castable: vec![obj(2)],
            mana_abilities: vec![],
            abilities: vec![],
            suspendable: vec![],
        };
        let i = interaction(Pending::Priority {
            player: me(),
            legal: Box::new(legal),
        });
        assert_eq!(
            i.play_card(obj(1)),
            Some(PlayerAction::PlayLand { card: obj(1) })
        );
        assert_eq!(
            i.play_card(obj(2)),
            Some(PlayerAction::CastSpell { card: obj(2) })
        );
        // A card the engine did not list is not playable, whatever the board
        // looks like.
        assert_eq!(i.play_card(obj(3)), None);
    }

    #[test]
    fn activating_an_ability_requires_it_to_have_been_offered() {
        let legal = LegalActions {
            can_pass: true,
            lands: vec![],
            castable: vec![],
            mana_abilities: vec![obj(5)],
            abilities: vec![(obj(6), 2)],
            suspendable: vec![],
        };
        let i = interaction(Pending::Priority {
            player: me(),
            legal: Box::new(legal),
        });
        assert_eq!(
            i.activate(obj(5), 0),
            Some(PlayerAction::ActivateManaAbility { source: obj(5) })
        );
        assert_eq!(
            i.activate(obj(6), 2),
            Some(PlayerAction::ActivateAbility {
                source: obj(6),
                ability_index: 2
            })
        );
        assert_eq!(i.activate(obj(6), 3), None, "wrong ability index");
        assert_eq!(i.activate(obj(7), 0), None, "not a listed source");
    }

    #[test]
    fn mulligan_and_yes_no_answers_are_mode_gated() {
        let mull = interaction(Pending::Mulligan {
            player: me(),
            taken: 1,
            next_is_free: false,
        });
        assert_eq!(mull.answer_mulligan(true), Some(PlayerAction::MulliganKeep));
        assert_eq!(
            mull.answer_mulligan(false),
            Some(PlayerAction::MulliganTake)
        );
        // A mulligan is not a yes/no question, and answering it as one is not
        // possible.
        assert_eq!(mull.answer_yes_no(true), None);

        let yn = interaction(Pending::YesNo {
            player: me(),
            prompt: YesNoPrompt::Generic,
        });
        assert_eq!(yn.answer_yes_no(true), Some(PlayerAction::YesNo(true)));
        assert_eq!(yn.answer_mulligan(true), None);
    }

    #[test]
    fn discarding_operates_on_the_hand_which_the_engine_leaves_implicit() {
        let mut i = interaction(Pending::DiscardChoice {
            player: me(),
            count: 2,
        });
        // No enumerated options, so any card in hand is fair game.
        assert!(i.selectable().is_empty());
        assert_eq!(i.toggle(obj(1)), SelectionOutcome::Added);
        assert_eq!(i.toggle(obj(2)), SelectionOutcome::Added);
        assert_eq!(i.toggle(obj(3)), SelectionOutcome::Full);
        assert!(i.can_confirm());
    }

    #[test]
    fn cancelling_clears_a_selection_and_any_declarations() {
        let combat = CombatCandidates {
            attackers: vec![obj(1)],
            defenders: vec![PlayerId::new(1)],
            ..CombatCandidates::default()
        };
        let mut i = Interaction::new(Pending::ChooseAttackers { player: me() }, me(), &combat);
        i.declare_attacker(obj(1), PlayerId::new(1));
        i.cancel();
        assert_eq!(
            i.confirm(),
            Some(PlayerAction::DeclareAttackers { attackers: vec![] })
        );
    }

    #[test]
    fn prompt_headlines_are_written_for_a_player_not_a_developer() {
        let i = interaction(Pending::ChooseTargets {
            player: me(),
            options: vec![obj(1)],
            min: 0,
            max: 2,
        });
        assert_eq!(i.prompt().headline(), "Choose up to 2 target(s)");

        let i = interaction(Pending::ChooseNumber {
            player: me(),
            min: 0,
            max: 50,
        });
        assert_eq!(i.prompt().headline(), "Choose a number (0–50)");

        let i = interaction(Pending::YesNo {
            player: me(),
            prompt: YesNoPrompt::PayLifeOrEnterTapped { amount: 2 },
        });
        assert_eq!(
            i.prompt().headline(),
            "Pay 2 life? Otherwise it enters tapped"
        );
    }

    #[test]
    fn every_pending_variant_produces_a_prompt_without_panicking() {
        // A completeness guard: adding a choice to the engine without teaching
        // the client about it should fail here rather than at the table.
        let all = vec![
            Pending::Mulligan {
                player: me(),
                taken: 0,
                next_is_free: true,
            },
            Pending::MulliganBottom {
                player: me(),
                count: 1,
            },
            Pending::Priority {
                player: me(),
                legal: Box::new(LegalActions {
                    can_pass: true,
                    lands: vec![],
                    castable: vec![],
                    mana_abilities: vec![],
                    abilities: vec![],
                    suspendable: vec![],
                }),
            },
            Pending::ChooseAttackers { player: me() },
            Pending::ChooseBlockers {
                player: me(),
                attacker: PlayerId::new(1),
            },
            Pending::DiscardChoice {
                player: me(),
                count: 1,
            },
            Pending::LegendChoice {
                player: me(),
                options: vec![obj(1), obj(2)],
            },
            Pending::ChooseCards {
                player: me(),
                options: vec![],
                min: 0,
                max: 1,
                prompt: ChoicePrompt::Generic,
            },
            Pending::ChooseTargets {
                player: me(),
                options: vec![],
                min: 0,
                max: 1,
            },
            Pending::ChooseSubtype {
                player: me(),
                options: vec![],
            },
            Pending::ChooseColor {
                player: me(),
                options: vec![ManaColor::White],
            },
            Pending::ChooseNumber {
                player: me(),
                min: 0,
                max: 1,
            },
            Pending::ChoosePlayer {
                player: me(),
                options: vec![me()],
            },
            Pending::ChooseCastMode {
                player: me(),
                options: vec![],
            },
            Pending::OrderObjects {
                player: me(),
                objects: vec![],
            },
            Pending::YesNo {
                player: me(),
                prompt: YesNoPrompt::Generic,
            },
        ];
        for pending in all {
            let i = interaction(pending);
            assert!(!i.prompt().headline().is_empty());
        }
    }
}
