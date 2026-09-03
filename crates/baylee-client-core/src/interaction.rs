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
//! that do **not** carry the list of *creatures* to offer, unlike every other
//! variant. The caller passes those in from the board model as an affordance
//! hint only — the engine remains the authority and rejects an illegal
//! declaration.
//!
//! What an attacker may be sent *at* is different: `ChooseAttackers` carries
//! its defender list, because "which planeswalkers may I attack" (CR 508.1a)
//! is a rules question and re-deriving it client-side would be a second,
//! divergent implementation of it.

use crate::i18n::{Lang, Phrase};
use baylee_core::ids::{Defender, ObjectId, PlayerId};
use baylee_core::mana::ManaColor;
use baylee_engine::choice::{
    BlockOption, CastModeDesc, ChoicePrompt, LegalActions, Pending, PlayerAction, YesNoPrompt,
};

/// What a combat declaration is currently pointed at.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CombatFocus {
    /// Attacks declared now are sent at this defender.
    Defender(Defender),
    /// Blocks declared now are put in front of this attacker.
    Attacker(ObjectId),
    /// Not a combat choice, or nothing left to point at.
    None,
}

/// Every attacker any blocker may be assigned to, deduplicated, in the order
/// the engine listed them.
///
/// Stable ordering is what makes "the next attacker" mean anything to a
/// player stepping through them, so this is not a set.
fn ordered_attackers(options: &[BlockOption]) -> Vec<ObjectId> {
    let mut out: Vec<ObjectId> = Vec::new();
    for option in options {
        for attacker in &option.attackers {
            if !out.contains(attacker) {
                out.push(*attacker);
            }
        }
    }
    out
}

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
    pub fn headline(&self, lang: Lang) -> String {
        match self {
            Self::Waiting { on: Some(p) } => Phrase::WaitingForSeat.fill(lang, &[&p.to_string()]),
            Self::Waiting { on: None } => Phrase::JustWaiting.text(lang).to_string(),
            Self::Mulligan { taken, free } => {
                if *free {
                    Phrase::MulliganFree.text(lang).to_string()
                } else {
                    Phrase::MulliganTaken.fill(lang, &[&taken.to_string()])
                }
            }
            Self::BottomCards { count } => Phrase::PutOnBottom.fill(lang, &[&count.to_string()]),
            Self::Priority { .. } => Phrase::YouHavePriority.text(lang).to_string(),
            Self::DeclareAttackers => Phrase::DeclareAttackers.text(lang).to_string(),
            Self::DeclareBlockers { .. } => Phrase::DeclareBlockers.text(lang).to_string(),
            Self::Discard { count } => Phrase::DiscardCards.fill(lang, &[&count.to_string()]),
            Self::LegendRule => Phrase::LegendRule.text(lang).to_string(),
            Self::ChooseCards { min, max, .. } => choose_line(lang, Phrase::NounCards, *min, *max),
            Self::ChooseTargets { min, max } => choose_line(lang, Phrase::NounTargets, *min, *max),
            Self::ChooseSubtype => Phrase::ChooseCreatureType.text(lang).to_string(),
            Self::ChooseColor { .. } => Phrase::ChooseColour.text(lang).to_string(),
            Self::ChooseNumber { min, max } => {
                Phrase::ChooseNumberIn.fill(lang, &[&min.to_string(), &max.to_string()])
            }
            Self::ChoosePlayer { .. } => Phrase::ChoosePlayer.text(lang).to_string(),
            Self::CastMode { .. } => Phrase::ChooseHowToCast.text(lang).to_string(),
            Self::OrderObjects => Phrase::PutInOrder.text(lang).to_string(),
            Self::YesNo { question } => yes_no_line(lang, *question),
            Self::GameOver => Phrase::TheGameIsOver.text(lang).to_string(),
        }
    }
}

