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
//! A seat playing over the gateway has no engine to copy, and
//! `docs/proposals/lookahead-design.md` is the design for that one. The host
//! that owns the game does: `baylee-gamehost`'s house search
//! (`session/foresight.rs`) forks the table for a house chair, within the
//! level's bound and the host's (`docs/house-ai.md` §"Foresight"). It is
//! compiled only under the `fork` feature (and in this crate's tests).

mod driver;
mod redeal;
mod transcript;

use super::Engine;
use crate::choice::Pending;
use crate::rng::GameRng;
use crate::state::CardLookup;
use crate::text_changes::TextOrigin;
use baylee_core::ids::{CardIndex, EffectId, ObjectId, PlayerId};

/// The `ChaCha8` stream of the policy seed a fork's hidden cards are dealt
/// from.
const BELIEF: u64 = u64::from_be_bytes(*b"belief\0\0");
/// The `ChaCha8` stream of the policy seed a fork's future randomness —
/// its shuffles, coin flips and dice — comes from.
const FUTURE: u64 = u64::from_be_bytes(*b"future\0\0");

/// What the forking seat may assume about the decks at the table.
///
/// [`Engine::fork_for`] deals the hidden cards again. Where they come from
/// is the question: the multiset that is really hidden is the opponent's
/// submitted deck less what has been seen, and reading it is legitimate only
/// where the seat knows that deck anyway.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeckKnowledge<'a> {
    /// Every deck list at the table, main deck and sideboard as submitted
    /// and in their submitted order, is public to the seat: a published
    /// precon, a deck an evaluation harness chose. The caller asserts it;
    /// the engine cannot check it. The hidden cards are dealt from the
    /// multiset that is hidden, read off the game.
    Known,
    /// Some deck list is private to its owner and the seat has no prior
    /// over it. Nothing about it may be read off the game, so the fork is
    /// refused.
    Unknown,
    /// The seat knows its own deck list and no other. Its own library is
    /// dealt from its own hidden cards; every other owner's hidden cards
    /// are dealt from that owner's list in the prior, and the real ones are
    /// removed unread.
    Prior(&'a PublicPrior),
}

/// What a seat believes each other player's hidden cards to be, as far as
/// it can tell from what it has seen: for each of them a weighted list of
/// cards.
///
/// A fork under [`DeckKnowledge::Prior`] fills every slot of another
/// owner's hidden zones — a hand the seat does not see, a library below a
/// revealed top, the cards outside the game — with a card drawn from that
/// owner's list, with replacement, each with probability proportional to
/// its weight. The zones keep their sizes, which are public. The card need
/// not be in any deck at the table.
///
/// **The caller asserts that the prior was built only from facts visible to
/// the seat** — its own view of the game and its own deck list — and from
/// nothing the real hidden cards or the other deck lists could have
/// changed. The engine cannot check that. A prior computed from an
/// opponent's list is that list, dealt back into the fork.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PublicPrior {
    /// One list per player named, sorted by player; each list sorted by
    /// card with one entry per card.
    players: Vec<(PlayerId, Vec<(CardIndex, u32)>)>,
}

impl PublicPrior {
    /// A prior that names nobody.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            players: Vec::new(),
        }
    }

    /// This prior with `cards` as what `player`'s hidden cards are drawn
    /// from: each card with its weight, which must be above zero. A list
    /// given for `player` before is replaced. Entries naming the same card
    /// are one entry with their weights added (saturating), and the order
    /// of `cards` does not matter: two priors that weigh the same cards the
    /// same deal the same fork.
    #[must_use]
    pub fn with_player(mut self, player: PlayerId, mut cards: Vec<(CardIndex, u32)>) -> Self {
        cards.sort_unstable_by_key(|&(card, _)| card);
        cards.dedup_by(|later, kept| {
            let same = later.0 == kept.0;
            if same {
                kept.1 = kept.1.saturating_add(later.1);
            }
            same
        });
        match self
            .players
            .binary_search_by_key(&player, |&(named, _)| named)
        {
            Ok(at) => self.players[at].1 = cards,
            Err(at) => self.players.insert(at, (player, cards)),
        }
        self
    }

    /// The weighted cards `player`'s hidden cards are drawn from, sorted by
    /// card; empty for a player the prior does not name.
    #[must_use]
    pub fn cards(&self, player: PlayerId) -> &[(CardIndex, u32)] {
        self.players
            .binary_search_by_key(&player, |&(named, _)| named)
            .map_or(&[], |at| &self.players[at].1)
    }

    /// The players the prior names, in seat order.
    pub fn players(&self) -> impl Iterator<Item = PlayerId> + '_ {
        self.players.iter().map(|&(player, _)| player)
    }

    /// Whether a table of `seats` players can be dealt from this prior, as
    /// far as the prior alone says: every player it names is at the table
    /// and every card has a weight and is known to `lookup`. Whether a
    /// player with hidden cards has a list is the deal's to say.
    fn check(&self, seats: usize, lookup: &impl CardLookup) -> Result<(), ForkRefusal> {
        for (player, cards) in &self.players {
            if usize::from(player.get()) >= seats {
                return Err(ForkRefusal::UnusablePrior(PriorFault::NoSuchPlayer(
                    *player,
                )));
            }
            for &(card, weight) in cards {
                if weight == 0 {
                    return Err(ForkRefusal::UnusablePrior(PriorFault::NoWeight(
                        *player, card,
                    )));
                }
                if lookup.card(card).is_none() {
                    return Err(ForkRefusal::UnusablePrior(PriorFault::UnknownCard(card)));
                }
            }
        }
        Ok(())
    }
}

