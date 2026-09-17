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

use crate::i18n::{Lang, Phrase, seat_name};
use baylee_core::ids::{Defender, ObjectId, PlayerId, SubtypeId};
use baylee_core::mana::ManaColor;
use baylee_engine::choice::{
    BlockOption, CastModeDesc, ChoicePrompt, LegalActions, Pending, PlayerAction, TargetPrompt,
    YesNoPrompt,
};
use baylee_engine::win::{EndReason, GameResult, Victor};
use baylee_view::GameStatic;

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
        /// Why.
        reason: TargetPrompt,
    },
    /// Choose a creature type.
    ChooseSubtype {
        /// The types on offer, in the engine's order.
        ///
        /// There are about three hundred and fifty of them, which is why
        /// this is the one indexed choice with a filter in front of it.
        options: Vec<SubtypeId>,
    },
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

impl Prompt {
    /// A short line for the prompt bar.
    ///
    /// `statics` is the roster, and it is here for the two lines that are
    /// about **somebody else**: the one that says whose answer the table is
    /// waiting for, and the one that says who offered a draw. Both had the
    /// seat and printed it as a number — "Warte auf Platz 1", "ein Remis
    /// wurde angeboten" — which is answerable at a duel by knowing there is
    /// only one other chair, and is not a question at all at a table of four.
    /// [`crate::i18n::seat_name`] numbers a seat the roster does not describe,
    /// so a frame drawn before `GameStatic` arrives says what it used to.
    #[must_use]
    pub fn headline(&self, lang: Lang, turn: Turn, statics: Option<&GameStatic>) -> String {
        match self {
            Self::Waiting { on: Some(p) } => {
                Phrase::WaitingForPlayer.fill(lang, &[&seat_name(lang, statics, *p)])
            }
            Self::Waiting { on: None } => Phrase::JustWaiting.text(lang).to_string(),
            Self::Mulligan { taken, free } => {
                if *free {
                    Phrase::MulliganFree.text(lang).to_string()
                } else {
                    Phrase::MulliganTaken.fill(lang, &[&taken.to_string()])
                }
            }
            Self::BottomCards { count } => Phrase::counted(
                usize::from(*count),
                Phrase::PutCardOnBottom,
                Phrase::PutOnBottom,
            )
            .fill(lang, &[&count.to_string()]),
            // Not "You have priority". A priority window is an invitation, and
            // stating the rules term for it — next to a button labelled OK —
            // read as a modal that had to be dismissed before play could go on.
            //
            // And not "Your move" on an opponent's turn, which is the same
            // invitation and a different sentence: a player reads "Du bist
            // dran" over the Pass button as *it is your turn*, and then reads
            // the phase rail and the seat bars as disagreeing with it.
            Self::Priority { .. } => match turn {
                Turn::Mine => Phrase::YourMove.text(lang).to_string(),
                Turn::Theirs => Phrase::YouMayRespond.text(lang).to_string(),
            },
            Self::DeclareAttackers => Phrase::DeclareAttackers.text(lang).to_string(),
            Self::DeclareBlockers { .. } => Phrase::DeclareBlockers.text(lang).to_string(),
            Self::Discard { count } => Phrase::counted(
                usize::from(*count),
                Phrase::DiscardCard,
                Phrase::DiscardCards,
            )
            .fill(lang, &[&count.to_string()]),
            Self::LegendRule => Phrase::LegendRule.text(lang).to_string(),
            // Delve and convoke are not selections of cards or of targets:
            // both are "spend what you have to help pay", and a line saying
            // so is the difference between a question and a puzzle.
            Self::ChooseCards {
                reason: ChoicePrompt::Delve,
                ..
            } => Phrase::DelveToHelpPay.text(lang).to_string(),
            // Every other reason is said by the noun that is counted, which is
            // the one place in this sentence where it fits: "Wähle bis zu 2
            // Karten, die nach unten gehen". Without it a tutor, a scry, a
            // put-back and a wish all read "Wähle 1 Karte", and two of those
            // four decide the turn.
            Self::ChooseCards { reason, min, max } => {
                let (one, many) = choice_noun(*reason);
                choose_line(lang, one, many, *min, *max)
            }
            Self::ChooseTargets {
                reason: TargetPrompt::Convoke,
                ..
            } => Phrase::ConvokeToHelpPay.text(lang).to_string(),
            Self::ChooseTargets { min, max, .. } => {
                choose_line(lang, Phrase::NounTarget, Phrase::NounTargets, *min, *max)
            }
            Self::ChooseSubtype { .. } => Phrase::ChooseCreatureType.text(lang).to_string(),
            Self::ChooseColor { .. } => Phrase::ChooseColour.text(lang).to_string(),
            Self::ChooseNumber { min, max } => {
                Phrase::ChooseNumberIn.fill(lang, &[&min.to_string(), &max.to_string()])
            }
            Self::ChoosePlayer { .. } => Phrase::ChoosePlayer.text(lang).to_string(),
            Self::CastMode { .. } => Phrase::ChooseHowToCast.text(lang).to_string(),
            Self::OrderObjects => Phrase::PutInOrder.text(lang).to_string(),
            Self::YesNo { question } => yes_no_line(lang, *question, statics),
            Self::GameOver => Phrase::TheGameIsOver.text(lang).to_string(),
        }
    }
}

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
/// ten poison — is not here because it is not in the view: `SeatView` carries
/// `has_lost` and no reason for it, and inventing one from the life totals
/// would be the client deciding a rules fact. See the backlog's
/// "richer loss reason" item.
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

