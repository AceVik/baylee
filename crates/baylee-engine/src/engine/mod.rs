//! The engine driver: the complete turn/priority state machine.
//!
//! Public contract (see `docs/engine-internals.md`): the game only advances
//! through [`Engine::apply`] answering [`Engine::pending`]. Everything else
//! — SBAs, stack resolution, turn-based actions — is automatic.

use crate::casting::{self, CastFailure};
use crate::choice::{LegalActions, Pending, PlayerAction};
use crate::combat::{self, AttackerInfo};
use crate::eval;
use crate::event::{Cause, GameEvent, LossReason};
use crate::mana_pay;
use crate::object::{AbilityLoc, GameObject, ObjectKind, Status};
use crate::resolve::{self, Resolution};
use crate::sba;
use crate::state::{CardLookup, GameState, SetupError, StateError};
use crate::trigger;
use crate::turn::{Phase, Step};
use crate::win::{EndReason, GameResult};
use crate::zone::{Zone, ZoneLocation, ZonePosition};
use baylee_cards_dsl::{AbilityDef, ActivationTiming, Cost, CostPart};
use baylee_core::ids::{NameRef, ObjectId, PlayerId};
use baylee_core::preset::{GamePreset, HouseRules};
use baylee_core::types::TypeSet;
use smallvec::SmallVec;
use std::collections::VecDeque;