/// Why a [`PublicPrior`] could not be dealt from
/// ([`ForkRefusal::UnusablePrior`]). Each is a fact about the prior and the
/// public sizes of the hidden zones, never about a hidden card.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PriorFault {
    /// The prior names a player who is not at the table.
    #[error("it names {0:?}, who is not at this table")]
    NoSuchPlayer(PlayerId),
    /// A player other than the seat holds hidden cards and the prior has no
    /// cards for them: no list, or an empty one.
    #[error("it has no cards for {0:?}, who holds hidden cards")]
    NoCards(PlayerId),
    /// A card is listed with weight zero.
    #[error("it lists card {1} for {0:?} with no weight")]
    NoWeight(PlayerId, CardIndex),
    /// The card lookup does not know a card the prior lists, or a card that
    /// one links to.
    #[error("it lists card {0}, which the card lookup does not know")]
    UnknownCard(CardIndex),
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
    /// The engine is asking something other than ordinary priority or a
    /// combat declaration (declare attackers, declare blockers).
    #[error("the engine is not at an ordinary priority decision or a combat declaration")]
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
    /// [`DeckKnowledge::Prior`] with a prior that cannot be dealt from.
    #[error("the public prior cannot be dealt from: {0}")]
    UnusablePrior(PriorFault),
}

impl<L: CardLookup + Clone> Engine<L> {
    /// A disposable copy of this game for `seat`'s lookahead, with every
    /// card `seat` cannot see dealt again from `policy_seed`.
    ///
    /// For training and evaluation harnesses that own the real engine; a
    /// seat playing over the gateway has none. `decks` says where the hidden
    /// cards are dealt from. Under [`DeckKnowledge::Known`] they are the
    /// ones hidden in this game, which only a seat that knows every deck
    /// list may read. Under [`DeckKnowledge::Prior`] that holds for `seat`'s
    /// own library alone: every slot of another owner's hidden zones is
    /// drawn, with replacement and by weight, from that owner's list in the
    /// caller's [`PublicPrior`], as a card with no printing of its own, and
    /// the real cards there are removed unread — what they are, what they
    /// carry and how often they moved decide nothing, not the deal and not
    /// a refusal. Whether graveyard order matters (CR 404.3), which setup
    /// decides from every list, is then decided again from the fork's cards.
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
    ///   static reveals or a player it controls holds (CR 724.4), and a
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
    /// real hidden cards and random state were — and under a prior, for a
    /// fixed prior, whatever the other deck lists were; forking, and
    /// advancing the fork, leave this engine's state, hash and random stream
    /// unchanged. It is about where the hidden cards are, not about history:
    /// a public card's version still counts its hidden moves, and under a
    /// prior the name table, the base cache and the effect table's counters
    /// still count the real lists (`docs/engine-internals.md` §"Forking a
    /// simulation"), which no rule reads.
    ///
    /// [`ObjectId`]: baylee_core::ids::ObjectId
    /// [`ObjectId::NO_SOURCE`]: baylee_core::ids::ObjectId::NO_SOURCE
    ///
    /// # Errors
    /// A [`ForkRefusal`] for every position the first version cannot fork
    /// faithfully: anything but ordinary priority or a combat declaration, a suspended operation,
    /// face-down objects, hidden exile, a hidden card with rules state of
    /// its own or a static that works where it hides, something public
    /// pointing at a hidden card, an unknown deck, a prior that cannot be
    /// dealt from. Every refusal depends on what `seat` can see, never on
    /// where a hidden card is, and under a prior never on what another
    /// owner's hidden card is.
    pub fn fork_for(
        &self,
        seat: PlayerId,
        decks: DeckKnowledge<'_>,
        policy_seed: u64,
    ) -> Result<Self, ForkRefusal> {
        self.fork_for_with_shared(seat, decks, policy_seed, baylee_core::ids::SeatSet::new())
    }