/// What a card choice is *for*, as the noun it counts — both forms.
///
/// [`ChoicePrompt`] has eleven variants and the prompt bar used to read one of
/// them. A library search, a scry, a put-back and a wish are four different
/// decisions and were four copies of the same sentence, so a player could not
/// tell whether they were fetching something, burying it or bringing it in
/// from outside. `Delve` is answered a line earlier (it is part of a cost,
/// not a selection) and `Generic` is the plain noun, which is honest: the
/// engine did not say what it was for either.
///
/// The last four are the other thing that is part of a cost and, unlike
/// delve, are one card rather than a heap of them: `CostSacrifice`,
/// `CostDiscard`, `CostTap` and `CostReturn` arrive while CR 601.2h is being
/// paid, so each
/// gets a noun that says what happens to the card rather than sharing delve's
/// "spend what you have" line. A player who is told only "choose 1 card"
/// while paying for Survival of the Fittest cannot tell the discard from the
/// creature it fetches — and the tap and the return are the two whose card
/// *survives*, which nobody would guess from a sacrifice's wording. Those
/// two are not one noun either: Quirion Ranger's Forest may already be
/// tapped, and a player reading "untapped permanent to tap" over their own
/// lands would look for the wrong one.
///
/// `LeaveTapped` is the one that is neither: the untap step asking the
/// active player which of their permanents stay tapped (CR 502.3). Its noun
/// says what *not* choosing does, because the empty answer is the whole
/// board untapping and a player shown "permanent to untap" over a menu of
/// one would read the question backwards.
fn choice_noun(reason: ChoicePrompt) -> (Phrase, Phrase) {
    match reason {
        ChoicePrompt::SearchLibrary => (Phrase::NounCardFromLibrary, Phrase::NounCardsFromLibrary),
        ChoicePrompt::ScryBottom => (Phrase::NounCardToBottom, Phrase::NounCardsToBottom),
        ChoicePrompt::SurveilGraveyard => {
            (Phrase::NounCardToGraveyard, Phrase::NounCardsToGraveyard)
        }
        ChoicePrompt::PutBackOnTop => (Phrase::NounCardToTop, Phrase::NounCardsToTop),
        ChoicePrompt::Wish => (Phrase::NounCardOutside, Phrase::NounCardsOutside),
        ChoicePrompt::CostSacrifice => (
            Phrase::NounPermanentToSacrifice,
            Phrase::NounPermanentsToSacrifice,
        ),
        ChoicePrompt::CostDiscard => (Phrase::NounCardToDiscard, Phrase::NounCardsToDiscard),
        ChoicePrompt::CostTap => (Phrase::NounPermanentToTap, Phrase::NounPermanentsToTap),
        ChoicePrompt::CostReturn => (
            Phrase::NounPermanentToReturn,
            Phrase::NounPermanentsToReturn,
        ),
        ChoicePrompt::LeaveTapped => (
            Phrase::NounPermanentToLeaveTapped,
            Phrase::NounPermanentsToLeaveTapped,
        ),
        ChoicePrompt::Delve | ChoicePrompt::Generic => (Phrase::NounCard, Phrase::NounCards),
    }
}

