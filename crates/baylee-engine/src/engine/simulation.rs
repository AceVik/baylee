//! Disposable simulations forked from a running game, for one seat's
//! lookahead in training and evaluation (`docs/engine-internals.md`
//! §"Forking a simulation").
//!
//! A harness that already owns the real [`Engine`] — the evaluator runs its
//! games in-process — can search from any ordinary priority decision by
//! advancing copies of it. A copy of the whole engine keeps the complete
//! driver continuation (whose priority it is, how many passed, which step
//! of combat, the loop watch), so nothing has to be reconstructed and no
//! latch is reset. What a copy may not keep is anything the searching seat
//! could not see: [`Engine::fork_for`] copies the engine and, before
//! handing it out, deals every hidden card again from a policy seed and
//! replaces everything that still names the real ones.
//!
//! This is not the live-play seam: a seat playing over the gateway has no
//! engine to copy, and `docs/proposals/lookahead-design.md` is the design
//! for that one. It is compiled only under the `simulation` feature (and in
//! this crate's tests), which `cargo dev gate shipped-features` keeps out of
//! every hosting build.

mod driver;
mod redeal;
mod transcript;

use super::Engine;
use crate::choice::Pending;
use crate::rng::GameRng;
use crate::state::CardLookup;
use baylee_core::ids::PlayerId;

/// The `ChaCha8` stream of the policy seed a fork's hidden cards are dealt
/// from.
const BELIEF: u64 = u64::from_be_bytes(*b"belief\0\0");
/// The `ChaCha8` stream of the policy seed a fork's future randomness —
/// its shuffles, coin flips and dice — comes from.
const FUTURE: u64 = u64::from_be_bytes(*b"future\0\0");

/// What the forking seat may assume about the decks at the table.
///
/// [`Engine::fork_for`] deals the hidden cards from the multiset that is
/// hidden, which it reads off the real game. That is the opponent's
/// submitted deck less what has been seen, and reading it is legitimate only
/// where the seat knows that deck anyway.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeckKnowledge {
    /// Every deck list at the table, main deck and sideboard as submitted
    /// and in their submitted order, is public to the seat: a published
    /// precon, a deck an evaluation harness chose. The caller asserts it;
    /// the engine cannot check it.
    Known,
    /// Some deck list is private to its owner. Nothing about it may be read
    /// off the game, and a fork would need a public prior over decks that
    /// does not exist yet, so the fork is refused.
    Unknown,
}