    /// As [`Self::fork_for`], additionally keeping the hands of any teammates
    /// in `shared_hands` who have shared their full hand with `seat`.
    ///
    /// # Errors
    /// As [`Self::fork_for`]: when `seat` is not at the table, or the fork
    /// is refused for the reasons [`ForkRefusal`] names.
    pub fn fork_for_with_shared(
        &self,
        seat: PlayerId,
        decks: DeckKnowledge<'_>,
        policy_seed: u64,
        shared_hands: baylee_core::ids::SeatSet,
    ) -> Result<Self, ForkRefusal> {
        if usize::from(seat.get()) >= self.state.players.len() {
            return Err(ForkRefusal::NoSuchSeat(seat));
        }
        let prior = match decks {
            DeckKnowledge::Known => None,
            DeckKnowledge::Prior(prior) => Some(prior),
            DeckKnowledge::Unknown => return Err(ForkRefusal::UnknownDecks),
        };
        if self.state.game_end.is_some() {
            return Err(ForkRefusal::GameOver);
        }
        driver::check_settled(self)?;
        let sees_hand = |owner| {
            self.may_inspect_private(seat, owner)
                || self.state.hand_revealed(owner)
                || shared_hands.contains(owner)
        };
        let deal = redeal::Deal::plan(&self.state, seat, &sees_hand, prior, &self.lookup)?;
        self.state
            .audit_fork(&|id| deal.is_retired(id), &|id| deal.is_read(id))?;
        transcript::check(&self.state.journal, seat)?;

        let mut fork = driver::duplicate(self);
        driver::forget_settings(&mut fork, seat);
        deal.apply(
            &mut fork.state,
            &mut GameRng::stream(policy_seed, BELIEF),
            &self.lookup,
        )?;
        if prior.is_some() {
            // A fact about every deck list, decided again from the fork's
            // own cards.
            fork.state.decide_graveyard_order_from_cards(&self.lookup);
        }
        transcript::withhold(&mut fork.state.journal, &self.state, seat, deal.retired());
        fork.state.rng = GameRng::stream(policy_seed, FUTURE);
        fork.settle_fork(deal.retired());
        Ok(fork)
    }

    /// Rebuilds what the fork derives from its cards: the statics of
    /// hidden cards that work where they are, the layer projection, and the
    /// priority offer that was computed from the real hand.
    ///
    /// The re-sync removes the effects of the cards the deal removed
    /// (`removed`, sorted). Their text overrides go with them, as the end of
    /// the turn would drop them (`GameState::prune_damage_sources`), and so
    /// does an override that quotes a removed card: either is read by nothing
    /// and names the card. No other override goes: a phased-out permanent's
    /// statics are parked, not gone (CR 702.26b), and their overrides come
    /// back with them. An effect is judged gone after the sync, so an id
    /// registered again by it keeps its override.
    fn settle_fork(&mut self, removed: &[ObjectId]) {
        let is_removed = |id: ObjectId| removed.binary_search(&id).is_ok();
        let orphaned: Vec<EffectId> = self
            .state
            .effects
            .iter_with_parked()
            .filter(|effect| effect.source.is_some_and(is_removed))
            .map(|effect| effect.id)
            .collect();
        self.state.invalidate_projections();
        self.sync_static_effects();
        let effects = &self.state.effects;
        self.state.effect_text_overrides.retain(|(id, origin)| {
            let quotes = match *origin {
                TextOrigin::Live(source) | TextOrigin::Ability { source, .. } => {
                    is_removed(source.object)
                }
                TextOrigin::Frozen(_) => false,
            };
            let gone = orphaned.contains(id) && !effects.contains(*id);
            !gone && !quotes
        });
        self.state.refresh_characteristics();
        if let Pending::Priority { player, .. } = self.pending {
            self.pending = Pending::Priority {
                player,
                legal: Box::new(self.compute_legal(player)),
            };
        }
    }
}
