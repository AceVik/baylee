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
//! its defender list, because "which planeswalkers may I attack" (CR 506.2)
//! is a rules question and re-deriving it client-side would be a second,
//! divergent implementation of it.

use crate::arrange::{Arrangement, Nudge, Row};
use crate::i18n::{Lang, Phrase, seat_name};
use baylee_core::ids::{CardIndex, Defender, ObjectId, PlayerId, SeatSet, SubtypeId};
use baylee_core::mana::ManaColor;
use baylee_engine::choice::{
    AnswerFault, ArrangePlace, ArrangePrompt, BlockOption, CastModeDesc, ChoicePrompt,
    DamageChoiceId, DamageEffectOption, DamagePartView, LegalActions, NumberPrompt, Pending,
    PlayerAction, TargetPrompt, YesNoPrompt,
};
use baylee_engine::win::{EndReason, GameResult, Victor};
use baylee_view::{GameStatic, HouseAnswer, LossCause, PlayerView, SeatView};

mod attack;
pub use attack::AttackOption;
mod block;
pub use block::BlockRow;
mod ending;
pub use ending::{
    Outcome, ending_reason, loss_lines, loss_phrases, outcome, table_losses, verdict,
};
mod headline;
pub use headline::{counter_label, counter_label_counted, payment_line};

/// What a combat declaration is currently pointed at.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum CombatFocus {
    /// Attacks declared now are sent at this defender.
    Defender(Defender),
    /// Blocks declared now are put in front of this attacker.
    Attacker(ObjectId),
    /// Not a combat choice, or nothing left to point at.
    None,
}

/// The order a click draws members out of a merged card: every one after
/// the first, front to back, and the first last.
///
/// A card is drawn as its group's first member, and what is picked becomes
/// a card of its own, drawn as *its* first. Measured on thirty Elves, taking
/// from the back handed the declared card a new first member on every click
/// (31, then 30, then 29), so the card that had stepped forward was
/// replaced at each one. In this order the card left at home keeps its
/// first member until it is the last one, and the declared card keeps the
/// first it was given: every later pick sorts after it, and a take-back
/// (`members.iter().rev()` in `toggle_group`) removes the newest.
fn pool_order(members: &[ObjectId]) -> impl Iterator<Item = ObjectId> + '_ {
    members.iter().skip(1).chain(members.first()).copied()
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

/// Identity of a versioned decision; unrelated dialog kinds never compare equal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecisionId {
    /// One mandatory resolving mana activation.
    Mana(baylee_engine::choice::ManaChoiceId),
    /// Exact affected incarnation and offered vocabulary.
    Text {
        /// The object whose words change.
        target: baylee_core::ids::DamageSourceRef,
        /// Offered family of words.
        kind: baylee_cards_dsl::TextWordKind,
    },
    /// Damage replacement ordering or allocation.
    Damage(DamageChoiceId),
    /// Exact source incarnation selection.
    Source(baylee_core::ids::SourceChoiceId),
    /// Retargeting context, including the original exact object version.
    Retarget {
        /// Existing target that an empty answer preserves.
        current: baylee_core::ids::TargetRef,
        /// Target position, zero-based.
        index: u32,
        /// Total targets in the copied spell or ability.
        of: u32,
    },
}

/// What the player is being asked, in renderer-friendly terms.
// Not `PartialEq`: two of the variants carry engine types that are plain
// data without an equality impl, and a prompt is displayed rather than
// compared.
#[derive(Clone, Debug)]
pub enum Prompt {
    /// The old and replacement word are separate, explicit selections.
    TextReplacement {
        /// Current two-word draft.
        draft: crate::text_choice::Draft,
    },
    /// Choose one exact legal mana ability; this instruction cannot be passed.
    ChooseManaAbility {
        /// Number of activations already processed by this instruction.
        processed: u64,
        /// Legal source incarnations and ability indices.
        options: Vec<baylee_engine::choice::ManaAbilityChoice>,
    },
    /// Choose one exact source, without targeting it.
    ChooseDamageSource {
        /// Legal incarnations in their offered order.
        options: Vec<baylee_core::ids::DamageSourceRef>,
    },
    /// Select the next effect that modifies damage.
    ChooseDamageEffect {
        /// The simultaneous damage being modified.
        damage: Vec<DamagePartView>,
        /// Offered effects with stable IDs and provenance.
        options: Vec<DamageEffectOption>,
    },
    /// Divide prevention between damage sources.
    AllocatePrevention {
        /// The already selected prevention effect.
        effect: DamageEffectOption,
        /// Damage parts in display order.
        damage: Vec<DamagePartView>,
        /// Exact amount required.
        total: u32,
        /// The current draft, in damage-part order.
        amounts: Vec<u32>,
    },
    /// Nothing is being asked of this seat.
    Waiting {
        /// Who the game is waiting for, when it is waiting for a player.
        on: Option<PlayerId>,
    },
    /// This seat has kept its opening hand and other seats have not: built
    /// by [`Prompt::after_keeping`], never from a pending question.
    Deciding {
        /// The seats still deciding, never empty and never this one.
        others: SeatSet,
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
    /// The seat holds priority. What it may do is
    /// [`Interaction::legal_actions`]: a prompt is asked for several times a
    /// frame, and the copy of the legal actions it used to carry was made
    /// for each and read by nobody (`/allocs`, 08.10.2026).
    Priority,
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
        min: u32,
        /// Maximum.
        max: u32,
        /// Why.
        reason: TargetPrompt,
    },
    /// Choose a creature type, or a basic land type where those are all
    /// that is offered (Phantasmal Terrain).
    ChooseSubtype {
        /// The types on offer, in the engine's order.
        ///
        /// There are about three hundred and fifty of them, which is why
        /// this is the one indexed choice with a filter in front of it.
        options: Vec<SubtypeId>,
    },
    /// Choose a card type (Serra's Emissary).
    ChooseCardType {
        /// The types on offer, in the engine's order.
        options: Vec<baylee_core::types::TypeSet>,
    },
    /// Choose a card name (Pithing Needle).
    ///
    /// No list rides with it: any card's name may be chosen (CR 201.4), so
    /// the pool is the list, and a renderer that links the card pool offers
    /// it narrowed by what the player types. The model hears which card and
    /// face was picked ([`Interaction::choose_card_name`]).
    ChooseCardName {
        /// Eligible printed card types; empty means unrestricted.
        types: baylee_core::types::TypeSet,
        /// Printed card types a named face must not have.
        excluded_types: baylee_core::types::TypeSet,
    },
    /// Choose a colour.
    ChooseColor {
        /// The allowed colours.
        options: Vec<ManaColor>,
    },
    /// Choose a number: X, how many times to pay a replicate cost, or one
    /// target's share of divided damage.
    ChooseNumber {
        /// Lowest legal value.
        min: u32,
        /// Highest legal value.
        max: u32,
        /// What the number counts, which is what the headline says.
        reason: NumberPrompt,
    },
    /// Choose a player.
    ChoosePlayer {
        /// The candidate seats.
        options: Vec<PlayerId>,
    },
    /// Choose how to cast a spell.
    CastMode {
        /// The card being cast or played — or the permanent whose modal
        /// trigger is choosing, which arrives through the same question.
        ///
        /// Carried because two options can be identical in everything a
        /// [`CastModeDesc`] holds and differ only in the name they print: a
        /// pathway's two land faces (CR 712.12) are the same kind at the same
        /// empty cost. This is the handle the label is resolved through.
        object: ObjectId,
        /// The offered options.
        options: Vec<CastModeDesc>,
    },
    /// Choose one of the piles another player separated (Fact or Fiction).
    /// Answered by position, as a cast option is.
    ChoosePile {
        /// The piles, in the engine's order; one may be empty.
        piles: Vec<Vec<ObjectId>>,
        /// Raging River: the attacker the chosen pile is a label for (only
        /// that pile and fliers may block it); `None` for a pile taken into
        /// the hand.
        label: Option<ObjectId>,
    },
    /// Put cards into places, each in an order.
    Arrange {
        /// Why the cards are being arranged.
        reason: ArrangePrompt,
        /// The one place they all go, when there is only one — which is
        /// what says whether the first card named is the top card or not.
        onto: Option<ArrangePlace>,
    },
    /// A yes-or-no question.
    YesNo {
        /// What is being decided.
        question: YesNoPrompt,
    },
    /// The game is over.
    GameOver,
}