/// Why [`Engine::fork_for`] made no fork.
///
/// Every refusal names a position the first version cannot copy without
/// either leaking a hidden fact or dropping a rules fact. A search that is
/// refused answers the way it would have without searching.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ForkRefusal {
    /// The seat is not at this table.
    #[error("no seat {0:?} at this table")]
    NoSuchSeat(PlayerId),
    /// [`DeckKnowledge::Unknown`]: a deck list is not the seat's to read.
    #[error("an opposing deck list is not known to the seat")]
    UnknownDecks,
    /// The game has ended.
    #[error("the game is over")]
    GameOver,
    /// The opening hands are still being decided, on streams of the real
    /// seed.
    #[error("the opening hands are still being decided")]
    OpeningHands,
    /// The engine is asking something other than ordinary priority.
    #[error("the engine is not at an ordinary priority decision")]
    NotPriority,
    /// The driver holds a suspended operation (a resolution, a cast, a
    /// payment window, queued triggers), named by its field. Its
    /// continuation may already name hidden cards, and dealing them again
    /// under it is not supported yet.
    #[error("the engine holds a suspended operation: {0}")]
    Continuation(&'static str),
    /// The loop watch has saved a signature of the real board, hidden
    /// cards included, or is breaking a loop.
    #[error("the loop watch holds a signature of the real hidden cards")]
    LoopWatch,
    /// A face-down permanent or spell is in play: what it is, and who may
    /// look at it, is not dealt yet (CR 708.2, CR 708.5).
    #[error("a face-down object is in play")]
    FaceDown,
    /// A card is exiled face down, with its own viewing rights (CR 406.3).
    #[error("a card is exiled face down")]
    HiddenExile,
    /// A commander is in a hidden zone: its identity is public, its place
    /// is not, and the deal does not separate the two yet.
    #[error("a commander is in a hidden zone")]
    HiddenCommander,
    /// A hidden card carries rules state of its own (a reveal that binds
    /// it, counters, a copy effect), which a dealt card would not have, or a
    /// static that works where it hides, whose registrations record where
    /// it has been (Grist, the Hunger Tide).
    #[error("a hidden card carries rules state of its own: {0}")]
    MarkedHiddenCard(&'static str),
    /// Something public points at a hidden card, which the deal would
    /// sever.
    #[error("{0} names a hidden card")]
    ReferencedHiddenCard(&'static str),
    /// The journal records another seat's standing answers, which are that
    /// seat's own setting.
    #[error("the journal records another seat's standing answers")]
    PrivatePolicy,
}

impl<L: CardLookup + Clone> Engine<L> {
    /// A disposable copy of this game for `seat`'s lookahead, with every
    /// card `seat` cannot see dealt again from `policy_seed`.
    ///
    /// For training and evaluation harnesses that own the real engine; a
    /// seat playing over the gateway has none. `decks` must be
    /// [`DeckKnowledge::Known`]: the cards dealt are the ones hidden in this
    /// game, which only a seat that knows every deck list may read.
    ///
    /// The fork is the whole engine — board, journal length, priority
    /// holder, passes, combat progress, loop watch — so it continues exactly
    /// where this one stands, with these differences:
    ///
    /// - **Hidden cards.** Each opponent's hand and library together, each
    ///   opponent's cards outside the game, and `seat`'s own library are
    ///   collected per owner, sorted by card identity (never by object or
    ///   position), shuffled on the policy seed's `belief` stream and dealt
    ///   back into the same zones at the same sizes as new objects. Their
    ///   old [`ObjectId`]s no longer exist in the fork, and the new ones are
    ///   allocated in zone order, so neither an id nor a position says which
    ///   real card was there. What `seat` can see now is not dealt: a hand a
    ///   static reveals or a player it controls holds (CR 722), and a
    ///   library top a static reveals. What it knew earlier about a hidden
    ///   card (a top it saw, a card revealed from a hand and kept there) is
    ///   lost with it; nothing it did not know is gained.
    /// - **Randomness.** The fork rolls on the policy seed's `future`
    ///   stream. The real seed, stream position and call count are not in
    ///   it. `policy_seed` is the caller's and must not be derived from the
    ///   game's seed, or the separation is only apparent.
    /// - **Journal.** Same length and sequence numbers; the seed is zeroed,
    ///   moves between zones `seat` cannot see and every mention of a dealt
    ///   card's old id name [`ObjectId::NO_SOURCE`] instead, and developer
    ///   commands lose their text.
    /// - **Notes about hidden cards.** The copies the game kept of objects
    ///   as they left a zone `seat` cannot see, this turn's draws, the other
    ///   per-turn notes naming a dealt card's old id, and the text overrides
    ///   of effects the re-sync removed are dropped; none can be read by a
    ///   rule once that id is gone. Every other field of the state is
    ///   classified in `GameState::audit_fork`.
    /// - **Settings.** No seat has capabilities, and every other seat's
    ///   standing answers and priority holds are the defaults; `seat` keeps
    ///   its own.
    ///
    /// Derived caches (the layer projection, statics that work from hidden
    /// zones, the priority offer) are rebuilt by the engine's own refresh,
    /// not copied.
    ///
    /// The invariant: for fixed seat-visible facts and `policy_seed`, the
    /// fork's hidden zones and future randomness are the same whatever the
    /// real hidden cards and random state were; forking, and advancing the
    /// fork, leave this engine's state, hash and random stream unchanged. It
    /// is about where the hidden cards are, not about history: a public
    /// card's version still counts its hidden moves.
    ///
    /// [`ObjectId`]: baylee_core::ids::ObjectId
    /// [`ObjectId::NO_SOURCE`]: baylee_core::ids::ObjectId::NO_SOURCE
    ///
    /// # Errors
    /// A [`ForkRefusal`] for every position the first version cannot fork
    /// faithfully: anything but ordinary priority, a suspended operation,
    /// face-down objects, hidden exile, a hidden card with rules state of
    /// its own or a static that works where it hides, something public
    /// pointing at a hidden card, an unknown deck. Every refusal depends on
    /// what `seat` can see, never on where a hidden card is.
    pub fn fork_for(
        &self,
        seat: PlayerId,
        decks: DeckKnowledge,
        policy_seed: u64,
    ) -> Result<Self, ForkRefusal> {
        if usize::from(seat.get()) >= self.state.players.len() {
            return Err(ForkRefusal::NoSuchSeat(seat));
        }
        if decks != DeckKnowledge::Known {
            return Err(ForkRefusal::UnknownDecks);
        }
        if self.state.game_end.is_some() {
            return Err(ForkRefusal::GameOver);
        }
        driver::check_settled(self)?;
        let sees_hand =
            |owner| self.may_inspect_private(seat, owner) || self.state.hand_revealed(owner);
        let deal = redeal::Deal::plan(&self.state, seat, &sees_hand)?;
        self.state.audit_fork(&|id| deal.is_retired(id))?;
        transcript::check(&self.state.journal, seat)?;

        let mut fork = driver::duplicate(self);
        driver::forget_settings(&mut fork, seat);
        deal.apply(&mut fork.state, &mut GameRng::stream(policy_seed, BELIEF));
        transcript::withhold(&mut fork.state.journal, &self.state, seat, deal.retired());
        fork.state.rng = GameRng::stream(policy_seed, FUTURE);
        fork.settle_fork();
        Ok(fork)
    }

    /// Rebuilds what the fork derives from its cards: the statics of
    /// hidden cards that work where they are, the layer projection, and the
    /// priority offer that was computed from the real hand.
    ///
    /// The re-sync removes the effects of the cards the deal removed, and
    /// their text overrides go with them, as the end of the turn would drop
    /// them (`GameState::prune_damage_sources`): an override of an effect
    /// that no longer exists is read by nothing, and it names the card.
    fn settle_fork(&mut self) {
        self.state.invalidate_projections();
        self.sync_static_effects();
        let effects = &self.state.effects;
        self.state
            .effect_text_overrides
            .retain(|(id, _)| effects.contains(*id));
        self.state.refresh_characteristics();
        if let Pending::Priority { player, .. } = self.pending {
            self.pending = Pending::Priority {
                player,
                legal: Box::new(self.compute_legal(player)),
            };
        }
    }
}