/// Engine API errors.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    /// The game already ended.
    #[error("game is over")]
    GameOver,
    /// The action does not match the pending request.
    #[error("action does not match the pending request")]
    MismatchedAction,
    /// The action is not legal right now.
    #[error("illegal action: {0}")]
    IllegalAction(&'static str),
    /// Setup failed.
    #[error("setup: {0}")]
    Setup(#[from] SetupError),
    /// Casting/playing failed.
    #[error("casting: {0}")]
    Cast(#[from] CastFailure),
    /// Zone machinery failure.
    #[error("state: {0}")]
    State(#[from] StateError),
}

/// Which combat declaration has already happened this step.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CombatDeclared {
    None,
    Attackers,
    Blockers,
}

/// A CR 605.3a payment window, and the resolution it was opened over.
///
/// The resolution travels *with* the window rather than staying in
/// [`Engine::resolution`], because that slot belongs to whatever is resolving
/// **now** — and a window is precisely the moment when something else can be.
/// A seat told to make mana may activate a mana ability that asks a question:
/// Badlands asks which of its two colours, and such an ability suspends a
/// resolution of its own. Sharing one slot, it overwrote the tax it was being
/// made for and then completed, leaving nothing for `close_mana_window` to
/// settle. The `expect` there is the only thing that noticed (#167) — without
/// it the payment is silently lost, with the mana still floating and the
/// spell escaping a ward that was paid for.
///
/// Holding the two together is what makes that unrepresentable rather than
/// merely repaired: there is no window without its resolution, and no way to
/// lose one while the other stands.
///
/// It nests exactly once. A window is opened only by a tax, only a mana
/// ability may be activated inside one, and no mana ability in this pool
/// charges a tax — which is a claim about all of the cards rather than about
/// this struct, so it is a scan and not this sentence:
/// `offer_tests::no_mana_ability_in_the_pool_opens_a_payment_window`.
#[derive(Clone, Debug)]
struct PaymentWindow {
    /// The seat that said it would pay.
    player: PlayerId,
    /// The resolution waiting for that payment.
    suspended: Box<crate::resolve::Resolution>,
}

/// A deterministic, self-contained game of Magic.
// The driver genuinely is a set of independent latches (a pending answer,
// a queued resolution, an agreed draw, a broken loop); folding them into an
// enum would make each one lie about the others.
#[allow(clippy::struct_excessive_bools)]
pub struct Engine<L: CardLookup> {
    lookup: L,
    state: GameState,
    pending: Pending,
    house_rules: HouseRules,
    /// Consecutive priority passes since the last non-pass action.
    passes: u8,
    /// Who holds priority in the current APNAP round (`None` = no round).
    priority_holder: Option<PlayerId>,
    /// All players passed with a non-empty stack: resolve the top.
    resolve_next: bool,
    /// The player who just took a non-pass action and is owed priority back
    /// (CR 117.3c) — once the machine has finished with what they did.
    ///
    /// A flag rather than a published `Pending`, and that is the whole of
    /// the difference. [`Engine::after_action`] used to build the question
    /// itself and set `awaiting_answer`, which is the first line
    /// [`Engine::run_machine`] returns on — so between an action and the
    /// *next* one the machine did not run at all. Nothing settled the
    /// continuous effects the action had invalidated, nothing ran the
    /// state-based actions it owed (CR 117.5), and nothing put the abilities
    /// it triggered on the stack (CR 603.3b). A player who dropped a land
    /// that says "when this land enters, it deals 1 damage to target
    /// opponent" was handed priority looking at an empty stack, with the
    /// trigger still unread in the journal.
    ///
    /// So the action records only that priority is owed, and
    /// [`Engine::priority_round`] hands it over at step 5 the way it hands
    /// over every other priority — after the machine has done the work the
    /// action made for it.
    regrant_priority: Option<PlayerId>,
    /// Mulligan progress per seat.
    mulligans: Vec<u8>,
    /// Seat currently mulliganing.
    mulligan_player: usize,
    /// Combat declaration progress.
    combat_declared: CombatDeclared,
    /// Planeswalkers that already used a loyalty ability this turn.
    loyalty_used_this_turn: Vec<ObjectId>,
    /// `true` while the current pending request is unanswered — the
    /// progression machine must never overwrite a fresh pending.
    awaiting_answer: bool,
    /// A suspended effect resolution (choice continuation).
    resolution: Option<Resolution>,
    /// A player making mana to meet a payment an effect has asked of them
    /// (CR 605.3a), while the resolution that asked is suspended above.
    ///
    /// The rule is explicit that a mana ability may be activated "whenever a
    /// rule or effect asks for a mana payment, even if it's in the middle of
    /// casting or resolving a spell or activating or resolving an ability",
    /// and this engine had no moment at which that could happen: priority is
    /// the only time a player may act, and a resolution grants none. So the
    /// payment question opens one, and it is an ordinary `Pending::Priority`
    /// rather than a request of its own — the client draws it and the agent
    /// answers it already, and a question shaped like every other question
    /// needs no `VIEW_VERSION`. The *price* did, in the end
    /// ([`Self::payment_window`], `VIEW_VERSION` 24): a question that looks
    /// like every other question is exactly one nothing can tell apart from
    /// an empty priority, and the agent read it as one — it said yes to
    /// ward's tax, was handed the window, saw nothing castable and passed.
    /// What makes it a window and not priority is
    /// this field: `compute_legal` reads it and offers mana and nothing
    /// else, and passing closes the window instead of counting toward the
    /// round.
    ///
    /// It carries the player and the resolution the window was opened over,
    /// and not the price. The price is already in that resolution's
    /// `AwaitingOp::PlayerMayPay`, and a second copy of it here would be a
    /// number that could disagree with the one actually charged.
    mana_window: Option<PaymentWindow>,
    /// Journal sequence number up to which triggers were collected.
    trigger_scan_seq: u64,
    /// A cast/activation waiting for its target choice.
    pending_plan: Option<PlanKind>,
    /// A player chosen for a pending loyalty `AnyPlayer` target.
    loyalty_player_choice: Option<PlayerId>,
    /// The seats named by a pending activation's target choice.
    ///
    /// Carried in a field for the reason [`Engine::activating_abilities`] is:
    /// `start_activation` is re-entered with the answer, and threading a
    /// second list through every caller of a function most of them pass
    /// nothing to buys nothing. It is taken at the top of that function, so
    /// an activation refused after `apply` set it cannot hand it on.
    activation_target_players: Vec<PlayerId>,
    /// The objects named for a pending activation's *cost* (CR 601.2h).
    ///
    /// The same bargain as `activation_target_players` one field up, and one
    /// step later in the same checklist: `apply` appends an answer and
    /// re-enters `start_activation`, which counts what is here against what
    /// the cost still wants and either asks again or pays. In the order the
    /// cost prints its parts, so the nth answer pays the nth asking part.
    ///
    /// Taken at the moment the cost is paid rather than at the top of
    /// `start_activation`, because unlike the seats it is being *accumulated*
    /// across several answers — clearing it on entry would ask the first
    /// question forever. Every path that can refuse the activation clears it
    /// on the way out.
    activation_cost_choices: Vec<ObjectId>,
    /// The number a pending activation's counter cost was given (CR 601.2b).
    ///
    /// The third field in this family and the one asked *first*: a cost that
    /// says "remove any number of storage counters" names no number, and an
    /// X is announced before targets are chosen. `start_activation` asks for
    /// it, `apply` puts the answer here and re-enters, and the same number
    /// is both what `pay_cost` takes off the source and the `Amount::X`
    /// every effect of that ability reads back.
    ///
    /// Cleared where [`Engine::activation_cost_choices`] is — on a fresh
    /// press and on every path that refuses — for the same reason twice
    /// over: a stale answer here would pay the *next* activation's cost with
    /// the last one's number and never ask again.
    activation_x: Option<u32>,
    /// The ability list the next push to the stack should use instead of
    /// asking the source — read before its cost is paid, or carried on a
    /// trigger that is looking back in time.
    ///
    /// CR 602.2a puts an activated ability on the stack *before* its costs
    /// are paid; this engine pays first and pushes after, which is invisible
    /// until a cost moves the source. "Sacrifice this creature" does, and a
    /// card-backed object stops being a copy when it moves (CR 400.7) — so a
    /// Glasspool Mimic copying Werefox Bodyguard could no longer say what its
    /// own ability was by the time the ability existed. Carried here rather
    /// than threaded through four resumption points, the way
    /// [`Engine::loyalty_player_choice`] already is.
    ///
    /// The other filler is [`Engine::hand_over_trigger_abilities`]: a
    /// leaves-the-battlefield or dies trigger of a copy is collected after
    /// the copy has been given back (CR 603.10a), so the list has to travel
    /// with the trigger and be laid down here on the way to the stack.
    ///
    /// Keyed by the source it was read from, so an activation that is refused
    /// after setting it cannot lend its list to the next ability anything
    /// pushes.
    ///
    /// [`Engine::hand_over_trigger_abilities`]: crate::engine::Engine
    activating_abilities: Option<(ObjectId, &'static [baylee_cards_dsl::AbilityDef])>,
    /// What each seat may do beyond answering its own choices.
    ///
    /// Not game state: it never enters the snapshot hash and never changes
    /// during a game, because it is a statement about who is sitting there
    /// rather than about the board.
    capabilities: Vec<baylee_core::preset::SeatCapabilities>,
    /// Journal seq up to which as-it-enters modifiers were applied.
    entry_scan_seq: u64,
    /// Delayed actions queued by upkeep processing.
    delayed_queue: VecDeque<crate::state::DelayedAction>,
    /// Synthetic keyword-trigger effects by stack object (prowess).
    synthetic_fx: rustc_hash::FxHashMap<ObjectId, &'static [baylee_cards_dsl::Effect]>,
    /// A spell being cast step by step (modes/targets/X/kicker/pitch).
    cast_wizard: Option<cast_wizard::CastWizard>,
    /// Triggers collected but not yet stacked (target choices first).
    trigger_queue: VecDeque<trigger::PendingTrigger>,
    /// Every player still in the game accepted a draw offer (CR 104.4i).
    agreed_draw: bool,
    /// Per-seat automation: when to offer priority, which yes/no
    /// questions to answer without asking (see `choice::Automation`).
    automation: Vec<crate::choice::SeatAutomation>,
    /// A loop was broken and its fuel is being withheld (house rule
    /// `RunOnceThenBreak`). Lasts one answer.
    breaking_loop: bool,
    /// Watches the situation as answers arrive, which is where a mandatory
    /// loop shows up: every loop in Magic runs through the stack, so the
    /// players are asked every time round.
    action_loops: crate::loops::LoopWatch,
    /// How many endless loops this game has broken. Diagnostic only: the
    /// journal carries the authoritative `LoopDetected` entries.
    loops_broken: u32,
}

/// A distinguishing tag per priority-hold shape, including its payload —
/// two holds of the same kind with different conditions are different
/// engine states.
fn hold_tag(hold: crate::choice::PriorityHold) -> u64 {
    use crate::choice::PriorityHold as H;
    match hold {
        H::Always => 1,
        H::PassWhenNothingToDo => 2,
        H::UntilStackEmpty { depth } => 3 ^ (u64::from(depth) << 8),
        H::UntilTopOfStack { object } => 4 ^ (u64::from(object.slot()) << 8),
        H::UntilEndOfTurn { turn } => 5 ^ (u64::from(turn) << 8),
    }
}

/// What a `Pending::ChooseTargets` is targeting for.
#[derive(Clone, Debug)]
enum PlanKind {
    /// Activating an ability.
    ActivateAbility {
        /// Source permanent.
        source: ObjectId,
        /// Ability index.
        ability_index: u32,
    },
    /// Putting a triggered ability on the stack.
    Trigger {
        /// Source permanent.
        source: ObjectId,
        /// Ability index.
        ability_index: u32,
        /// The mode already chosen, for a modal trigger (CR 603.3c).
        ///
        /// Carried on the plan rather than read back off the queue when the
        /// answer arrives: the queue's front is what this plan is *about*,
        /// but nothing in the type says so, and a mode read off the wrong
        /// entry resolves the wrong half of a card.
        mode: Option<u8>,
    },
    /// A shockland entry choice (pay life or enter tapped).
    EntryTap {
        /// The entering land.
        object: ObjectId,
        /// Life to pay.
        amount: u16,
    },
    /// A delayed pay-or-lose decision (Pact of Negation).
    DelayedPay {
        /// The mana cost to pay.
        cost: baylee_core::mana::ManaCost,
    },
    /// A delayed pay-or-sacrifice decision (echo).
    DelayedPaySacrifice {
        /// The mana cost to pay.
        cost: baylee_core::mana::ManaCost,
        /// The permanent sacrificed when not paid.
        card: ObjectId,
    },
    /// A clone choosing what to copy as it enters.
    CopyOnEnter {
        /// The entering permanent.
        object: ObjectId,
        /// Whether the permanent is still on its way in.
        ///
        /// CR 614.12a makes this choice one that is taken **before** the
        /// permanent enters, so a permanent *spell* is asked while it is
        /// still on the stack and the answer owes it the move onto the
        /// battlefield. Every other door — reanimation, a search, a token
        /// copy — is still asked one step after it arrived, and there the
        /// move has already happened.
        ///
        /// It is a field on the plan rather than a second variant because
        /// the question, its options and its answer are the same in both
        /// cases; only who performs the move differs, and a reader of the
        /// answer should have to see which it is.
        before_entry: bool,
    },
    /// Choosing a creature type as a permanent enters (Roaming Throne).
    ChooseSubtype {
        /// The entering permanent.
        object: ObjectId,
    },
    /// Choosing a color as a permanent enters (Uncharted Haven).
    ///
    /// `Pending::ChooseColor` is asked for two different reasons — this, and
    /// a mana ability picking a colour as it resolves — and this plan is
    /// what tells them apart. Without it the entry-time answer would reach
    /// the resolution handler and take a `Resolution` that is not there.
    ChooseColor {
        /// The entering permanent.
        object: ObjectId,
    },
    /// Choosing which land face of an MDFC to play (pathways).
    PlayLandFace {
        /// The card being played.
        card: ObjectId,
    },
    /// Miracle offer for a drawn card (CR 702.94).
    Miracle {
        /// The drawn card.
        card: ObjectId,
    },
    /// A commander offered its way back to the command zone (CR 903.9a).
    CommanderZone {
        /// The commander card, in a graveyard or in exile.
        card: ObjectId,
    },
    /// Target choice for a synthetic trigger (granted triggered ability).
    SyntheticTriggerTarget {
        /// The queued trigger.
        trigger: crate::trigger::PendingTrigger,
    },
    /// The untap step's own determination (CR 502.3), waiting for the
    /// active player to say which permanents stay tapped.
    ///
    /// The one plan with no fields: what it is about is the step the game
    /// is in, and the step cannot have moved on while the question stands.
    UntapChoice,
    /// A loyalty ability waiting for its target player.
    LoyaltyPlayer {
        /// The walker.
        source: ObjectId,
        /// Ability index.
        ability_index: u32,
    },
    /// A draw offer working its way around the table (CR 104.4i).
    DrawOffer {
        /// Who offered the draw.
        proposer: PlayerId,
        /// Players not yet asked.
        remaining: Vec<PlayerId>,
        /// The decision the offer interrupted, restored on a refusal.
        resume: Box<Pending>,
    },
    /// A modal trigger waiting for its mode choice.
    ModalTrigger {
        /// The source permanent.
        source: ObjectId,
        /// Ability index.
        ability_index: u32,
    },
    /// An activation waiting for the number its counter cost asks for.
    ///
    /// Carries nothing but the ability, because it is the *first* question
    /// an activation asks: no target has been chosen yet and no cost answer
    /// has been given, so there is nothing else to hand back on the
    /// re-entry.
    ChooseActivationX {
        /// The permanent whose ability is being activated.
        source: ObjectId,
        /// Ability index.
        ability_index: u32,
    },
    /// An activation waiting for one of its cost's answers (CR 601.2h).
    ///
    /// The whole answered half of the activation travels on the plan,
    /// because `start_activation` takes `activation_target_players` out of
    /// the engine at its first statement — an activation refused further
    /// down must not leave it standing for the next one — so a re-entry that
    /// did not carry the seats back would drop every "target player" an
    /// ability had already been pointed at.
    PayActivationCost {
        /// The permanent whose ability is being activated.
        source: ObjectId,
        /// Ability index.
        ability_index: u32,
        /// The targets already chosen (CR 601.2c comes first).
        targets: SmallVec<[ObjectId; 2]>,
        /// The seats already chosen, put back before the re-entry.
        target_players: Vec<PlayerId>,
    },
}

impl<L: CardLookup> Engine<L> {
    /// Creates a game from a preset. The first pending request is the
    /// first seat's mulligan decision.
    ///
    /// # Errors
    /// [`EngineError::Setup`] for invalid presets or unknown cards.
    pub fn new(preset: &GamePreset, lookup: L) -> Result<Self, EngineError> {
        let state = GameState::from_preset(preset, &lookup)?;
        let trigger_scan_seq = state.journal.last_seq();
        let mut engine = Self {
            lookup,
            capabilities: preset.seats.iter().map(|s| s.capabilities).collect(),
            mulligans: vec![0; state.players.len()],
            automation: vec![crate::choice::SeatAutomation::default(); state.players.len()],
            breaking_loop: false,
            action_loops: crate::loops::LoopWatch::default(),
            loops_broken: 0,
            mulligan_player: 0,
            house_rules: preset.house_rules.clone(),
            state,
            pending: Pending::Mulligan {
                player: PlayerId::new(0),
                taken: 0,
                next_is_free: preset.house_rules.mulligan_free_first,
            },
            passes: 0,
            priority_holder: None,
            resolve_next: false,
            regrant_priority: None,
            combat_declared: CombatDeclared::None,
            loyalty_used_this_turn: Vec::new(),
            awaiting_answer: true,
            resolution: None,
            mana_window: None,
            trigger_scan_seq,
            pending_plan: None,
            agreed_draw: false,
            loyalty_player_choice: None,
            activation_target_players: Vec::new(),
            activation_cost_choices: Vec::new(),
            activation_x: None,
            activating_abilities: None,
            entry_scan_seq: 0,
            delayed_queue: VecDeque::new(),
            synthetic_fx: rustc_hash::FxHashMap::default(),
            cast_wizard: None,
            trigger_queue: VecDeque::new(),
        };
        // Every seat, not just the first: a permanent the preset put on the
        // battlefield was there before anybody's turn began, and a seat that
        // has not had a turn yet would otherwise measure against zero and
        // find its whole opening board asleep.
        let stamp = engine.state.timestamp;
        for player in &mut engine.state.players {
            player.turn_start_timestamp = stamp;
        }
        Ok(engine)
    }

    /// The current pending request.
    #[must_use]
    pub fn pending(&self) -> &Pending {
        &self.pending
    }

    /// Read-only state access (tests, debugging, dev mode).
    #[must_use]
    pub fn state(&self) -> &GameState {
        &self.state
    }

    /// The seat inside a CR 605.3a payment window and the generic mana it was
    /// asked for, if such a window is open.
    ///
    /// The window is an ordinary `Pending::Priority` — that is what lets the
    /// client draw it and the agent answer it with no new question shape — so
    /// nothing about the offer says *why* the seat is holding priority with
    /// nothing castable. This is the why, and it exists to be projected:
    /// a seat that has just said it will pay is owed the number it said yes
    /// to, and without it the only readings available are "there is nothing
    /// to do here" and a guess.
    ///
    /// The price is read back out of the operation the window is holding
    /// rather than stored beside it, for the reason written on the field: a
    /// second copy is a number that can disagree with the one actually
    /// charged.
    ///
    /// It used to have to be *gated* on the window as well, because the
    /// shared resolution slot is suspended with `PlayerMayPay` for the whole
    /// of the yes-or-no question too — so reading the operation alone
    /// reported a debt while the seat was still being asked whether it wanted
    /// one. There is no such moment to confuse this with any more: a
    /// resolution only reaches this field once the seat has said yes (#167).
    #[must_use]
    pub fn payment_window(&self) -> Option<(PlayerId, u16)> {
        let window = self.mana_window.as_ref()?;
        match window.suspended.awaiting {
            Some(crate::resolve::AwaitingOp::PlayerMayPay {
                player: payer,
                mana,
                ..
            }) if payer == window.player => Some((window.player, mana)),
            _ => None,
        }
    }

    /// Mutable state access for a seat that may rewrite the board.
    ///
    /// `None` for every seat that was not granted `dev_commands`, which in a
    /// lobby game is all of them: the gateway builds its presets without
    /// capabilities and a `CreateGame` request has no way to ask for one.
    /// The old `state_mut_dev()` took no seat and asked nobody — it was a
    /// public door into the state with a name that only sounded like a lock.
    pub fn dev_state_mut(&mut self, seat: PlayerId) -> Option<&mut GameState> {
        self.capabilities
            .get(seat.get() as usize)?
            .dev_commands
            .then_some(&mut self.state)
    }

    /// Republish the priority offer after the board was rewritten behind the
    /// engine's back.
    ///
    /// [`Engine::dev_state_mut`] hands out the whole state and nothing asks
    /// the question again afterwards: `compute_legal` ran when priority was
    /// granted, so a card moved by a dev command leaves `Pending::Priority`
    /// describing a board that no longer exists. Most seeding never notices
    /// — a permanent put on the battlefield is read off the state by
    /// whatever looks at it next — and it is wrong for everything the
    /// *offer* is a function of. The measured case is Deathrite Shaman:
    /// "exile target land card from a graveyard" is withheld while no
    /// graveyard holds a land (CR 601.2c has the same shape for a spell), so
    /// seeding two Forests after priority left the ability off
    /// `legal.abilities` with both of them sitting there.
    ///
    /// Only a priority offer is rebuilt. Every other `Pending` is a question
    /// with a fixed set of answers — which blocker, which mode — that the
    /// asking code computed from the board it meant; recomputing one here
    /// would be a second answer to a question already asked.
    pub fn refresh_offer(&mut self) {
        let Pending::Priority { player, .. } = self.pending else {
            return;
        };
        self.pending = Pending::Priority {
            player,
            legal: Box::new(self.compute_legal(player)),
        };
    }

    /// What a seat may do beyond answering its own choices.
    #[must_use]
    pub fn capabilities(&self, seat: PlayerId) -> baylee_core::preset::SeatCapabilities {
        self.capabilities
            .get(seat.get() as usize)
            .copied()
            .unwrap_or_default()
    }

    /// The journal.
    #[must_use]
    pub fn journal(&self) -> &crate::event::Journal {
        &self.state.journal
    }

    /// Determinism hash (state + suspended resolution + machine fields).
    #[must_use]
    pub fn snapshot_hash(&self) -> u64 {
        let base = self.state.snapshot_hash();
        let mut extra = self.trigger_scan_seq;
        if let Some(r) = &self.resolution {
            extra = extra
                .wrapping_mul(31)
                .wrapping_add(r.pc as u64)
                .wrapping_add(u64::from(r.on_stack.slot()))
                .wrapping_add(u64::from(r.controller.get()));
        }
        extra = extra.wrapping_mul(31).wrapping_add(u64::from(self.passes));
        // A CR 605.3a payment window narrows the legal actions to mana and
        // makes passing close the window rather than count toward the round,
        // so two engines that differ in it answer the next question
        // differently. Without this they hashed identically one step either
        // side of the tax being agreed to, with the board byte for byte the
        // same — and `snapshot_hash` is what a replay and a cross-machine
        // comparison compare, so that divergence passed.
        //
        // `+ 1` rather than the bare seat number: a window on **seat 0**
        // would otherwise fold in as zero and be indistinguishable from no
        // window at all, which is the one seat a test is least likely to use.
        extra = extra.wrapping_mul(31).wrapping_add(
            self.mana_window
                .as_ref()
                .map_or(0, |w| u64::from(w.player.get()) + 1),
        );
        // And the resolution that window is holding, for exactly the reason
        // `self.resolution` is folded in above. It *moved* out of that slot
        // in #167, so without this line it would have stopped being hashed
        // the day it was fixed — two engines inside a window over different
        // taxes comparing equal, which is the divergence with the longest
        // fuse here: it breaks a replay rather than a test.
        if let Some(w) = &self.mana_window {
            extra = extra
                .wrapping_mul(31)
                .wrapping_add(w.suspended.pc as u64)
                .wrapping_add(u64::from(w.suspended.on_stack.slot()))
                .wrapping_add(u64::from(w.suspended.controller.get()));
        }
        // Automation decides which decisions the engine takes on a seat's
        // behalf, and how many loops it has already broken decides whether
        // the next one is broken or drawn. Two engines that differ in
        // either will diverge from here on, so a resync that compared only
        // the board would call them identical while they are not.
        extra = extra
            .wrapping_mul(31)
            .wrapping_add(u64::from(self.loops_broken));
        for seat in &self.automation {
            extra = extra
                .wrapping_mul(31)
                .wrapping_add(hold_tag(seat.hold))
                .wrapping_add(seat.standing_answers().count() as u64);
            for (ability, answer) in seat.standing_answers() {
                extra = extra
                    .wrapping_mul(31)
                    .wrapping_add(u64::from(ability.card.get()))
                    .wrapping_add(u64::from(ability.index))
                    .wrapping_add(u64::from(answer.as_bool()));
            }
        }
        base ^ extra.rotate_left(17)
    }

    /// Applies a player's action and advances automatically until the next
    /// decision point.
    ///
    /// # Errors
    /// [`EngineError`] on mismatched/illegal actions.
    pub fn apply(&mut self, player: PlayerId, action: PlayerAction) -> Result<(), EngineError> {
        if matches!(self.pending, Pending::GameOver(_)) {
            return Err(EngineError::GameOver);
        }
        // A game that has started repeating itself keeps asking the same
        // questions forever, and no answer breaks it — every mandatory loop
        // in Magic runs *through* the stack, so the players are asked every
        // time round and passing is all they can do. Watching the situation
        // as each answer arrives is therefore where a real endless loop can
        // be told apart from a large-but-finite pile of work: the ally
        // deck's thousand rally triggers change the situation every time
        // round, a loop returns to it. See `crate::loops`.
        //
        // Trigger suppression from an earlier break lasts until the stack
        // has actually drained. Stopping after one answer is not enough:
        // the trigger that feeds the loop is only collected once whatever
        // is on the stack resolves, which is several answers later.
        if self.state.zones.stack_is_empty() && self.trigger_queue.is_empty() {
            self.breaking_loop = false;
        }
        let signature = self.action_loops.wants_sample().then(|| {
            self.state.loop_signature()
                ^ u64::from(player.get()).rotate_left(37)
                ^ u64::from(self.passes).rotate_left(53)
        });
        if let Some(period) = self.action_loops.step(signature)
            && self.on_loop_detected(period)
        {
            return Ok(());
        }
        // Automation settings change who gets interrupted, not the game.
        // They are answered in place: whatever was pending stays pending,
        // and the priority round is untouched.
        if action.is_automation_setting() {
            self.set_automation(player, &action);
            // Re-run the driver *without* clearing `awaiting_answer`: the
            // question on the table has not been answered, so the machine
            // must not step past it. All this does is give the new setting
            // a chance to cover that same question — and if it does not,
            // the pending stands exactly as it was.
            self.run_until_choice();
            return Ok(());
        }
        // Concession is always legal for any seated player (CR 104.3a).
        if let PlayerAction::Concede = action {
            sba::eliminate_player(&mut self.state, player, LossReason::Conceded);
            self.awaiting_answer = false;
            self.run_until_choice();
            return Ok(());
        }
        // A draw offer interrupts whoever holds priority and asks every
        // other player in turn; nothing else about the game moves.
        if let PlayerAction::OfferDraw = action {
            return self.offer_draw(player);
        }
        self.awaiting_answer = false;
        self.apply_inner(player, action)?;
        // And then the machine, which is the whole of the settling an action
        // owes. It used to be unreachable from here: `after_action` published
        // the next `Pending` itself and set `awaiting_answer`, which is the
        // first line `run_machine` returns on, so between an action and the
        // next one nothing ran at all — no layer projection, no state-based
        // actions, no triggers. That was patched once, by doing the machine's
        // steps 0a and 0b by hand right here; `after_action` records the
        // priority it owes instead now, and the hand-rolled copy goes with
        // the reason for it.
        self.run_until_choice();
        Ok(())
    }
}

mod abilities;
mod decision;
pub use decision::DecisionContext;
mod actions;
mod cast_wizard;
pub(crate) mod cost_wizard;
mod progress;

#[cfg(test)]
mod amount_sign_tests;
#[cfg(test)]
mod automation_tests;
#[cfg(test)]
mod base_sharing_tests;
#[cfg(test)]
mod capability_tests;
#[cfg(test)]
mod card_rider_tests;
#[cfg(test)]
mod card_tests;
#[cfg(test)]
mod cast_face_tests;
#[cfg(test)]
mod chosen_tests;
#[cfg(test)]
mod claim_tests;
#[cfg(test)]
mod combat_choice_tests;
#[cfg(test)]
mod combo_tests;
#[cfg(test)]
mod commander_tests;
#[cfg(test)]
mod condition_tests;
#[cfg(test)]
mod convoke_tests;
#[cfg(test)]
mod cycling_tests;
#[cfg(test)]
mod day_night_tests;
#[cfg(test)]
mod draw_tests;
#[cfg(test)]
mod enter_tests;
#[cfg(test)]
mod flashback_tests;

#[cfg(test)]
mod house_rules_tests;
#[cfg(test)]
mod investigate_tests;
#[cfg(test)]
mod keyword_tests;
#[cfg(test)]
mod land_mana_tests;
#[cfg(test)]
mod land_play_tests;
#[cfg(test)]
mod loop_tests;
#[cfg(test)]
mod m2_tests;
#[cfg(test)]
mod mana_tests;
#[cfg(test)]
mod mdfc_tests;
#[cfg(test)]
mod miracle_tests;
#[cfg(test)]
mod offer_tests;
#[cfg(test)]
mod printed_tests;
#[cfg(test)]
mod priority_tests;
#[cfg(test)]
mod resolution_tests;
#[cfg(test)]
mod s3_tests;
#[cfg(test)]
mod s4_tests;
#[cfg(test)]
mod s6_tests;
#[cfg(test)]
mod s7_tests;
#[cfg(test)]
mod s7b_tests;
#[cfg(test)]
mod s7c_tests;
#[cfg(test)]
mod saga_tests;
#[cfg(test)]
mod search_tests;
#[cfg(test)]
mod sickness_tests;
#[cfg(test)]
pub(crate) mod synthetic;
#[cfg(test)]
mod target_tests;
#[cfg(test)]
mod team_tests;
#[cfg(test)]
pub(crate) mod testkit;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod this_object_tests;
#[cfg(test)]
mod token_tests;
#[cfg(test)]
mod untap_tests;
#[cfg(test)]
mod vocabulary_tests;
#[cfg(test)]
mod w1_tests;
#[cfg(test)]
mod walker_tests;
#[cfg(test)]
mod waterbend_tests;
#[cfg(test)]
mod werewolf_tests;