/// Whose turn it is, for the one line that has to know.
///
/// A priority window is not a turn: on an opponent's turn a seat still gets
/// asked, and the prompt bar was answering "Your move" — "Du bist dran" —
/// which every player reads as *it is your turn*. Two different sentences
/// for the same question, so the fact has to reach [`Prompt::headline`].
///
/// It is a two-value enum rather than a `bool` because the call site reads
/// `Turn::Theirs` and a `false` reads as nothing at all. The client answers
/// it with `view.active == view.seat`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Turn {
    /// The viewing seat is the active player.
    Mine,
    /// Somebody else is.
    Theirs,
}

impl Turn {
    /// Which one a seat is looking at.
    #[must_use]
    pub fn of(active: PlayerId, seat: PlayerId) -> Self {
        if active == seat {
            Self::Mine
        } else {
            Self::Theirs
        }
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
    TextReplacement(crate::text_choice::Draft),
    Source {
        choice: baylee_core::ids::SourceChoiceId,
        options: Vec<baylee_core::ids::DamageSourceRef>,
    },
    DamageEffect {
        choice: DamageChoiceId,
        options: Vec<DamageEffectOption>,
    },
    Prevention(crate::damage::Allocation),
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
        /// Which candidate the aim keys stand on, indexing `options`
        /// followed by `player_options`.
        ///
        /// The same field combat has, for the same reason: a pointer can tap
        /// the thing it means and a keyboard cannot. It aims at the *first*
        /// half here rather than the second — the spell being cast is the
        /// second half and is already fixed — so walking it moves the cursor
        /// to what a click would pick.
        focus: usize,
    },
    /// Cards put into piles, in an order: `Pending::Arrange`. The whole
    /// answer lives in the [`Arrangement`], which is why this is the one
    /// mode that never touches `picks`.
    Arrange(Arrangement),
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
    Number {
        min: u32,
        max: u32,
    },
    /// One of a fixed set of colours.
    Color {
        options: Vec<ManaColor>,
    },
    /// One of a fixed set of seats.
    Player {
        options: Vec<PlayerId>,
    },
    /// One of a fixed set of cast options.
    CastOption {
        count: usize,
    },
    /// One of a fixed set of creature types.
    ///
    /// Three hundred and fifty of them are offered, which is why the
    /// answer is an index into the list rather than a click on the board:
    /// a creature type is not a thing on the table. Narrowing that list is
    /// the renderer's job; the model only ever hears which row was picked.
    Subtype {
        options: Vec<SubtypeId>,
    },
    /// One of a fixed set of card types.
    CardType {
        options: Vec<baylee_core::types::TypeSet>,
    },
    /// A card name: the card and the face whose name it is, once one is
    /// picked.
    ///
    /// Not an index, because there is no list to index: the pool is the
    /// option set (CR 201.4), and the engine checks the answer against it.
    CardName {
        named: Option<(CardIndex, u8)>,
    },
    /// A yes-or-no answer.
    YesNo,
    /// Priority: an action menu rather than a selection.
    Priority {
        legal: Box<LegalActions>,
    },
    /// Keep or mulligan.
    Mulligan,
    /// The game has ended.
    GameOver,
}

impl Mode {
    /// An object-only selection; player targets use their separate offer.
    fn objects(options: &[ObjectId], min: usize, max: usize) -> Self {
        Self::Objects {
            options: options.to_vec(),
            player_options: Vec::new(),
            min,
            max,
            focus: 0,
        }
    }
}

/// One thing the question can be pointed at.
///
/// A seat is a target like a permanent is ("any target", CR 115.4), and the
/// two used to live in two lists. They are one list now because `Esc` takes
/// back *the last* pick and "the last" has no answer across two of them —
/// and because `min` and `max` were already counted across both, so the
/// bounds were being read off a pair of lists that recorded no order between
/// them.
///
/// It doubles as what the aim keys stand on, which is the same vocabulary
/// one step earlier: a click on the aimed candidate makes exactly this pick.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pick {
    /// A permanent, a card in a zone, or a spell on the stack.
    Object(ObjectId),
    /// A seat, by its face.
    Seat(PlayerId),
}