/// "Choose two cards", with the noun as an argument rather than glued on.
///
/// A count and a noun agree differently in different languages, so the whole
/// sentence has to be one phrase — pasting a translated noun onto a
/// translated "choose up to" is how a translation ends up ungrammatical.
///
/// The noun comes in both forms and **`max` picks between them**, because
/// `max` is the number the noun stands next to in all three frames: "up to
/// 2 cards", "1 card", "1–3 cards". A range whose top is more than one is
/// plural however low it starts.
fn choose_line(lang: Lang, one: Phrase, many: Phrase, min: u8, max: u8) -> String {
    let noun = Phrase::counted(usize::from(max), one, many).text(lang);
    match (min, max) {
        (0, m) => Phrase::ChooseUpTo.fill(lang, &[&m.to_string(), noun]),
        (a, b) if a == b => Phrase::ChooseExactly.fill(lang, &[&a.to_string(), noun]),
        (a, b) => Phrase::ChooseBetween.fill(lang, &[&a.to_string(), &b.to_string(), noun]),
    }
}

/// The roster is taken for the one question that is about another player.
fn yes_no_line(lang: Lang, question: YesNoPrompt, statics: Option<&GameStatic>) -> String {
    match question {
        YesNoPrompt::PayLifeOrEnterTapped { amount } => {
            Phrase::PayLifeOrTapped.fill(lang, &[&amount.to_string()])
        }
        YesNoPrompt::Kicker => Phrase::PayAdditionalCost.text(lang).to_string(),
        YesNoPrompt::PayTax { mana } => Phrase::PayTax.fill(lang, &[&mana.to_string()]),
        YesNoPrompt::Miracle { .. } => Phrase::CastForMiracle.text(lang).to_string(),
        // The `..` here is what the whole repair was: `proposer` travels with
        // the question and was dropped one line short of the sentence.
        YesNoPrompt::DrawOffer { proposer } => {
            Phrase::DrawOfferedBy.fill(lang, &[&seat_name(lang, statics, proposer)])
        }
        YesNoPrompt::CommanderZone { .. } => Phrase::CommanderToCommandZone.text(lang).to_string(),
        // The destination is the whole of the decision (CR 903.8 taxes only
        // the command zone), so the line has to name it rather than ask the
        // same question twice.
        YesNoPrompt::CommanderReplace { to_library, .. } => if to_library {
            Phrase::CommanderInsteadOfLibrary
        } else {
            Phrase::CommanderInsteadOfHand
        }
        .text(lang)
        .to_string(),
        // The line says *that* it is optional and not what the clause does:
        // the ability's own text belongs on the stack entry beside it, and
        // a second rendering of it here would be a translation of a
        // translation.
        YesNoPrompt::MayDo => Phrase::UseTheOptionalAbility.text(lang).to_string(),
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
    /// One of a fixed set of creature types.
    ///
    /// Three hundred and fifty of them are offered, which is why the
    /// answer is an index into the list rather than a click on the board:
    /// a creature type is not a thing on the table. Narrowing that list is
    /// the renderer's job; the model only ever hears which row was picked.
    Subtype { options: Vec<SubtypeId> },
    /// A yes-or-no answer.
    YesNo,
    /// Priority: an action menu rather than a selection.
    Priority { legal: Box<LegalActions> },
    /// Keep or mulligan.
    Mulligan,
    /// The game has ended.
    GameOver,
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
            mode,
            picks: Vec::new(),
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
                    focus: 0,
                }
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
                focus: 0,
            },
            Pending::ChooseCards {
                options, min, max, ..
            } => Mode::Objects {
                options: options.clone(),
                player_options: Vec::new(),
                min: *min as usize,
                max: *max as usize,
                focus: 0,
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
                focus: 0,
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
            // Answered from a list rather than from the board, because a
            // creature type is not a thing on it. `Mode::Idle` stood here
            // until now, which made this the one pending choice a client
            // could not answer at all.
            Pending::ChooseSubtype { options, .. } => Mode::Subtype {
                options: options.clone(),
            },
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
            Pending::ChooseTargets {
                min, max, reason, ..
            } => Prompt::ChooseTargets {
                min: *min,
                max: *max,
                reason: *reason,
            },
            Pending::ChooseSubtype { options, .. } => Prompt::ChooseSubtype {
                options: options.clone(),
            },
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
            Pending::ChooseCastMode {
                object, options, ..
            } => Prompt::CastMode {
                object: *object,
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
            Mode::Order { options } => Some((options.len(), options.len())),
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
        matches!(self.mode, Mode::Order { .. })
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
                if let Some(pos) = self.picks.iter().position(|p| *p == Pick::Object(id)) {
                    self.picks.remove(pos);
                    return SelectionOutcome::Removed;
                }
                if self.picks.len() >= self.capacity() {
                    return SelectionOutcome::Full;
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
            Mode::Order { options } => options.len(),
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
    pub fn toggle_group(&mut self, members: &[ObjectId]) -> SelectionOutcome {
        if let Some(next) = members
            .iter()
            .find(|id| self.is_selectable(**id) && !self.is_selected(**id))
        {
            return self.toggle(*next);
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

    /// Takes back the last pick, leaving the rest of the answer standing.
    ///
    /// `Esc` used to be [`Self::cancel`] here, which wipes a half-built
    /// answer whole — expensive for a mis-click on the third of five targets
    /// and, with no `PlayerAction::Cancel` to send, not even a way out of the
    /// question. One pick at a time is what a player means by "no, not that
    /// one". Returns what was taken back, or `None` when nothing was picked.
    pub fn take_back(&mut self) -> Option<Pick> {
        self.picks.pop()
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
                CombatFocus::Defender(Defender::Planeswalker(id)) | CombatFocus::Attacker(id) => {
                    Some(Pick::Object(id))
                }
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
        let max = *max;
        if let Some(at) = self.picks.iter().position(|p| *p == Pick::Seat(player)) {
            self.picks.remove(at);
            return SelectionOutcome::Removed;
        }
        if self.picks.len() >= max {
            return SelectionOutcome::Full;
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
        let count = match &self.mode {
            Mode::CastOption { count } => *count,
            Mode::Color { options } => options.len(),
            Mode::Player { options } => options.len(),
            Mode::Subtype { options } => options.len(),
            _ => 0,
        };
        if index >= count {
            return false;
        }
        self.choice_index = Some(index);
        true
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

    /// Whether the current answer is complete enough to submit.
    #[must_use]
    pub fn can_confirm(&self) -> bool {
        match &self.mode {
            Mode::Objects { min, .. } => self.picks.len() >= *min,
            Mode::Order { options } => self.picks.len() == options.len(),
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
            | Mode::Subtype { .. } => self.choice_index.is_some(),
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
            Mode::Order { options } if self.picks.len() == options.len() => {
                Some(PlayerAction::OrderObjects {
                    objects: self.selected().collect(),
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
            Mode::Subtype { options } => options
                .get(self.choice_index?)
                .copied()
                .map(PlayerAction::ChooseSubtype),
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
mod tests;