/// "Choose two cards", with the noun as an argument rather than glued on.
///
/// A count and a noun agree differently in different languages, so the whole
/// sentence has to be one phrase — pasting a translated noun onto a
/// translated "choose up to" is how a translation ends up ungrammatical.
fn choose_line(lang: Lang, noun: Phrase, min: u8, max: u8) -> String {
    let noun = noun.text(lang);
    match (min, max) {
        (0, m) => Phrase::ChooseUpTo.fill(lang, &[&m.to_string(), noun]),
        (a, b) if a == b => Phrase::ChooseExactly.fill(lang, &[&a.to_string(), noun]),
        (a, b) => Phrase::ChooseBetween.fill(lang, &[&a.to_string(), &b.to_string(), noun]),
    }
}

fn yes_no_line(lang: Lang, question: YesNoPrompt) -> String {
    match question {
        YesNoPrompt::PayLifeOrEnterTapped { amount } => {
            Phrase::PayLifeOrTapped.fill(lang, &[&amount.to_string()])
        }
        YesNoPrompt::Kicker => Phrase::PayAdditionalCost.text(lang).to_string(),
        YesNoPrompt::PayTax { mana } => Phrase::PayTax.fill(lang, &[&mana.to_string()]),
        YesNoPrompt::Miracle { .. } => Phrase::CastForMiracle.text(lang).to_string(),
        YesNoPrompt::DrawOffer { .. } => Phrase::DrawWasOffered.text(lang).to_string(),
        YesNoPrompt::Generic => Phrase::YesOrNo.text(lang).to_string(),
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

/// Internal shape of the answer being assembled.
#[derive(Clone, Debug)]
enum Mode {
    /// Nothing to answer.
    Idle,
    /// A set of objects, bounded by `min` and `max`.
    Objects {
        options: Vec<ObjectId>,
        /// Seats that may be chosen alongside the objects ("any target",
        /// CR 115.4). Empty for every prompt that names objects alone, so
        /// the ordinary "target creature" path is untouched.
        player_options: Vec<PlayerId>,
        min: usize,
        max: usize,
    },
    /// An ordered list; every offered object must appear exactly once.
    Order { options: Vec<ObjectId> },
    /// Attacker declarations.
    Attackers {
        candidates: Vec<ObjectId>,
        defenders: Vec<Defender>,
        pairs: Vec<(ObjectId, Defender)>,
        /// Which defender a newly declared attacker is sent at. An index
        /// rather than a `Defender` so it can be stepped through with one
        /// key, which is the whole point of having it.
        focus: usize,
    },
    /// Blocker declarations.
    Blockers {
        candidates: Vec<ObjectId>,
        options: Vec<BlockOption>,
        pairs: Vec<(ObjectId, ObjectId)>,
        /// Every attacker any blocker may be put in front of, in a stable
        /// order, so "the next attacker" means something.
        attackers: Vec<ObjectId>,
        /// Which of them a newly declared blocker is assigned to.
        focus: usize,
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
    /// Seats picked as targets, the other half of an "any target" answer.
    selected_players: Vec<PlayerId>,
    number: u32,
    choice_index: Option<usize>,
}

impl Interaction {
    /// Builds the interaction for a pending choice as seen by `seat`.
    ///
    /// Everything selectable comes out of the choice itself — including the
    /// creatures that may attack and which attackers each blocker may be
    /// assigned to. The client used to filter its own board for that and got
    /// it wrong for anything the printed card restricted.
    #[must_use]
    pub fn new(pending: Pending, seat: PlayerId) -> Self {
        let mode = Self::mode_for(&pending, seat);
        let number = match &mode {
            Mode::Number { min, .. } => *min,
            _ => 0,
        };
        Self {
            pending,
            seat,
            mode,
            selected: Vec::new(),
            selected_players: Vec::new(),
            number,
            choice_index: None,
        }
    }

    fn mode_for(pending: &Pending, seat: PlayerId) -> Mode {
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
                    player_options: Vec::new(),
                    min: *count as usize,
                    max: *count as usize,
                }
            }
            Pending::Priority { legal, .. } => Mode::Priority {
                legal: legal.clone(),
            },
            // The defender list comes from the engine rather than the
            // client: it is the one place that knows which planeswalkers
            // are attackable (CR 508.1a), and the engine validates a
            // declaration against exactly this list.
            Pending::ChooseAttackers {
                attackers,
                defenders,
                ..
            } => Mode::Attackers {
                candidates: attackers.clone(),
                defenders: defenders.clone(),
                pairs: Vec::new(),
                focus: 0,
            },
            Pending::ChooseBlockers { blockers, .. } => Mode::Blockers {
                candidates: blockers.iter().map(|b| b.blocker).collect(),
                attackers: ordered_attackers(blockers),
                options: blockers.clone(),
                pairs: Vec::new(),
                focus: 0,
            },
            Pending::LegendChoice { options, .. } => Mode::Objects {
                options: options.clone(),
                player_options: Vec::new(),
                min: 1,
                max: 1,
            },
            Pending::ChooseCards {
                options, min, max, ..
            } => Mode::Objects {
                options: options.clone(),
                player_options: Vec::new(),
                min: *min as usize,
                max: *max as usize,
            },
            Pending::ChooseTargets {
                options,
                player_options,
                min,
                max,
                ..
            } => Mode::Objects {
                options: options.clone(),
                player_options: player_options.clone(),
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
            // Combat accepts both halves of a pair: the creature being
            // declared, and the thing it is being declared against — tapping
            // a planeswalker or an attacker aims the next declaration.
            Mode::Attackers {
                candidates,
                defenders,
                ..
            } => candidates.contains(&id) || defenders.contains(&Defender::Planeswalker(id)),
            Mode::Blockers {
                candidates,
                attackers,
                ..
            } => candidates.contains(&id) || attackers.contains(&id),
            _ => false,
        }
    }

    /// The current selection, in the order it was made.
    #[must_use]
    pub fn selected(&self) -> &[ObjectId] {
        &self.selected
    }

    /// Whether an object is part of the answer being built.
    ///
    /// In combat that means declared, not merely touched: the overlay lights
    /// up exactly the creatures that will be in the action when it is sent.
    #[must_use]
    pub fn is_selected(&self, id: ObjectId) -> bool {
        match &self.mode {
            Mode::Attackers { pairs, .. } => pairs.iter().any(|(a, _)| *a == id),
            Mode::Blockers { pairs, .. } => pairs.iter().any(|(b, _)| *b == id),
            _ => self.selected.contains(&id),
        }
    }

    /// Adds or removes an object from the answer.
    ///
    /// Combat goes through here too, and it is the reason this is not a plain
    /// set toggle. An attack is a *pair* — a creature and what it is sent at —
    /// and so is a block. Tapping a creature during combat used to push it
    /// onto the generic selection list, which `confirm` never reads for these
    /// two modes, so a player could light up their whole board and still
    /// declare no attackers. Now a tap pairs the creature with whatever the
    /// focus is pointing at, and a tap on a defender (or on an attacker, when
    /// blocking) moves the focus instead.
    pub fn toggle(&mut self, id: ObjectId) -> SelectionOutcome {
        match &mut self.mode {
            Mode::Attackers {
                candidates,
                defenders,
                pairs,
                focus,
            } => {
                // A planeswalker is both a thing to attack and, for its
                // controller, a permanent on the board. Here it can only mean
                // "send the next attacker at this", so that comes first.
                if let Some(at) = defenders
                    .iter()
                    .position(|d| *d == Defender::Planeswalker(id))
                {
                    *focus = at;
                    return SelectionOutcome::Added;
                }
                if !candidates.contains(&id) {
                    return SelectionOutcome::Rejected;
                }
                if let Some(pos) = pairs.iter().position(|(a, _)| *a == id) {
                    pairs.remove(pos);
                    return SelectionOutcome::Removed;
                }
                let Some(defender) = defenders.get(*focus).copied() else {
                    return SelectionOutcome::Rejected;
                };
                pairs.push((id, defender));
                SelectionOutcome::Added
            }
            Mode::Blockers {
                candidates,
                options,
                pairs,
                attackers,
                focus,
            } => {
                if let Some(at) = attackers.iter().position(|a| *a == id) {
                    *focus = at;
                    return SelectionOutcome::Added;
                }
                if !candidates.contains(&id) {
                    return SelectionOutcome::Rejected;
                }
                if let Some(pos) = pairs.iter().position(|(b, _)| *b == id) {
                    pairs.remove(pos);
                    return SelectionOutcome::Removed;
                }
                let Some(attacker) = attackers.get(*focus).copied() else {
                    return SelectionOutcome::Rejected;
                };
                // Evasion is a pairing question: a flier is a legal blocker
                // and still not a legal block, so this is refused rather than
                // sent for the engine to bounce.
                if !options
                    .iter()
                    .any(|o| o.blocker == id && o.attackers.contains(&attacker))
                {
                    return SelectionOutcome::Rejected;
                }
                pairs.push((id, attacker));
                SelectionOutcome::Added
            }
            _ => {
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
                    _ => 0,
                };
                if self.selected.len() >= max {
                    return SelectionOutcome::Full;
                }
                self.selected.push(id);
                SelectionOutcome::Added
            }
        }
    }

    /// What a declaration made right now would be pointed at.
    #[must_use]
    pub fn combat_focus(&self) -> CombatFocus {
        match &self.mode {
            Mode::Attackers {
                defenders, focus, ..
            } => defenders
                .get(*focus)
                .copied()
                .map_or(CombatFocus::None, CombatFocus::Defender),
            Mode::Blockers {
                attackers, focus, ..
            } => attackers
                .get(*focus)
                .copied()
                .map_or(CombatFocus::None, CombatFocus::Attacker),
            _ => CombatFocus::None,
        }
    }

    /// Steps the combat focus, wrapping in both directions.
    ///
    /// A pointer can tap the defender it means; a keyboard needs this, and a
    /// two-player game with no planeswalkers never needs either — there is
    /// exactly one thing to attack and the focus starts on it.
    pub fn cycle_focus(&mut self, delta: i32) -> CombatFocus {
        let (len, focus) = match &mut self.mode {
            Mode::Attackers {
                defenders, focus, ..
            } => (defenders.len(), focus),
            Mode::Blockers {
                attackers, focus, ..
            } => (attackers.len(), focus),
            _ => return CombatFocus::None,
        };
        if len > 0 {
            let len = i64::try_from(len).unwrap_or(1);
            let next = (i64::try_from(*focus).unwrap_or(0) + i64::from(delta)).rem_euclid(len);
            *focus = usize::try_from(next).unwrap_or(0);
        }
        self.combat_focus()
    }

    /// Where the combat focus sits, as `(position, count)`.
    ///
    /// `None` outside combat. The overlay uses it to say "2 of 3" so a player
    /// cycling with one key can tell there is more to cycle to; with a count
    /// of one there is nothing to aim and the hint is worth hiding.
    #[must_use]
    pub const fn focus_position(&self) -> Option<(usize, usize)> {
        match &self.mode {
            Mode::Attackers {
                defenders, focus, ..
            } => Some((*focus, defenders.len())),
            Mode::Blockers {
                attackers, focus, ..
            } => Some((*focus, attackers.len())),
            _ => None,
        }
    }

    /// Whether the pending choice is a combat declaration.
    #[must_use]
    pub const fn is_combat(&self) -> bool {
        matches!(self.mode, Mode::Attackers { .. } | Mode::Blockers { .. })
    }

    /// What this creature has been declared against, if anything.
    ///
    /// The overlay draws the assignment from this, so a player can see that
    /// their two attackers are going at different seats before confirming.
    #[must_use]
    pub fn assignment(&self, id: ObjectId) -> Option<CombatFocus> {
        match &self.mode {
            Mode::Attackers { pairs, .. } => pairs
                .iter()
                .find(|(a, _)| *a == id)
                .map(|(_, d)| CombatFocus::Defender(*d)),
            Mode::Blockers { pairs, .. } => pairs
                .iter()
                .find(|(b, _)| *b == id)
                .map(|(_, a)| CombatFocus::Attacker(*a)),
            _ => None,
        }
    }

    /// How many declarations are standing.
    #[must_use]
    pub fn declared(&self) -> usize {
        match &self.mode {
            Mode::Attackers { pairs, .. } => pairs.len(),
            Mode::Blockers { pairs, .. } => pairs.len(),
            _ => self.selected.len(),
        }
    }

    /// Clears the answer being built without sending anything.
    pub fn cancel(&mut self) {
        self.selected.clear();
        self.choice_index = None;
        match &mut self.mode {
            Mode::Attackers { pairs, focus, .. } => {
                pairs.clear();
                *focus = 0;
            }
            Mode::Blockers { pairs, focus, .. } => {
                pairs.clear();
                *focus = 0;
            }
            _ => {}
        }
    }

    /// Declares `attacker` as attacking `defender`.
    ///
    /// Returns `false` when either side is not a candidate, so the caller can
    /// play a rejection cue instead of sending an action that will bounce.
    pub fn declare_attacker(&mut self, attacker: ObjectId, defender: Defender) -> bool {
        let Mode::Attackers {
            candidates,
            defenders,
            pairs,
            ..
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
        let Mode::Blockers { options, pairs, .. } = &mut self.mode else {
            return false;
        };
        // Evasion is a pairing question, so the check is a pairing check:
        // a flier is a legal blocker and still not a legal block.
        if !options
            .iter()
            .any(|o| o.blocker == blocker && o.attackers.contains(&attacker))
        {
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

    /// Adds or removes a seat as a target ("any target", CR 115.4).
    ///
    /// Separate from [`Interaction::toggle`] because a player has no
    /// `ObjectId` to be named by — not because targeting a face is a
    /// different kind of choice. `min`/`max` count across both halves, so
    /// the two must be answered together.
    pub fn toggle_player(&mut self, player: PlayerId) -> SelectionOutcome {
        let Mode::Objects {
            player_options,
            max,
            ..
        } = &self.mode
        else {
            return SelectionOutcome::Rejected;
        };
        if !player_options.contains(&player) {
            return SelectionOutcome::Rejected;
        }
        if let Some(at) = self.selected_players.iter().position(|p| *p == player) {
            self.selected_players.remove(at);
            return SelectionOutcome::Removed;
        }
        if self.selected.len() + self.selected_players.len() >= *max {
            return SelectionOutcome::Rejected;
        }
        self.selected_players.push(player);
        SelectionOutcome::Added
    }

    /// The seats currently chosen as targets.
    #[must_use]
    pub fn selected_players(&self) -> &[PlayerId] {
        &self.selected_players
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
            Mode::Objects { min, .. } => self.selected.len() + self.selected_players.len() >= *min,
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
            Mode::Objects { min, .. }
                if self.selected.len() + self.selected_players.len() >= *min =>
            {
                // `Mode::Objects` also answers mulligan bottoming, a
                // discard and the legend rule, none of which is a
                // `Pending::ChooseTargets` — so the richer action is sent
                // only when a seat was actually picked.
                if self.selected_players.is_empty() {
                    Some(PlayerAction::ChooseObjects {
                        objects: self.selected.clone(),
                    })
                } else {
                    Some(PlayerAction::ChooseTargets {
                        objects: self.selected.clone(),
                        players: self.selected_players.clone(),
                    })
                }
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
        | Pending::ChooseAttackers { player, .. }
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

    /// A seat as a defender.
    fn seat(id: u8) -> Defender {
        Defender::Player(PlayerId::new(id))
    }

    /// The attacker choice with a given list of legal attackers and defenders.
    fn attack_choice(attackers: Vec<ObjectId>, defenders: Vec<Defender>) -> Pending {
        Pending::ChooseAttackers {
            player: me(),
            attackers,
            defenders,
        }
    }

    fn interaction(pending: Pending) -> Interaction {
        Interaction::new(pending, me())
    }

    #[test]
    fn a_choice_addressed_to_another_seat_is_not_actionable() {
        let mut i = interaction(Pending::ChooseTargets {
            player: PlayerId::new(1),
            options: vec![obj(1)],
            player_options: vec![],
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
            player_options: vec![],
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
            player_options: vec![],
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
        let mut i = interaction(attack_choice(vec![obj(1), obj(2)], vec![seat(1)]));

        assert!(!i.declare_attacker(obj(9), seat(1)), "not a candidate");
        assert!(!i.declare_attacker(obj(1), seat(7)), "not a defender");
        assert!(i.declare_attacker(obj(1), seat(1)));

        assert_eq!(
            i.confirm(),
            Some(PlayerAction::DeclareAttackers {
                attackers: vec![(obj(1), seat(1))]
            })
        );
    }

    #[test]
    fn re_declaring_an_attacker_replaces_its_defender_rather_than_duplicating() {
        let mut i = interaction(attack_choice(vec![obj(1)], vec![seat(1), seat(2)]));
        i.declare_attacker(obj(1), seat(1));
        i.declare_attacker(obj(1), seat(2));
        assert_eq!(
            i.confirm(),
            Some(PlayerAction::DeclareAttackers {
                attackers: vec![(obj(1), seat(2))]
            })
        );
    }

    #[test]
    fn declaring_no_attackers_is_a_valid_answer() {
        let i = interaction(attack_choice(vec![obj(1)], vec![seat(1)]));
        assert!(i.can_confirm());
        assert_eq!(
            i.confirm(),
            Some(PlayerAction::DeclareAttackers { attackers: vec![] })
        );
    }

    #[test]
    fn blockers_must_block_an_actual_attacker() {
        let mut i = interaction(Pending::ChooseBlockers {
            player: me(),
            attacker: PlayerId::new(1),
            blockers: vec![BlockOption {
                blocker: obj(10),
                attackers: vec![obj(1)],
            }],
        });
        assert!(!i.declare_blocker(obj(10), obj(99)));
        assert!(i.declare_blocker(obj(10), obj(1)));
        assert_eq!(
            i.confirm(),
            Some(PlayerAction::DeclareBlockers {
                blockers: vec![(obj(10), obj(1))]
            })
        );
    }

    /// A blocking choice with one attacker per listed blocker.
    fn block_choice(options: Vec<BlockOption>) -> Pending {
        Pending::ChooseBlockers {
            player: me(),
            attacker: PlayerId::new(1),
            blockers: options,
        }
    }

    // The bug this whole pairing model exists to close: tapping a creature in
    // combat pushed it onto the generic selection list, which `confirm` never
    // reads for these two modes. A player could light up their entire board
    // and still declare no attackers — the client looked like it had combat
    // and did not.
    #[test]
    fn tapping_a_creature_in_combat_actually_declares_it() {
        let mut i = interaction(attack_choice(vec![obj(1), obj(2)], vec![seat(1)]));
        assert_eq!(i.toggle(obj(1)), SelectionOutcome::Added);
        assert!(i.is_selected(obj(1)), "a declared attacker reads as chosen");
        assert_eq!(i.declared(), 1);
        assert_eq!(
            i.confirm(),
            Some(PlayerAction::DeclareAttackers {
                attackers: vec![(obj(1), seat(1))]
            })
        );
    }

    #[test]
    fn tapping_a_declared_attacker_again_calls_it_off() {
        let mut i = interaction(attack_choice(vec![obj(1)], vec![seat(1)]));
        i.toggle(obj(1));
        assert_eq!(i.toggle(obj(1)), SelectionOutcome::Removed);
        assert!(!i.is_selected(obj(1)));
        assert_eq!(
            i.confirm(),
            Some(PlayerAction::DeclareAttackers { attackers: vec![] })
        );
    }

    #[test]
    fn a_table_with_one_defender_needs_no_aiming_at_all() {
        // The two-player case has to cost nothing: one thing to attack, and
        // the focus already on it before the player touches anything.
        let i = interaction(attack_choice(vec![obj(1)], vec![seat(1)]));
        assert_eq!(i.combat_focus(), CombatFocus::Defender(seat(1)));
    }

    #[test]
    fn attacks_go_where_the_focus_points_and_the_focus_can_be_moved() {
        let mut i = interaction(attack_choice(
            vec![obj(1), obj(2)],
            vec![seat(1), seat(2), Defender::Planeswalker(obj(50))],
        ));
        i.toggle(obj(1));
        assert_eq!(i.cycle_focus(1), CombatFocus::Defender(seat(2)));
        i.toggle(obj(2));
        assert_eq!(
            i.confirm(),
            Some(PlayerAction::DeclareAttackers {
                attackers: vec![(obj(1), seat(1)), (obj(2), seat(2))]
            }),
            "two attackers, two different seats"
        );
        // And it wraps in both directions, so one key is enough to reach
        // every defender at a four-player table.
        assert_eq!(i.cycle_focus(-1), CombatFocus::Defender(seat(1)));
        assert_eq!(
            i.cycle_focus(-1),
            CombatFocus::Defender(Defender::Planeswalker(obj(50))),
            "stepping back past the start wraps round"
        );
    }

    #[test]
    fn tapping_a_planeswalker_aims_at_it() {
        let walker = Defender::Planeswalker(obj(50));
        let mut i = interaction(attack_choice(vec![obj(1)], vec![seat(1), walker]));
        // A pointer should never have to find a cycle key: the thing being
        // attacked is on the table and can be tapped.
        assert_eq!(i.toggle(obj(50)), SelectionOutcome::Added);
        assert_eq!(i.combat_focus(), CombatFocus::Defender(walker));
        i.toggle(obj(1));
        assert_eq!(
            i.confirm(),
            Some(PlayerAction::DeclareAttackers {
                attackers: vec![(obj(1), walker)]
            })
        );
        assert_eq!(i.assignment(obj(1)), Some(CombatFocus::Defender(walker)));
    }

    #[test]
    fn blocks_are_paired_with_the_attacker_in_focus() {
        let mut i = interaction(block_choice(vec![
            BlockOption {
                blocker: obj(10),
                attackers: vec![obj(1), obj(2)],
            },
            BlockOption {
                blocker: obj(11),
                attackers: vec![obj(2)],
            },
        ]));
        assert_eq!(i.combat_focus(), CombatFocus::Attacker(obj(1)));
        i.toggle(obj(10));
        // Tap the second attacker to aim at it, then the blocker for it.
        assert_eq!(i.toggle(obj(2)), SelectionOutcome::Added);
        assert_eq!(i.combat_focus(), CombatFocus::Attacker(obj(2)));
        i.toggle(obj(11));
        assert_eq!(
            i.confirm(),
            Some(PlayerAction::DeclareBlockers {
                blockers: vec![(obj(10), obj(1)), (obj(11), obj(2))]
            })
        );
    }

    #[test]
    fn a_block_the_rules_forbid_is_refused_rather_than_sent() {
        // Evasion is a pairing question — a flier is a legal blocker and
        // still not a legal block — so the client must not send it and wait
        // for the engine to bounce it.
        let mut i = interaction(block_choice(vec![
            BlockOption {
                blocker: obj(10),
                attackers: vec![obj(1)],
            },
            BlockOption {
                blocker: obj(11),
                attackers: vec![obj(2)],
            },
        ]));
        assert_eq!(i.combat_focus(), CombatFocus::Attacker(obj(1)));
        assert_eq!(
            i.toggle(obj(11)),
            SelectionOutcome::Rejected,
            "obj(11) may only block obj(2)"
        );
        assert_eq!(i.declared(), 0);
        // The same creature against the attacker it *can* block goes through.
        i.cycle_focus(1);
        assert_eq!(i.toggle(obj(11)), SelectionOutcome::Added);
    }

    #[test]
    fn calling_off_combat_forgets_the_declarations_and_the_aim() {
        let mut i = interaction(attack_choice(vec![obj(1)], vec![seat(1), seat(2)]));
        i.cycle_focus(1);
        i.toggle(obj(1));
        i.cancel();
        assert_eq!(i.declared(), 0);
        assert_eq!(i.combat_focus(), CombatFocus::Defender(seat(1)));
        assert_eq!(
            i.confirm(),
            Some(PlayerAction::DeclareAttackers { attackers: vec![] })
        );
    }

    #[test]
    fn a_creature_that_cannot_attack_is_refused() {
        let mut i = interaction(attack_choice(vec![obj(1)], vec![seat(1)]));
        assert_eq!(i.toggle(obj(99)), SelectionOutcome::Rejected);
        assert_eq!(i.declared(), 0);
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
            source: None,
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
        let mut i = interaction(attack_choice(vec![obj(1)], vec![seat(1)]));
        i.declare_attacker(obj(1), seat(1));
        i.cancel();
        assert_eq!(
            i.confirm(),
            Some(PlayerAction::DeclareAttackers { attackers: vec![] })
        );
    }

    #[test]
    fn a_face_is_a_target_like_any_other() {
        // "Any target" (CR 115.4) spans objects and players, so one prompt
        // has to be answerable with either — or with both, when it takes two.
        let mut i = interaction(Pending::ChooseTargets {
            player: me(),
            options: vec![obj(1)],
            player_options: vec![PlayerId::new(0), PlayerId::new(1)],
            min: 2,
            max: 2,
        });
        assert_eq!(i.toggle(obj(1)), SelectionOutcome::Added);
        assert!(!i.can_confirm());
        assert_eq!(i.toggle_player(PlayerId::new(1)), SelectionOutcome::Added);
        assert_eq!(i.selected_players(), &[PlayerId::new(1)]);
        assert!(i.can_confirm());
        assert_eq!(
            i.confirm(),
            Some(PlayerAction::ChooseTargets {
                objects: vec![obj(1)],
                players: vec![PlayerId::new(1)],
            })
        );
    }

    #[test]
    fn a_seat_the_spell_cannot_reach_is_refused() {
        // The tab is a camera control the rest of the time, so a rejection
        // here is what lets the click fall through to the camera.
        let mut i = interaction(Pending::ChooseTargets {
            player: me(),
            options: vec![obj(1)],
            player_options: vec![],
            min: 1,
            max: 1,
        });
        assert_eq!(
            i.toggle_player(PlayerId::new(1)),
            SelectionOutcome::Rejected
        );
        i.toggle(obj(1));
        assert_eq!(
            i.confirm(),
            Some(PlayerAction::ChooseObjects {
                objects: vec![obj(1)]
            })
        );
    }

    #[test]
    fn prompt_headlines_are_written_for_a_player_not_a_developer() {
        let i = interaction(Pending::ChooseTargets {
            player: me(),
            options: vec![obj(1)],
            player_options: vec![],
            min: 0,
            max: 2,
        });
        assert_eq!(i.prompt().headline(Lang::En), "Choose up to 2 target(s)");

        let i = interaction(Pending::ChooseNumber {
            player: me(),
            min: 0,
            max: 50,
        });
        assert_eq!(i.prompt().headline(Lang::En), "Choose a number (0–50)");

        let i = interaction(Pending::YesNo {
            player: me(),
            prompt: YesNoPrompt::PayLifeOrEnterTapped { amount: 2 },
            source: None,
        });
        assert_eq!(
            i.prompt().headline(Lang::En),
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
            attack_choice(vec![obj(1)], vec![seat(1)]),
            Pending::ChooseBlockers {
                player: me(),
                attacker: PlayerId::new(1),
                blockers: vec![],
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
                player_options: vec![],
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
                source: None,
            },
        ];
        for pending in all {
            let i = interaction(pending);
            assert!(!i.prompt().headline(Lang::En).is_empty());
        }
    }
}