/// The interaction state for one pending choice.
pub struct Interaction {
    pending: Pending,
    seat: PlayerId,
    mode: Mode,
    /// The answer being built, in the order it was made.
    picks: Vec<Pick>,
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
            choice_index: matches!(mode, Mode::Prevention(_)).then_some(0),
            mode,
            picks: Vec::new(),
            number,
        }
    }

    /// [`Self::new`], keeping a half-built arrangement when the question is
    /// the same one asked again.
    ///
    /// A choice is re-sent whole whenever the view is, and a print entry
    /// earned in the middle of a scry is enough to cause one. For a set of
    /// targets a fresh interaction loses a click; for an arrangement it
    /// loses every move the player has made, so the same seat asked to put
    /// the same cards into the same piles keeps what it built.
    #[must_use]
    pub fn new_keeping(pending: Pending, seat: PlayerId, previous: Option<&Self>) -> Self {
        let mut next = Self::new(pending, seat);
        if let (Mode::Arrange(fresh), Some(Mode::Arrange(built))) = (
            &mut next.mode,
            previous.filter(|p| p.seat == seat).map(|p| &p.mode),
        ) && built.same_question(fresh.dealt(), fresh.specs())
        {
            *fresh = built.clone();
        }
        if let (Mode::Prevention(fresh), Some(old)) =
            (&mut next.mode, previous.filter(|p| p.seat == seat))
            && let Mode::Prevention(built) = &old.mode
            && built.same_offer(fresh)
            && matches!((&next.pending, &old.pending),
                (Pending::AllocatePrevention { effect: a, .. }, Pending::AllocatePrevention { effect: b, .. }) if a == b)
        {
            *fresh = built.clone();
            next.choice_index = Some(built.focus());
        }
        if let Some(old) = previous.filter(|p| p.seat == seat)
            && let (
                Mode::Source {
                    choice: a,
                    options: x,
                },
                Mode::Source {
                    choice: b,
                    options: y,
                },
            ) = (&next.mode, &old.mode)
            && a == b
            && x == y
        {
            next.choice_index = old.choice_index;
        }
        if let Some(old) =
            previous.filter(|old| old.seat == seat && old.decision_id() == next.decision_id())
            && let (Mode::TextReplacement(fresh), Mode::TextReplacement(built)) =
                (&mut next.mode, &old.mode)
        {
            *fresh = built.clone();
            next.choice_index = old.choice_index;
        }
        next
    }

    fn mode_for(pending: &Pending, seat: PlayerId) -> Mode {
        if pending_player(pending) != Some(seat) {
            return if matches!(pending, Pending::GameOver(_)) {
                Mode::GameOver
            } else {
                Mode::Idle
            };
        }
        Self::own_mode(pending)
    }

    fn own_mode(pending: &Pending) -> Mode {
        match pending {
            Pending::ChooseManaAbility { options, .. } => Mode::CastOption {
                count: options.len(),
            },
            Pending::ChooseDamageSource {
                choice, options, ..
            } => Mode::Source {
                choice: *choice,
                options: options.clone(),
            },
            Pending::ChooseDamageEffect {
                choice, options, ..
            } => Mode::DamageEffect {
                choice: *choice,
                options: options.clone(),
            },
            Pending::AllocatePrevention {
                choice,
                damage,
                total,
                ..
            } => Mode::Prevention(crate::damage::Allocation::new(*choice, damage, *total)),
            Pending::Mulligan { .. } => Mode::Mulligan,
            Pending::MulliganBottom { count, .. } | Pending::DiscardChoice { count, .. } => {
                Mode::objects(&[], *count as usize, *count as usize)
            }
            Pending::Priority { legal, .. } => Mode::Priority {
                legal: legal.clone(),
            },
            // The defender list comes from the engine rather than the
            // client: it is the one place that knows which planeswalkers
            // are attackable (CR 506.2), and the engine validates a
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
            // The blocks a requirement asks for (CR 509.1c) start chosen:
            // the engine refuses a declaration obeying fewer of them, and a
            // blocker asked to block two attackers can be given them here
            // and nowhere else.
            Pending::ChooseBlockers {
                blockers, obeying, ..
            } => Mode::Blockers {
                candidates: blockers.iter().map(|b| b.blocker).collect(),
                attackers: ordered_attackers(blockers),
                options: blockers.clone(),
                pairs: obeying.clone(),
                focus: 0,
            },
            Pending::LegendChoice { options, .. } => Mode::objects(options, 1, 1),
            Pending::ChooseCards {
                options, min, max, ..
            } => Mode::objects(options, *min as usize, *max as usize),
            Pending::ChooseTargets {
                options,
                player_options,
                min,
                max,
                ..
            } => Mode::Objects {
                options: options.clone(),
                player_options: player_options.clone(),
                min: usize::try_from(*min).unwrap_or(usize::MAX),
                max: usize::try_from(*max).unwrap_or(usize::MAX),
                focus: 0,
            },
            Pending::Arrange { cards, piles, .. } => Mode::Arrange(Arrangement::new(cards, piles)),
            Pending::ChooseColor { options, .. } => Mode::Color {
                options: options.clone(),
            },
            Pending::ChoosePlayer { options, .. } => Mode::Player {
                options: options.clone(),
            },
            Pending::ChooseNumber {
                reason: NumberPrompt::TextReplacement { kind, .. },
                ..
            } => Mode::TextReplacement(crate::text_choice::Draft::new(*kind)),
            Pending::ChooseNumber { min, max, .. } => Mode::Number {
                min: *min,
                max: *max,
            },
            Pending::ChooseCastMode { options, .. } => Mode::CastOption {
                count: options.len(),
            },
            // A pile is answered by its position, with the same action a
            // cast option is (`PlayerAction::ChooseMode`).
            Pending::ChoosePile { piles, .. } => Mode::CastOption { count: piles.len() },
            Pending::YesNo { .. } => Mode::YesNo,
            // Answered from a list rather than from the board, because a
            // creature type is not a thing on it. `Mode::Idle` stood here
            // until now, which made this the one pending choice a client
            // could not answer at all.
            Pending::ChooseSubtype { options, .. } => Mode::Subtype {
                options: options.clone(),
            },
            Pending::ChooseCardType { options, .. } => Mode::CardType {
                options: options.clone(),
            },
            Pending::ChooseCardName { .. } => Mode::CardName { named: None },
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
            return self.waiting_prompt();
        }
        if let Mode::TextReplacement(draft) = &self.mode {
            return Prompt::TextReplacement {
                draft: draft.clone(),
            };
        }
        match &self.pending {
            Pending::ChooseManaAbility { .. }
            | Pending::ChooseDamageSource { .. }
            | Pending::ChooseDamageEffect { .. }
            | Pending::AllocatePrevention { .. } => self.damage_choice_prompt(),
            Pending::Mulligan {
                taken,
                next_is_free,
                ..
            } => Prompt::Mulligan {
                taken: *taken,
                free: *next_is_free,
            },
            Pending::MulliganBottom { count, .. } => Prompt::BottomCards { count: *count },
            Pending::Priority { .. } => Prompt::Priority,
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
            Pending::ChooseTargets {
                min, max, reason, ..
            } => Prompt::ChooseTargets {
                min: *min,
                max: *max,
                reason: reason.clone(),
            },
            Pending::ChooseSubtype { options, .. } => Prompt::ChooseSubtype {
                options: options.clone(),
            },
            Pending::ChooseCardType { options, .. } => Prompt::ChooseCardType {
                options: options.clone(),
            },
            Pending::ChooseCardName {
                types,
                excluded_types,
                ..
            } => Prompt::ChooseCardName {
                types: *types,
                excluded_types: *excluded_types,
            },
            Pending::ChooseColor { options, .. } => Prompt::ChooseColor {
                options: options.clone(),
            },
            Pending::ChooseNumber {
                min, max, reason, ..
            } => Prompt::ChooseNumber {
                min: *min,
                max: *max,
                reason: *reason,
            },
            Pending::ChoosePlayer { options, .. } => Prompt::ChoosePlayer {
                options: options.clone(),
            },
            Pending::ChooseCastMode {
                object, options, ..
            } => Prompt::CastMode {
                object: *object,
                options: options.clone(),
            },
            Pending::ChoosePile { piles, label, .. } => Prompt::ChoosePile {
                piles: piles.clone(),
                label: *label,
            },
            Pending::Arrange { piles, prompt, .. } => Prompt::Arrange {
                reason: *prompt,
                onto: match piles.as_slice() {
                    [pile] => Some(pile.place),
                    _ => None,
                },
            },
            Pending::YesNo { prompt, .. } => Prompt::YesNo { question: *prompt },
            Pending::GameOver(_) => Prompt::GameOver,
        }
    }

    /// The prompt for a mana-payment, damage or prevention choice.
    fn damage_choice_prompt(&self) -> Prompt {
        match &self.pending {
            Pending::ChooseManaAbility {
                options, choice, ..
            } => Prompt::ChooseManaAbility {
                processed: choice.step,
                options: options.clone(),
            },
            Pending::ChooseDamageSource { options, .. } => Prompt::ChooseDamageSource {
                options: options.clone(),
            },
            Pending::ChooseDamageEffect {
                damage, options, ..
            } => Prompt::ChooseDamageEffect {
                damage: damage.clone(),
                options: options.clone(),
            },
            Pending::AllocatePrevention {
                effect,
                damage,
                total,
                ..
            } => self.prevention_prompt(effect, damage, *total),
            _ => unreachable!("only damage choices use this prompt"),
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
            Mode::Objects { max: 0, .. } => &[],
            Mode::Objects { options, .. } => options,
            Mode::Arrange(arrangement) => arrangement.dealt(),
            Mode::Attackers { candidates, .. } | Mode::Blockers { candidates, .. } => candidates,
            _ => &[],
        }
    }

    /// How many objects the answer wants: the fewest and the most.
    ///
    /// The tally on the zone browser's dialog is what needs this — "1 of up
    /// to 2 chosen" — and so is the footer, because **Cancel exists exactly
    /// when the minimum is zero**. There is no cancel action on the wire: a
    /// question that will accept an empty answer is answered by sending one,
    /// and a question that will not has no way out to offer. A dialog that
    /// drew the button anyway would be promising something the engine cannot
    /// deliver.
    ///
    /// `None` for every choice that is not a set of objects. An ordering
    /// wants all of them, which is a bound rather than a special case.
    #[must_use]
    pub fn bounds(&self) -> Option<(usize, usize)> {
        match &self.mode {
            Mode::Objects { min, max, .. } => Some((*min, *max)),
            Mode::Arrange(arrangement) => {
                Some((arrangement.dealt().len(), arrangement.dealt().len()))
            }
            _ => None,
        }
    }

    /// Whether the answer is an ordering rather than a set.
    ///
    /// The one thing outside this module that has to know: an ordering is
    /// answered by clicking every offered card in the order it should go,
    /// so a renderer draws a place number beside each pick, and the zone
    /// browser opens for it even when every card is already on the table.
    #[must_use]
    pub fn is_ordering(&self) -> bool {
        matches!(self.mode, Mode::Arrange(_))
    }

    /// Whether an object may be selected.
    ///
    /// Choices whose options the engine leaves implicit (the seat's own hand)
    /// accept anything; every enumerated choice accepts only what was offered.
    #[must_use]
    pub fn is_selectable(&self, id: ObjectId) -> bool {
        match &self.mode {
            Mode::Objects { options, max, .. } => {
                *max > 0 && (options.is_empty() || options.contains(&id))
            }
            Mode::Arrange(arrangement) => arrangement.dealt().contains(&id),
            // Combat accepts both halves of a pair: the creature being
            // declared, and the thing it is being declared against — tapping
            // a planeswalker or an attacker aims the next declaration.
            Mode::Attackers {
                candidates,
                defenders,
                ..
            } => {
                candidates.contains(&id)
                    || (defenders.contains(&Defender::Planeswalker(id))
                        || defenders.contains(&Defender::Battle(id)))
            }
            // Blocking is the one case where aiming changes what is offered.
            // `BlockOption` is per blocker, so a flier in the focus leaves the
            // ground unable to answer, and lighting every candidate there
            // invites a click `toggle` then refuses. Re-read from the focus,
            // it costs one lookup and never lies.
            Mode::Blockers {
                options,
                attackers,
                focus,
                ..
            } => {
                attackers.contains(&id)
                    || attackers.get(*focus).is_some_and(|at| {
                        options
                            .iter()
                            .any(|o| o.blocker == id && o.attackers.contains(at))
                    })
            }
            _ => false,
        }
    }

    /// Whether the question this seat is answering picks among objects and
    /// leaves `id` out: neither something it may take nor something already
    /// taken. The table dims such a card for as long as the question is open
    /// (the owner, 13.10.2026), beside the light on the ones it will take.
    ///
    /// Only this seat's own question, so another player's choice dims
    /// nothing at this table. Combat counts both halves of a pair, as
    /// [`Self::is_selectable`] does: the creatures a declaration may take
    /// and what they are declared against, and in a block every creature
    /// that may block some attacker, not only the one in aim, so a blocker
    /// does not go dark and light again as the aim walks. A question whose
    /// options are left implicit (a discard from the hand) or that names no
    /// object at all leaves nothing out; nor does an ordering, whose cards
    /// lie in the zone dialog.
    #[must_use]
    pub fn leaves_out(&self, id: ObjectId) -> bool {
        if !self.is_mine() || self.is_selected(id) {
            return false;
        }
        match &self.mode {
            Mode::Objects { options, max, .. } => {
                *max > 0 && !options.is_empty() && !options.contains(&id)
            }
            Mode::Attackers {
                candidates,
                defenders,
                ..
            } => {
                !(candidates.contains(&id)
                    || defenders.contains(&Defender::Planeswalker(id))
                    || defenders.contains(&Defender::Battle(id)))
            }
            Mode::Blockers {
                candidates,
                attackers,
                ..
            } => !candidates.contains(&id) && !attackers.contains(&id),
            _ => false,
        }
    }

    /// The current selection, in the order it was made.
    ///
    /// Objects only; [`Self::selected_players`] is the other half and
    /// [`Self::picks`] is both in one order.
    pub fn selected(&self) -> impl Iterator<Item = ObjectId> + '_ {
        self.picks.iter().filter_map(|p| match p {
            Pick::Object(id) => Some(*id),
            Pick::Seat(_) => None,
        })
    }

    /// Every pick, objects and seats together, in the order they were made.
    #[must_use]
    pub fn picks(&self) -> &[Pick] {
        &self.picks
    }

    /// How many picks stand, counting seats.
    ///
    /// The number `min` and `max` are compared against, and the one the slip
    /// says "2 of 3" with.
    #[must_use]
    pub fn pick_count(&self) -> usize {
        self.picks.len()
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
            // Selected is "in hand": the card a tap has taken up to move.
            Mode::Arrange(arrangement) => arrangement.held() == Some(id),
            _ => self.picks.contains(&Pick::Object(id)),
        }
    }

    /// Whether a seat is part of the answer being built.
    #[must_use]
    pub fn is_seat_selected(&self, player: PlayerId) -> bool {
        self.picks.contains(&Pick::Seat(player))
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
            // Tap, then place: the first tap takes a card up and the next
            // puts it down in front of the card tapped.
            Mode::Arrange(arrangement) => arrangement.toggle(id),
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
                    .position(|d| matches!(d, Defender::Planeswalker(object) | Defender::Battle(object) if *object == id))
                {
                    *focus = at;
                    return SelectionOutcome::Added;
                }
                if !candidates.contains(&id) {
                    return SelectionOutcome::Rejected;
                }
                let Some(defender) = defenders.get(*focus).copied() else {
                    return SelectionOutcome::Rejected;
                };
                if let Some(pos) = pairs.iter().position(|(a, _)| *a == id) {
                    // Sent somewhere else, the press sends it where the aim
                    // is now (report 24): a player who sent a creature, aimed
                    // at a planeswalker and pressed the creature again meant
                    // "attack that", and the press took it out of the attack
                    // altogether. Only one aimed where it already goes comes
                    // back out.
                    if pairs[pos].1 == defender {
                        pairs.remove(pos);
                        return SelectionOutcome::Removed;
                    }
                    pairs[pos].1 = defender;
                    return SelectionOutcome::Added;
                }
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
                if let Mode::Objects { options, focus, .. } = &mut self.mode
                    && let Some(at) = options.iter().position(|candidate| *candidate == id)
                {
                    *focus = at;
                }
                if let Some(pos) = self.picks.iter().position(|p| *p == Pick::Object(id)) {
                    self.picks.remove(pos);
                    return SelectionOutcome::Removed;
                }
                if self.picks.len() >= self.capacity() {
                    // A choice of one takes the newer click: picking another
                    // card replaces the one picked (owner, beta.7), rather
                    // than asking for the first to be taken back.
                    if self.capacity() != 1 {
                        return SelectionOutcome::Full;
                    }
                    self.picks.clear();
                }
                self.picks.push(Pick::Object(id));
                SelectionOutcome::Added
            }
        }
    }

    /// How many picks this choice will take.
    fn capacity(&self) -> usize {
        match &self.mode {
            Mode::Objects { max, .. } => *max,
            Mode::Arrange(arrangement) => arrangement.dealt().len(),
            _ => 0,
        }
    }

    /// Answers a click on a card that stands for several identical ones.
    ///
    /// A counted stack takes clicks the way a card takes one: each click
    /// picks the next member that has not been picked, and once none is left
    /// the click takes the last one back — which on a stack of one is exactly
    /// the toggle it always was. There is no count stepper anywhere in the
    /// client, and this is why there needs not to be.
    ///
    /// Which four of the forty Soldiers is the engine's question, not the
    /// player's: they are identical, so the only thing a player can mean by
    /// clicking the stack four times is "four of these".
    ///
    /// In [`pool_order`]: the card drawn for a stack is its first member
    /// (`board::CardGroup::representative`), and a pick splits the stack
    /// (`board::Proposal`), so a pick has to leave both cards drawn as the
    /// member they were drawn as before it.
    pub fn toggle_group(&mut self, members: &[ObjectId]) -> SelectionOutcome {
        if let Some(next) =
            pool_order(members).find(|id| self.is_selectable(*id) && !self.is_selected(*id))
        {
            return self.toggle(next);
        }
        // Nothing left to pick, so the click takes one back — the newest pick
        // this stack made, so an over-shot click is undone where it was made
        // rather than wherever `Esc` happens to reach.
        if let Some(at) = self
            .picks
            .iter()
            .rposition(|p| matches!(p, Pick::Object(id) if members.contains(id)))
        {
            self.picks.remove(at);
            return SelectionOutcome::Removed;
        }
        // Combat keeps its declarations as pairs rather than picks, so it
        // takes one back through the same door it made it with.
        match members.iter().rev().find(|id| self.is_selected(**id)) {
            Some(id) => self.toggle(*id),
            None => SelectionOutcome::Rejected,
        }
    }

    /// Answers a gesture that means the whole card: every member at once.
    ///
    /// Adds every member that can still be added, in [`Self::toggle_group`]'s
    /// order, and stops at the first one refused — an answer that is full,
    /// a block the focus cannot take — so a partial pool is what was asked
    /// for, cut where the rules cut it. When none can be added, it takes
    /// every picked member back. What it answers is what happened to the
    /// first member it touched, which is the outcome a player is told about.
    pub fn toggle_all(&mut self, members: &[ObjectId]) -> SelectionOutcome {
        let open: Vec<ObjectId> = pool_order(members)
            .filter(|id| self.is_selectable(*id) && !self.is_selected(*id))
            .collect();
        if let Some((&first, rest)) = open.split_first() {
            let outcome = self.toggle(first);
            if outcome == SelectionOutcome::Added {
                for id in rest {
                    if self.toggle(*id) != SelectionOutcome::Added {
                        break;
                    }
                }
            }
            return outcome;
        }
        let picked: Vec<ObjectId> = members
            .iter()
            .copied()
            .filter(|id| self.is_selected(*id))
            .collect();
        if picked.is_empty() {
            return SelectionOutcome::Rejected;
        }
        for id in picked {
            self.toggle(id);
        }
        SelectionOutcome::Removed
    }

    /// Takes back the last pick, leaving the rest of the answer standing.
    ///
    /// `Esc` used to be [`Self::cancel`] here, which wipes a half-built
    /// answer whole — expensive for a mis-click on the third of five targets
    /// and, with no `PlayerAction::Cancel` to send, not even a way out of the
    /// question. One pick at a time is what a player means by "no, not that
    /// one". Returns what was taken back, or `None` when nothing was picked.
    pub fn take_back(&mut self) -> Option<Pick> {
        if let Mode::Arrange(arrangement) = &mut self.mode {
            let held = arrangement.held()?;
            arrangement.cancel();
            return Some(Pick::Object(held));
        }
        self.picks.pop()
    }

    /// The arrangement being built, for a `Pending::Arrange`.
    #[must_use]
    pub const fn arrangement(&self) -> Option<&Arrangement> {
        match &self.mode {
            Mode::Arrange(arrangement) => Some(arrangement),
            _ => None,
        }
    }

    /// Moves the held card of an arrangement one step. `false` when there
    /// is no arrangement, nothing is held, or there is nowhere to go.
    pub fn nudge(&mut self, nudge: Nudge) -> bool {
        match &mut self.mode {
            Mode::Arrange(arrangement) => arrangement.nudge(nudge),
            _ => false,
        }
    }

    /// Puts the held card of an arrangement at the end of `row`. `false`
    /// when there is no arrangement, nothing is held, or the row is full.
    pub fn place_held(&mut self, row: Row) -> bool {
        match &mut self.mode {
            Mode::Arrange(arrangement) => arrangement.place(row),
            _ => false,
        }
    }

    /// A card's one-based place in the ordered pile it is in, for the
    /// number drawn beside it.
    ///
    /// `None` outside an arrangement, for a card in no pile yet, and for a
    /// card in a pile whose order is not the player's to choose — a number
    /// there would be claiming an order that nobody reads.
    #[must_use]
    pub fn arrange_place(&self, id: ObjectId) -> Option<usize> {
        let arrangement = self.arrangement()?;
        match arrangement.slot(id)? {
            (Row::Pile(pile), at) if arrangement.specs().get(pile)?.ordered => Some(at + 1),
            _ => None,
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
    pub fn cycle_focus(&mut self, delta: i32) -> Option<Pick> {
        let (len, focus) = match &mut self.mode {
            Mode::Attackers {
                defenders, focus, ..
            } => (defenders.len(), focus),
            Mode::Blockers {
                attackers, focus, ..
            } => (attackers.len(), focus),
            // A target prompt walks the offer itself. Same keys, because to
            // a player it is the same gesture — "the next one" — and the
            // keymap stores a chord per action, not per question.
            Mode::Objects {
                options,
                player_options,
                focus,
                ..
            } => (options.len() + player_options.len(), focus),
            // The arrangement walks its own rows, because its focus is a
            // card and the order of the walk changes as cards move.
            Mode::Arrange(arrangement) => {
                return arrangement.cycle_focus(delta).map(Pick::Object);
            }
            _ => return None,
        };
        if len > 0 {
            let len = i64::try_from(len).unwrap_or(1);
            let next = (i64::try_from(*focus).unwrap_or(0) + i64::from(delta)).rem_euclid(len);
            *focus = usize::try_from(next).unwrap_or(0);
        }
        self.aim()
    }

    /// Ticks whatever the focus is standing on.
    ///
    /// The other half of [`Self::cycle_focus`], and the pair is what a
    /// keyboard has instead of a pointer: one walks the offer, this acts on
    /// where it stopped. Combat has had the walk since the focus existed and
    /// never had this, because there the *declaration* keys do the acting.
    ///
    /// `Rejected` when the focus stands on nothing, which is every mode that
    /// has no focus to begin with.
    pub fn toggle_focused(&mut self) -> SelectionOutcome {
        match self.aim() {
            Some(Pick::Object(id)) => self.toggle(id),
            Some(Pick::Seat(player)) => self.toggle_player(player),
            None => SelectionOutcome::Rejected,
        }
    }

    /// What the aim keys are standing on.
    ///
    /// Combat aims at the *second* half of the pair — the defender an attack
    /// would go at, the attacker a block would go in front of — and a target
    /// prompt at the first, because there the second half is the spell and is
    /// already fixed. Both are "what the next click means", which is the only
    /// thing a highlight can say.
    #[must_use]
    pub fn aim(&self) -> Option<Pick> {
        match &self.mode {
            Mode::Attackers { .. } | Mode::Blockers { .. } => match self.combat_focus() {
                CombatFocus::Defender(Defender::Player(p)) => Some(Pick::Seat(p)),
                CombatFocus::Defender(Defender::Planeswalker(id) | Defender::Battle(id))
                | CombatFocus::Attacker(id) => Some(Pick::Object(id)),
                CombatFocus::None => None,
            },
            Mode::Objects {
                options,
                player_options,
                focus,
                ..
            } => options.get(*focus).copied().map(Pick::Object).or_else(|| {
                player_options
                    .get(focus.saturating_sub(options.len()))
                    .copied()
                    .map(Pick::Seat)
            }),
            Mode::Arrange(arrangement) => arrangement.focused().map(Pick::Object),
            _ => None,
        }
    }

    /// Where the aim sits, as `(position, count)`.
    ///
    /// `None` for a choice with nothing to aim. The overlay uses it to say
    /// "2 of 3" so a player cycling with one key can tell there is more to
    /// cycle to; with a count of one there is nothing to aim and the hint is
    /// worth hiding.
    #[must_use]
    pub fn focus_position(&self) -> Option<(usize, usize)> {
        match &self.mode {
            Mode::Attackers {
                defenders, focus, ..
            } => Some((*focus, defenders.len())),
            Mode::Blockers {
                attackers, focus, ..
            } => Some((*focus, attackers.len())),
            Mode::Objects {
                options,
                player_options,
                focus,
                ..
            } => Some((*focus, options.len() + player_options.len())),
            Mode::Arrange(arrangement) => arrangement.focus_position(),
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

    /// Every declaration made so far, as `(creature, what it was declared
    /// against)`.
    ///
    /// [`Self::assignment`] answers for one creature, which is the right
    /// question when drawing a card and the wrong one when drawing the whole
    /// fight: asking it per object would walk the board to find the handful
    /// of pairs that are actually here. Empty outside a combat declaration.
    #[must_use]
    pub fn assignments(&self) -> Vec<(ObjectId, CombatFocus)> {
        match &self.mode {
            Mode::Attackers { pairs, .. } => pairs
                .iter()
                .map(|(a, d)| (*a, CombatFocus::Defender(*d)))
                .collect(),
            Mode::Blockers { pairs, .. } => pairs
                .iter()
                .map(|(b, a)| (*b, CombatFocus::Attacker(*a)))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// How many declarations are standing.
    #[must_use]
    pub fn declared(&self) -> usize {
        match &self.mode {
            Mode::Attackers { pairs, .. } => pairs.len(),
            Mode::Blockers { pairs, .. } => pairs.len(),
            _ => self.picks.len(),
        }
    }

    /// Clears the answer being built without sending anything.
    ///
    /// Wholesale, and still the right word for combat, where `O` means "no
    /// attacks" and is a real answer. For a target prompt [`Self::take_back`]
    /// is what `Esc` reaches instead.
    pub fn cancel(&mut self) {
        self.picks.clear();
        self.choice_index = None;
        match &mut self.mode {
            Mode::TextReplacement(draft) => *draft = crate::text_choice::Draft::new(draft.kind),
            Mode::Attackers { pairs, focus, .. } => {
                pairs.clear();
                *focus = 0;
            }
            Mode::Blockers { pairs, focus, .. } => {
                pairs.clear();
                *focus = 0;
            }
            // Lets go of a held card first and only then puts every card
            // back, so `Esc` in the middle of a move undoes the move and not
            // the arrangement built before it.
            Mode::Arrange(arrangement) => {
                arrangement.cancel();
            }
            Mode::CardName { named } => *named = None,
            Mode::Prevention(allocation) => {
                allocation.clear();
                self.choice_index = Some(allocation.focus());
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
        if let Mode::Prevention(allocation) = &mut self.mode {
            return allocation.set_number(value);
        }
        if let Mode::Number { min, max, .. } = &self.mode {
            self.number = value.clamp(*min, *max);
        }
        self.number
    }

    /// Lowers the top of an X question (CR 107.3) to `cap`, the most the
    /// player's own mana pays: the range the question shows and the one
    /// [`Interaction::set_number`] clamps to. Never below the engine's
    /// minimum and never above its maximum, and only for `NumberPrompt::X`
    /// — every other number is the engine's to bound.
    pub fn cap_number(&mut self, cap: u32) {
        let Pending::ChooseNumber {
            min,
            max,
            reason: NumberPrompt::X,
            ..
        } = &mut self.pending
        else {
            return;
        };
        *max = (*max).min(cap).max(*min);
        let (min, max) = (*min, *max);
        if let Mode::Number { max: shown, .. } = &mut self.mode {
            *shown = max;
        }
        self.number = self.number.clamp(min, max);
    }

    /// The currently chosen number.
    #[must_use]
    pub fn number(&self) -> u32 {
        self.allocation()
            .map_or(self.number, crate::damage::Allocation::number)
    }

    /// The prevention draft, when this seat is assigning shares.
    #[must_use]
    pub const fn allocation(&self) -> Option<&crate::damage::Allocation> {
        if let Mode::Prevention(allocation) = &self.mode {
            Some(allocation)
        } else {
            None
        }
    }

    /// Identity used to reject clicks from a replaced versioned decision.
    #[must_use]
    pub const fn decision_id(&self) -> Option<DecisionId> {
        match &self.pending {
            Pending::ChooseManaAbility { choice, .. } => Some(DecisionId::Mana(*choice)),
            Pending::ChooseNumber {
                reason: NumberPrompt::TextReplacement { target, kind },
                ..
            } => Some(DecisionId::Text {
                target: *target,
                kind: *kind,
            }),
            Pending::ChooseDamageEffect { choice, .. }
            | Pending::AllocatePrevention { choice, .. } => Some(DecisionId::Damage(*choice)),
            Pending::ChooseDamageSource { choice, .. } => Some(DecisionId::Source(*choice)),
            Pending::ChooseTargets {
                reason: TargetPrompt::Retarget { current, index, of },
                ..
            } => Some(DecisionId::Retarget {
                current: *current,
                index: *index,
                of: *of,
            }),
            _ => None,
        }
    }

    /// Whether direct numeric input edits this question.
    #[must_use]
    pub const fn edits_number(&self) -> bool {
        matches!(self.mode, Mode::Number { .. } | Mode::Prevention(_))
    }

    /// Adds or removes a seat as a target ("any target", CR 115.4).
    ///
    /// Separate from [`Interaction::toggle`] because a player has no
    /// `ObjectId` to be named by — not because targeting a face is a
    /// different kind of choice. `min`/`max` count across both halves, so
    /// the two must be answered together.
    pub fn toggle_player(&mut self, player: PlayerId) -> SelectionOutcome {
        let Mode::Objects {
            options,
            player_options,
            max,
            focus,
            ..
        } = &mut self.mode
        else {
            return SelectionOutcome::Rejected;
        };
        let Some(at) = player_options
            .iter()
            .position(|candidate| *candidate == player)
        else {
            return SelectionOutcome::Rejected;
        };
        *focus = options.len() + at;
        let max = *max;
        if let Some(at) = self.picks.iter().position(|p| *p == Pick::Seat(player)) {
            self.picks.remove(at);
            return SelectionOutcome::Removed;
        }
        if self.picks.len() >= max {
            // One to choose: the newer click replaces the pick.
            if max != 1 {
                return SelectionOutcome::Full;
            }
            self.picks.clear();
        }
        self.picks.push(Pick::Seat(player));
        SelectionOutcome::Added
    }

    /// The seats currently chosen as targets, in the order they were picked.
    pub fn selected_players(&self) -> impl Iterator<Item = PlayerId> + '_ {
        self.picks.iter().filter_map(|p| match p {
            Pick::Object(_) => None,
            Pick::Seat(id) => Some(*id),
        })
    }

    /// Picks an indexed option (a cast mode, a colour, or a seat).
    ///
    /// Returns `false` when the index is not one the engine offered.
    pub fn choose_index(&mut self, index: usize) -> bool {
        if let Mode::TextReplacement(draft) = &mut self.mode {
            if !draft.select(index) {
                return false;
            }
            self.choice_index = Some(index);
            return true;
        }
        if let Mode::Prevention(allocation) = &mut self.mode {
            if !allocation.select(index) {
                return false;
            }
            self.choice_index = Some(index);
            return true;
        }
        let count = match &self.mode {
            Mode::CastOption { count } => *count,
            Mode::DamageEffect { options, .. } => options.len(),
            Mode::Source { options, .. } => options.len(),
            Mode::Color { options } => options.len(),
            Mode::Player { options } => options.len(),
            Mode::Subtype { options } => options.len(),
            Mode::CardType { options } => options.len(),
            _ => 0,
        };
        if index >= count {
            return false;
        }
        self.choice_index = Some(index);
        true
    }

    /// Names face `face` of `card` for a card-name question (Pithing
    /// Needle).
    ///
    /// Returns `false` when no card name is being asked. Whether the pool
    /// has that card and the card that face is the engine's to say, against
    /// the pool it plays with; this model links no pool to ask.
    pub fn choose_card_name(&mut self, card: CardIndex, face: u8) -> bool {
        let Mode::CardName { named } = &mut self.mode else {
            return false;
        };
        *named = Some((card, face));
        true
    }

    /// The card name picked so far, as its card and face.
    #[must_use]
    pub const fn chosen_card_name(&self) -> Option<(CardIndex, u8)> {
        match &self.mode {
            Mode::CardName { named } => *named,
            _ => None,
        }
    }

    /// Which row of an indexed choice is picked, if one is.
    ///
    /// A colour, a seat, a cast option and a creature type are all answered
    /// by position, and a chooser that cannot show which position is picked
    /// is a row of identical buttons.
    #[must_use]
    pub const fn chosen_index(&self) -> Option<usize> {
        self.choice_index
    }

    /// Whether the current answer is complete enough to submit, and inside
    /// every bound the question states ([`Self::answer_fault`]).
    #[must_use]
    pub fn can_confirm(&self) -> bool {
        let complete = match &self.mode {
            Mode::Prevention(allocation) => allocation.answer().is_some(),
            Mode::Objects { min, .. } => self.picks.len() >= *min,
            Mode::Arrange(arrangement) => arrangement.answer().is_some(),
            // Declaring nothing is always legal (no attacks, no blocks), a
            // number always has its clamped value, and priority can always be
            // passed — all four are answerable the moment they are asked.
            Mode::Attackers { .. }
            | Mode::Blockers { .. }
            | Mode::Number { .. }
            | Mode::Priority { .. } => true,
            Mode::Color { .. }
            | Mode::Player { .. }
            | Mode::CastOption { .. }
            | Mode::Source { .. }
            | Mode::DamageEffect { .. }
            | Mode::Subtype { .. }
            | Mode::CardType { .. } => self.choice_index.is_some(),
            Mode::TextReplacement(draft) => draft.answer().is_some(),
            Mode::CardName { named } => named.is_some(),
            Mode::Mulligan | Mode::YesNo | Mode::Idle | Mode::GameOver => false,
        };
        complete && self.answer_fault().is_none()
    }

    /// Why the answer picked so far is one the engine refuses, by a bound
    /// the question states (`Pending::answer_fault`): creatures short of a
    /// crew's total power, one creature blocking an attacker with menace.
    /// `None` while the answer is incomplete or keeps every bound; the
    /// table shows the reason beside a confirm it holds back.
    #[must_use]
    pub fn answer_fault(&self) -> Option<AnswerFault> {
        self.answer()
            .and_then(|answer| self.pending.answer_fault(&answer))
    }

    /// Builds the action for the current answer, if it is complete and
    /// inside every bound the question states: the engine refuses any
    /// other, so none is sent.
    #[must_use]
    pub fn confirm(&self) -> Option<PlayerAction> {
        if !self.is_mine() {
            return None;
        }
        self.answer()
            .filter(|answer| self.pending.answer_fault(answer).is_none())
    }

    /// The action the current picks make, once they are complete by their
    /// own mode; [`Self::confirm`] less the checks on who is asked and on
    /// the question's bounds.
    fn answer(&self) -> Option<PlayerAction> {
        match &self.mode {
            Mode::TextReplacement(draft) => draft.answer(),
            Mode::Objects { min, .. } if self.picks.len() >= *min => {
                let objects: Vec<ObjectId> = self.selected().collect();
                let players: Vec<PlayerId> = self.selected_players().collect();
                // `Mode::Objects` also answers mulligan bottoming, a
                // discard and the legend rule, none of which is a
                // `Pending::ChooseTargets` — so the richer action is sent
                // only when a seat was actually picked.
                if players.is_empty() {
                    Some(PlayerAction::ChooseObjects { objects })
                } else {
                    Some(PlayerAction::ChooseTargets { objects, players })
                }
            }
            Mode::Arrange(arrangement) => arrangement
                .answer()
                .map(|piles| PlayerAction::Arrange { piles }),
            Mode::Attackers { pairs, .. } => Some(PlayerAction::DeclareAttackers {
                attackers: pairs.clone(),
            }),
            Mode::Blockers { pairs, .. } => Some(PlayerAction::DeclareBlockers {
                blockers: pairs.clone(),
            }),
            Mode::Source { choice, options } => {
                options.get(self.choice_index?).copied().map(|source| {
                    PlayerAction::ChooseDamageSource {
                        choice: *choice,
                        source,
                    }
                })
            }
            Mode::Prevention(allocation) => allocation.answer(),
            Mode::DamageEffect { choice, options } => {
                options
                    .get(self.choice_index?)
                    .map(|effect| PlayerAction::ChooseDamageEffect {
                        choice: *choice,
                        effect: effect.id,
                    })
            }
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
                if let Pending::ChooseManaAbility {
                    choice, options, ..
                } = &self.pending
                {
                    return options
                        .get(index)
                        .map(|option| PlayerAction::ChooseManaAbility {
                            choice: *choice,
                            source: option.source,
                            ability_index: option.ability_index,
                        });
                }
                (index < *count).then_some(PlayerAction::ChooseMode(index))
            }
            Mode::Subtype { options } => options
                .get(self.choice_index?)
                .copied()
                .map(PlayerAction::ChooseSubtype),
            Mode::CardType { options } => options
                .get(self.choice_index?)
                .copied()
                .map(PlayerAction::ChooseCardType),
            Mode::CardName { named } => {
                named.map(|(card, face)| PlayerAction::ChooseCardName { card, face })
            }
            _ => None,
        }
    }

    /// Answers a yes-or-no question.
    #[must_use]
    pub fn answer_yes_no(&self, yes: bool) -> Option<PlayerAction> {
        matches!(self.mode, Mode::YesNo).then_some(PlayerAction::YesNo(yes))
    }

    /// Answers a mulligan decision; `None` for a further mulligan the
    /// question says may not be taken (CR 103.5, `Pending::Mulligan::can_take`).
    #[must_use]
    pub fn answer_mulligan(&self, keep: bool) -> Option<PlayerAction> {
        let answer = if keep {
            PlayerAction::MulliganKeep
        } else {
            PlayerAction::MulliganTake
        };
        (matches!(self.mode, Mode::Mulligan) && self.pending.answer_fault(&answer).is_none())
            .then_some(answer)
    }

    /// The legal actions offered with priority, if this is a priority choice.
    #[must_use]
    pub fn legal_actions(&self) -> Option<&LegalActions> {
        match &self.mode {
            Mode::Priority { legal } => Some(legal),
            _ => None,
        }
    }

    /// Builds the action for suspending a card, rejecting anything the engine
    /// did not list as suspendable.
    ///
    /// Deliberately not part of [`Self::play_card`]. Suspending is the one
    /// thing a card can do from a hand that is *not* playing it — "rather
    /// than cast this card from your hand, pay {U} and exile it" — so a card
    /// that is both castable and suspendable has two answers and the caller
    /// has to pick, which is what an ability chooser is for. Folding it in
    /// would have made that choice silently.
    ///
    /// Nothing built this action before, which is why `legal.suspendable`
    /// appeared in the client only as an empty vector in test fixtures: a
    /// suspend card in hand was unreachable by mouse and by keyboard alike.
    #[must_use]
    pub fn suspend(&self, card: ObjectId) -> Option<PlayerAction> {
        self.legal_actions()?
            .suspendable
            .contains(&card)
            .then_some(PlayerAction::Suspend { card })
    }

    /// Builds foretell's special action, rejecting anything the engine did not
    /// list as foretellable. Like suspend, this is deliberately not a cast.
    #[must_use]
    pub fn foretell(&self, card: ObjectId) -> Option<PlayerAction> {
        self.legal_actions()?
            .foretellable
            .contains(&card)
            .then_some(PlayerAction::Foretell { card })
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

    /// Whether playing this card is a land drop and nothing else.
    ///
    /// The question the one-click rule turns on. A card the engine offers in
    /// *both* lists is a modal double-faced card with a spell on the front and
    /// a land on the back, and [`Self::play_card`] checks lands first — so a
    /// one-click would resolve it to "play as land" every time and the front
    /// face would be unreachable by mouse. Such a card is therefore not
    /// one-click; it opens a chooser like anything else with two answers.
    #[must_use]
    pub fn plays_only_as_a_land(&self, card: ObjectId) -> bool {
        self.legal_actions()
            .is_some_and(|legal| legal.lands.contains(&card) && !legal.castable.contains(&card))
    }

    /// Builds the action for activating an ability, rejecting anything not
    /// offered.
    ///
    /// The order of the two branches is the whole of it, and it used to be the
    /// other way round: the mana branch fired on `ability_index == 0` for any
    /// permanent named in `mana_abilities`, which reads the index's *numeric
    /// value* rather than what was offered. A fetchland enters
    /// `mana_abilities` the moment something grants it a mana ability — a
    /// Chromatic Lantern grants every land you control one — and its real
    /// `{T}, Sacrifice this: search` also sits at index 0, so a Flooded Strand
    /// under a Lantern tapped for mana instead of searching. Both cards are in
    /// the starter deck the lobby posts, which is why this was met in the
    /// first game and not in an edge case.
    ///
    /// So an explicit offer wins: `(source, index)` in `abilities` is the
    /// engine naming that ability, and index 0 is only the CR 305.6 shortcut
    /// when the engine offered nothing else there.
    #[must_use]
    pub fn activate(&self, source: ObjectId, ability_index: u32) -> Option<PlayerAction> {
        let legal = self.legal_actions()?;
        if legal.abilities.contains(&(source, ability_index)) {
            return Some(PlayerAction::ActivateAbility {
                source,
                ability_index,
            });
        }
        (legal.mana_abilities.contains(&source) && ability_index == 0)
            .then_some(PlayerAction::ActivateManaAbility { source })
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
        | Pending::ChooseCardType { player, .. }
        | Pending::ChooseCardName { player, .. }
        | Pending::ChooseColor { player, .. }
        | Pending::ChooseManaAbility { player, .. }
        | Pending::ChooseDamageSource { player, .. }
        | Pending::ChooseDamageEffect { player, .. }
        | Pending::AllocatePrevention { player, .. }
        | Pending::ChooseNumber { player, .. }
        | Pending::ChoosePlayer { player, .. }
        | Pending::ChooseCastMode { player, .. }
        | Pending::ChoosePile { player, .. }
        | Pending::Arrange { player, .. }
        | Pending::YesNo { player, .. } => Some(*player),
        Pending::GameOver(_) => None,
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod damage_tests;

impl Prompt {
    fn resolution_headline(&self, lang: Lang) -> String {
        match self {
            Self::TextReplacement { draft } => {
                let words = crate::text_choice::words(draft.kind, lang);
                Phrase::TextReplacementSummary.fill(
                    lang,
                    &[
                        draft.from.map_or("…", |i| words[usize::from(i)]),
                        draft.to.map_or("…", |i| words[usize::from(i)]),
                    ],
                )
            }
            Self::ChooseManaAbility { .. } => Phrase::ChooseRequiredMana.text(lang).to_string(),
            Self::ChooseDamageSource { .. } => Phrase::ChooseDamageSource.text(lang).to_string(),
            Self::ChooseDamageEffect { .. } => Phrase::ChooseDamageEffect.text(lang).to_string(),
            Self::AllocatePrevention {
                effect,
                total,
                amounts,
                ..
            } => crate::damage::allocation_headline(lang, effect, *total, amounts),
            Self::ChooseCards {
                reason: ChoicePrompt::CastFaceDown { x, .. },
                ..
            } => Phrase::MaskChoose.fill(lang, &[&x.to_string()]),
            Self::ChooseCards {
                reason: ChoicePrompt::CommandCard,
                ..
            } => Phrase::CommandChoose.text(lang).to_string(),
            _ => unreachable!("only resolution prompts use this formatter"),
        }
    }
}

impl Interaction {
    fn waiting_prompt(&self) -> Prompt {
        match &self.pending {
            Pending::GameOver(_) => Prompt::GameOver,
            other => Prompt::Waiting {
                on: pending_player(other),
            },
        }
    }

    fn prevention_prompt(
        &self,
        effect: &DamageEffectOption,
        damage: &[DamagePartView],
        total: u32,
    ) -> Prompt {
        Prompt::AllocatePrevention {
            effect: effect.clone(),
            damage: damage.to_vec(),
            total,
            amounts: self
                .allocation()
                .map_or_else(Vec::new, |a| a.amounts().to_vec()),
        }
    }
}

fn cast_choice_headline(options: &[CastModeDesc], lang: Lang) -> String {
    let phrase = if options
        .first()
        .is_some_and(|o| matches!(o.kind, baylee_engine::choice::CastModeKind::Dungeon(_)))
    {
        Phrase::ChooseDungeon
    } else if options.first().is_some_and(|o| {
        matches!(
            o.kind,
            baylee_engine::choice::CastModeKind::DungeonRoom { .. }
        )
    }) {
        Phrase::ChooseDungeonRoom
    } else if options.first().is_some_and(|o| {
        matches!(
            o.kind,
            baylee_engine::choice::CastModeKind::MutatePosition(_)
        )
    }) {
        Phrase::ChooseMutatePosition
    } else if !options.is_empty()
        && options
            .iter()
            .all(|option| matches!(option.kind, baylee_engine::choice::CastModeKind::Colors(_)))
    {
        Phrase::ChooseColours
    } else if !options.is_empty()
        && options
            .iter()
            .all(|option| matches!(option.kind, baylee_engine::choice::CastModeKind::Counter(_)))
    {
        Phrase::ChooseCounterKind
    } else {
        Phrase::ChooseHowToCast
    };
    phrase.text(lang).to_string()
}
