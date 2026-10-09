//! baylee-ai — heuristic AI controllers with difficulty profiles (M3).
//!
//! Controllers answer filtered views and engine offers. The ordinary `act`
//! interface remains usable without privileged data. A trusted host may also
//! supply a selected-effect explanation and an explicitly authorized scouting
//! report; neither is a network message and no agent receives an engine or
//! game-state reference. Scouting exists only for a decision, never in the
//! controller retained when a human takes over an AI chair.

#![warn(missing_docs)]

mod activate;
mod board;
pub mod combat;
mod constrained;
mod copying;
mod damage;
mod fight;
mod filter;
mod granted;
mod held;
pub mod intelligence;
mod policy;
mod redirect;
mod restricted;
pub mod search;
mod tactics;
mod worth;

use baylee_core::ids::{Defender, ObjectId, PlayerId};
pub use baylee_core::preset::AIProfile;
use baylee_core::preset::Politics;
use baylee_engine::choice::{
    ArrangePrompt, ChoicePrompt, Pending, PlayerAction, YesNoPrompt, default_arrangement,
};
use baylee_view::PlayerView;

/// A deterministic controller with hand planning and bounded combat search.
#[derive(Clone, Debug)]
pub struct HeuristicAgent {
    /// Shared policy knobs; the search reads only public combat positions.
    profile: AIProfile,
    /// Which side each seat plays for, in seat order. Empty means a table
    /// with no teams on it, where every seat is a side of its own.
    teams: std::sync::Arc<[Option<u8>]>,
    seed: u64,
    strategy: intelligence::Strategy,
    /// How many of this agent's answers broke the question they answered
    /// and were refitted to it ([`Self::fallbacks`]). Shared by every clone,
    /// so the agent a host seats and the copies it answers through count
    /// into the one number the host reads. It changes no answer.
    fallbacks: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl HeuristicAgent {
    /// A default-profile agent at a table with no teams.
    #[must_use]
    pub fn new(profile: AIProfile) -> Self {
        Self {
            profile,
            teams: std::sync::Arc::default(),
            seed: 0,
            strategy: intelligence::Strategy::default(),
            fallbacks: std::sync::Arc::default(),
        }
    }

    /// How many answers this agent and its clones built that broke the
    /// question they answered (`Pending::answer_fault`), each refitted to
    /// the nearest answer inside it before it was given (`held::refit`) and
    /// logged as a warning.
    ///
    /// Every one is a defect in the picker that built it. The self-play
    /// sweeps assert that this stays at zero: an answer the engine never
    /// refuses because the agent caught it first would otherwise be a
    /// defect nothing reports.
    #[must_use]
    pub fn fallbacks(&self) -> usize {
        self.fallbacks.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// `proposal`, if `pending` takes it, and otherwise the nearest answer
    /// the question takes, counted and logged (`held`).
    fn held_to(
        &self,
        view: &PlayerView,
        pending: &Pending,
        proposal: PlayerAction,
    ) -> PlayerAction {
        let Some(fault) = pending.answer_fault(&proposal) else {
            return proposal;
        };
        self.fallbacks
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let refit = held::refit(view, pending, &proposal, &|seat| {
            self.hostile(seat, view.seat)
        });
        let question: String = match pending {
            // The offer lists every legal action, a page of text.
            Pending::Priority { player, .. } => format!("Priority {{ player: {player:?}, .. }}"),
            other => format!("{other:?}").chars().take(300).collect(),
        };
        log::warn!(
            "house answer {proposal:?} broke its question ({fault:?}) {question}; \
             answering {refit:?} instead"
        );
        refit.unwrap_or(proposal)
    }

    /// Tells the agent which side each seat plays for, in seat order.
    ///
    /// Teams are part of the *setup*, not of the state, which is why they
    /// arrive here rather than in a [`PlayerView`]: they are as public as the
    /// format itself, so this hands the agent nothing a networked seat lacks.
    #[must_use]
    pub fn with_teams(mut self, teams: Vec<Option<u8>>) -> Self {
        self.teams = teams.into();
        self
    }

    /// Seeds near-equal choices independently for each game. Identical
    /// seed, seat, view and choice always produce the same answer.
    #[must_use]
    pub const fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Whether `seat` is an *opponent* of `me` — a different side, not merely
    /// a different seat (CR 102.3).
    ///
    /// The engine offers a teammate's creatures as legal targets, because
    /// they are (CR 115.4); which of the legal ones to take is the agent's
    /// own judgement, and shooting your partner is never it.
    fn hostile(&self, seat: PlayerId, me: PlayerId) -> bool {
        if seat == me {
            return false;
        }
        let side = |p: PlayerId| self.teams.get(p.get() as usize).copied().flatten();
        match (side(seat), side(me)) {
            (Some(a), Some(b)) => a != b,
            _ => true,
        }
    }

    /// Who this seat swings at, per its politics profile.
    ///
    /// In a duel every policy picks the only opponent, so this only starts
    /// to matter at three seats and up — where always taking
    /// `defenders.first()` meant one player absorbed every attack in the game
    /// purely for sitting in the lowest seat.
    fn pick_defender(&self, view: &PlayerView, defenders: &[PlayerId]) -> PlayerId {
        // The monarch, whatever the profile: any creature that connects takes
        // the crown, a card every turn, for this seat (CR 724.2). No policy
        // read it, so at a table of house seats the crown never moved.
        if let Some(monarch) = view.monarch.filter(|m| defenders.contains(m)) {
            return monarch;
        }
        let life = |p: &PlayerId| view.seat(*p).map_or(0, |s| s.life);
        match self.profile.politics {
            // Spread the aggression around without breaking determinism: the
            // view's sequence number is the seed, so the same game always
            // replays the same way. `std::random` here would be a replay bug.
            Politics::Random => {
                let n = view
                    .seq
                    .wrapping_add(u64::from(view.seat.get()) ^ self.seed)
                    .wrapping_mul(0x9E37_79B9_7F4A_7C15);
                defenders[(n >> 33) as usize % defenders.len()]
            }
            // Whoever is closest to winning the race; their board breaks ties.
            Politics::AttackLeader => *defenders
                .iter()
                .max_by_key(|p| (life(p), board_pressure(view, **p)))
                .unwrap_or(&defenders[0]),
            // Archenemy: the biggest board is the threat, however low their
            // life has dropped — a player on 2 life with an empty board is
            // not what loses this game.
            Politics::Archenemy => *defenders
                .iter()
                .max_by_key(|p| (board_pressure(view, **p), life(p)))
                .unwrap_or(&defenders[0]),
        }
    }

    /// Picks an action for the pending choice addressed to `view.seat`.
    #[must_use]
    pub fn act(&self, view: &PlayerView, pending: &Pending) -> PlayerAction {
        self.act_with_context(
            view,
            pending,
            &baylee_engine::engine::DecisionContext::default(),
        )
    }

    /// Answers with the engine's explanation of the selected spell or ability.
    #[must_use]
    pub fn act_with_context(
        &self,
        view: &PlayerView,
        pending: &Pending,
        context: &baylee_engine::engine::DecisionContext<'_>,
    ) -> PlayerAction {
        // Most questions are priority. Borrow its potentially large offer
        // instead of allocating a duplicate box and every legal-action list.
        let proposal = if let Pending::Priority { legal, .. } = pending {
            self.priority(view, legal)
        } else {
            self.choice(view, pending.clone(), context)
        };
        self.held_to(view, pending, proposal)
    }

    /// The attack this seat would like to make, before the rules have their
    /// say: [`combat::obey_attack_rules`] adds what must attack and moves
    /// what may not attack where it was sent.
    fn attack(
        &self,
        view: &PlayerView,
        squad: &[ObjectId],
        defenders: &[Defender],
    ) -> Vec<(ObjectId, Defender)> {
        let opponents: Vec<PlayerId> = defenders
            .iter()
            .filter_map(|d| match d {
                Defender::Player(p) => Some(*p),
                Defender::Planeswalker(_) => None,
            })
            .collect();
        if squad.is_empty() || opponents.is_empty() {
            return Vec::new();
        }
        let victim = self.pick_defender(view, &opponents);
        // Indexed once for the whole decision: every step below
        // looks creatures up by handle, one per creature.
        let board = board::Board::new(view);
        let report = search::attackers_on(&board, squad, victim, self.profile);
        // An attack that wins goes at the player, whatever walker is
        // standing there. The search's proof is one way to know, but
        // it gives up above sixteen creatures a side and never runs
        // for the shallow profiles, so every profile also asks the
        // estimate. Before it did, a squad only the estimate saw as
        // lethal went at the cheapest walker whenever its power
        // reached the loyalty, and a board that doubled every turn
        // killed a recast commander walker every turn and never its
        // controller (self-play r001 #431).
        let lethal = report.lethal || combat::breaks_through(&board, &report.attackers, victim);
        let searched = report.attackers.len();
        let going = combat::hold_back_for_the_crack_back(
            &board,
            report.attackers,
            victim,
            lethal,
            |seat| self.hostile(seat, view.seat),
        );
        if going.is_empty() {
            return Vec::new();
        }
        // What they aim at is decided by the squad that is actually
        // going, not by the whole board: the creatures staying home
        // add nothing to either sum. Where the crack-back pass kept
        // some home, the verdict is the estimate's on what is left.
        let wins =
            lethal && (going.len() == searched || combat::breaks_through(&board, &going, victim));
        if wins && defenders.contains(&Defender::Player(victim)) {
            going
                .into_iter()
                .map(|id| (id, Defender::Player(victim)))
                .collect()
        } else {
            combat::aim(&board, victim, &going, defenders)
        }
    }

    fn priority(
        &self,
        view: &PlayerView,
        legal: &baylee_engine::choice::LegalActions,
    ) -> PlayerAction {
        // 0. Finish a payment the seat has already agreed to (CR 605.3a).
        //
        //    Before everything else because the window is not a turn: the
        //    spell is on the stack and waiting for the price, and every step
        //    below this one is about developing a board. An agent that
        //    played a land here would be answering a different question.
        if let Some(action) = policy::pay_owed(view, legal) {
            return action;
        }
        if view.owed.is_some() && view.awaiting == Some(view.seat) {
            return PlayerAction::PassPriority;
        }
        if let Some(action) = granted::protect(view, legal, |seat| self.hostile(seat, view.seat)) {
            return action;
        }
        // 0b. Spend nothing in this seat's own upkeep on an empty stack.
        //
        //    The step's payments — a pact's price, echo, cumulative upkeep
        //    — are demanded when this round of passes closes (CR 605.3a
        //    lets them be paid with mana abilities then), from whatever is
        //    still untapped. A spell cast here could as well be cast a step
        //    later; a pact whose mana went on one is a game lost.
        //    With something on the stack there is something to answer, and
        //    the round is an ordinary one.
        if view.active == view.seat
            && view.step == baylee_view::Step::Upkeep
            && view.stack.is_empty()
        {
            return PlayerAction::PassPriority;
        }
        // 1. Play a land.
        if let Some(&card) = legal.lands.first() {
            return PlayerAction::PlayLand { card };
        }
        // 2. Crack a fetchland.
        //
        // Before tapping, because the land it finds is mana this
        // turn, and before casting, because step 4 measures what is
        // affordable. A fetchland left alone is not a slow land, it
        // is a land that makes nothing at all — and eight of them
        // sit in the acceptance decks.
        if let Some((source, ability_index)) = activate::fetch(view, legal) {
            return PlayerAction::ActivateAbility {
                source,
                ability_index,
            };
        }
        if let Some(action) = self.spell_or_mana(view, legal) {
            return action;
        }
        // 5. Everything else the policy recognises — a
        //    planeswalker's loyalty, a permanent that draws or
        //    makes a token for a tap.
        //
        //    This used to be a comment saying activated abilities
        //    were skipped because blind activation "loops on free
        //    no-op abilities". That is true of a free ability and
        //    of nothing else: any cost that taps, sacrifices,
        //    discards, exiles or spends alters the state the next
        //    `LegalActions` is built from, so the handle is gone or
        //    unaffordable when the seat next has priority.
        //    `activate::choose` refuses the free shape and takes
        //    the rest by what `worth` says each is worth, net of
        //    its cost.
        if let Some((source, ability_index)) = activate::choose(view, legal, self) {
            return PlayerAction::ActivateAbility {
                source,
                ability_index,
            };
        }
        PlayerAction::PassPriority
    }

    #[allow(clippy::too_many_lines)] // the pending taxonomy is one flat table
    fn choice(
        &self,
        view: &PlayerView,
        pending: Pending,
        context: &baylee_engine::engine::DecisionContext<'_>,
    ) -> PlayerAction {
        let player = view.seat;
        match pending {
            Pending::ChooseManaAbility {
                choice, options, ..
            } => constrained::mana_choice(choice, &options).unwrap_or(PlayerAction::PassPriority),
            ref pending @ (Pending::ChooseDamageSource { .. }
            | Pending::ChooseDamageEffect { .. }
            | Pending::AllocatePrevention { .. }) => {
                damage::answer(view, pending, &|other| self.hostile(player, other))
                    .unwrap_or(PlayerAction::PassPriority)
            }
            Pending::Mulligan {
                taken,
                next_is_free,
                ..
            } => self.mulligan(view, taken, next_is_free),
            Pending::MulliganBottom { count, .. } | Pending::DiscardChoice { count, .. } => {
                // Bottom (or pitch) the highest-cost cards; keep lands and
                // cheap plays.
                PlayerAction::ChooseObjects {
                    objects: self.discard(view, count as usize),
                }
            }
            Pending::Priority { .. } => unreachable!("priority is handled without cloning"),
            // Who may attack and what may be attacked both come from the
            // choice: the engine is the only thing that knows a Wall may
            // not swing (CR 508.1a) and which planeswalker is attackable
            // (CR 508.1b).
            Pending::ChooseAttackers {
                attackers: squad,
                defenders,
                required,
                limits,
                ..
            } => {
                let chosen = self.attack(view, &squad, &defenders);
                PlayerAction::DeclareAttackers {
                    attackers: combat::obey_attack_rules(chosen, &required, &limits, &defenders),
                }
            }
            Pending::ChooseBlockers {
                blockers,
                obeying,
                bounds,
                ..
            } => {
                let mut pairs = combat::obey_block_rules(
                    search::blockers(
                        view,
                        &blockers,
                        view.seat(player).map_or(0, |s| s.life),
                        self.profile,
                    ),
                    &obeying,
                    |attacker| {
                        view.object(attacker).is_some_and(|o| {
                            o.keywords & baylee_cards_dsl::KeywordSet::MENACE.bits() != 0
                        })
                    },
                );
                // Whatever chose them, the blocks are held to the counts the
                // question states (menace, CR 702.111b), which are the ones
                // the engine holds the declaration to.
                combat::keep_bounds(&board::Board::new(view), &blockers, &bounds, &mut pairs);
                PlayerAction::DeclareBlockers { blockers: pairs }
            }
            Pending::LegendChoice { options, .. } => PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
            Pending::ChooseCards {
                options,
                min,
                max,
                prompt,
                total,
                ..
            } => {
                // A total the question states (crew's power, CR 702.122a) is
                // a price the seat already chose to pay: the fewest cards
                // that reach it, by the weights the engine counts.
                if let Some(total) = &total
                    && let Some(objects) = policy::reach_total(&options, min, max, total)
                {
                    return PlayerAction::ChooseObjects { objects };
                }
                if let Some(objects) = self.select_cards(view, &options, min, max, prompt) {
                    return PlayerAction::ChooseObjects { objects };
                }
                // Delve is the one question in this family that is part of a
                // *cost*, and declining a cost is not free. Everything else
                // here may be answered with `min` and nothing is lost;
                // answering delve with zero loses the spell, because
                // `casting::can_cast` counted the graveyard when it put the
                // card in `legal.castable` and the mana is not there without
                // it. A cast that cannot pay is reversed whole (CR 601.2h)
                // and hands priority straight back with the same
                // `LegalActions` — so an agent that declines here casts the
                // same spell again, and again: the harness reports that as
                // `Halt::Repeated` and a real table would sit in it.
                //
                // `max` rather than "as many as are needed" because it *is*
                // as many as are needed: the question is bounded by the
                // generic mana in the spell's total cost, which is the same
                // subtraction `can_cast` made to decide the spell was
                // affordable at all. Taking it is taking the offer the
                // agent was already answering.
                //
                // The untap step's determination is the one question in the
                // family where `max` is the *harmful* answer: every
                // permanent named there stays tapped, so the `max <= 2`
                // shortcut would have left a storage land tapped for the
                // rest of the game. Untapping costs nothing, which is what
                // `min` says. Whether a storage land is worth leaving
                // tapped is a real judgement and not one this heuristic
                // makes.
                // Separating piles for an opponent (Fact or Fiction): one
                // card alone, so whichever pile they take, they do not take
                // all of them, which is what naming none would hand over.
                let n = match prompt {
                    // Under an untap limit the answer is what untaps.
                    ChoicePrompt::Delve | ChoicePrompt::Untap => max,
                    // A band (CR 702.22c) is a judgement this heuristic does
                    // not make: blocking one member blocks them all, which
                    // can cost a flier its evasion, and the menu may hold
                    // two creatures without banding, which one band cannot.
                    // Attacking unbanded is always legal.
                    ChoicePrompt::LeaveTapped | ChoicePrompt::Band { .. } => min,
                    ChoicePrompt::FirstPile => max.min(1),
                    _ if max <= 2 => max,
                    _ => min,
                };
                PlayerAction::ChooseObjects {
                    objects: options[..(n as usize).min(options.len())].to_vec(),
                }
            }
            // This prompt pays for a cast, despite sharing the target-choice
            // shape. Selecting one friendly permanent too few can underpay
            // and roll the whole cast back forever (paired match seed 41), so
            // the count is measured against the cast's price and the pool.
            Pending::ChooseTargets {
                options,
                max,
                reason: baylee_engine::choice::TargetPrompt::Convoke,
                ..
            } => PlayerAction::ChooseTargets {
                objects: policy::convoke_taps(view, context, options, max),
                players: vec![],
            },
            Pending::ChooseTargets {
                options,
                player_options,
                min,
                max,
                ..
            } => {
                // Concentrate evenly divided damage on one target. Splitting
                // it weakens each hit and may require an additional payment.
                let max = if min <= 1
                    && context.effects.iter().any(|effect| {
                        matches!(effect, baylee_cards_dsl::Effect::DealDamageEvenly { .. })
                    }) {
                    max.min(1)
                } else {
                    max
                };
                if let Some(action) =
                    self.targets(view, &options, &player_options, min, max, context)
                {
                    return action;
                }
                // An opponent's permanents first, everything else after: the
                // count may force a teammate's creature (a spell with two
                // required targets and one enemy on the board is still cast),
                // but nothing else may.
                let mut ordered: Vec<ObjectId> = options
                    .iter()
                    .copied()
                    .filter(|id| {
                        view.object(*id)
                            .is_none_or(|o| self.hostile(o.controller, player))
                    })
                    .collect();
                let enemies = ordered.len();
                for id in &options {
                    if !ordered.contains(id) {
                        ordered.push(*id);
                    }
                }
                // How many to name. The bounds are the card's own words:
                // "up to two" is 0..=2, "two target creatures" is 2..=2, and
                // "any number of target creatures" is 0..=99.
                //
                // Taking `min` for that last one takes *none*, and a
                // no-target answer to "any number" is what stalled a game at
                // turn 34: the engine put the cast back, the board was
                // unchanged, so the agent cast the same spell again and the
                // harness's loop detector ended the game. A spell was cast to
                // do something, so an open choice takes every enemy it was
                // offered, never fewer than one and never more than `max`:
                // "up to four" over eleven enemy Illusions named all eleven,
                // and the engine refused the answer as too many (the
                // trained AI's fuzzer, main 50050ff3, seeds 486 and 1931).
                let n = if max <= 2 {
                    usize::try_from(max).unwrap_or(usize::MAX)
                } else if min == 0 {
                    enemies.clamp(1, usize::try_from(max).unwrap_or(usize::MAX))
                } else {
                    usize::try_from(min).unwrap_or(usize::MAX)
                };
                let objects = ordered[..n.min(ordered.len())].to_vec();
                // "Any target" with nothing on the battlefield worth hitting
                // is still a legal spell: the rest of the count comes off the
                // face. Aiming at an opponent rather than the first player in
                // the list is the whole of the heuristic here — a burn spell
                // pointed at its own controller would be a bug that only ever
                // shows up as the AI losing.
                let want = n.saturating_sub(objects.len());
                let mut players: Vec<_> = Vec::new();
                for seat in player_options
                    .iter()
                    .filter(|p| self.hostile(**p, player))
                    .chain(player_options.iter().filter(|p| **p != player))
                    .chain(player_options.iter())
                {
                    // Targets are distinct (CR 601.2c), so naming a seat
                    // twice is not "two targets" — it is an illegal answer
                    // that would be counted as two and resolve as one.
                    if players.len() >= want {
                        break;
                    }
                    if !players.contains(seat) {
                        players.push(*seat);
                    }
                }
                PlayerAction::ChooseTargets { objects, players }
            }
            Pending::ChooseSubtype { options, .. } => {
                PlayerAction::ChooseSubtype(self.subtype(view, &options))
            }
            Pending::ChooseCardName { .. } => {
                let (card, face) = self.card_name(view);
                PlayerAction::ChooseCardName { card, face }
            }

            Pending::ChooseColor { options, .. } => {
                PlayerAction::ChooseColor(self.color(view, &options, context))
            }
            Pending::ChooseNumber {
                min, max, reason, ..
            } => PlayerAction::ChooseNumber(match reason {
                baylee_engine::choice::NumberPrompt::TextReplacement { kind, target } => {
                    constrained::text_word(view, kind, target).clamp(min, max)
                }
                // The cost was already paid. Evaluate the offered counters,
                // without trying to buy X again from the remaining mana pool.
                baylee_engine::choice::NumberPrompt::Counters { target, kind } => {
                    if view
                        .object(target)
                        .and_then(|object| self.counters(view, object, kind, 1))
                        .is_some_and(|worth| worth > 0)
                    {
                        max
                    } else {
                        min
                    }
                }
                baylee_engine::choice::NumberPrompt::X => self.number(view, min, max, context),
                baylee_engine::choice::NumberPrompt::ManaPayment { preventable_damage } => {
                    preventable_damage.clamp(min, max)
                }
                // Every payment the engine offers: it bounded the count by
                // what the floating pool pays beside the rest of the cast,
                // and each payment is the spell once more (CR 702.56a).
                baylee_engine::choice::NumberPrompt::Replicate { .. } => max,
                // A share of a division: what the target needs to die, if it
                // is an opponent's, within what the question allows; the
                // least for anything else.
                baylee_engine::choice::NumberPrompt::DivideDamage { target, .. } => {
                    self.damage_share(view, target, min, max)
                }
                // A creature's combat damage divided among two or more
                // (CR 510.1c–d, 702.22j–k): what finishes each creature in
                // turn, and the last takes the rest.
                baylee_engine::choice::NumberPrompt::CombatDamage {
                    source, recipient, ..
                } => self.combat_share(view, source, recipient, min, max),
            }),
            Pending::ChoosePlayer { options, .. } => {
                PlayerAction::ChoosePlayer(self.player_target(view, &options, context))
            }
            Pending::ChooseCastMode {
                object, options, ..
            } => PlayerAction::ChooseMode(self.cast_mode(view, object, &options)),
            Pending::ChoosePile { piles, .. } => PlayerAction::ChooseMode(self.pile(view, &piles)),
            // An order the AI has no opinion on yet: the cards as they were
            // offered, every pile filled to its minimum first. It is always
            // an answer, because the engine never asks an arrangement whose
            // piles cannot hold its cards.
            // A scry or a surveil: what the policy would send away goes to
            // the second pile and the rest stays on top as it lay. With no
            // opinion — every level below the one that reads cards — nothing
            // moves, and for a surveil that is the point: a card put into a
            // graveyard does not come back, and deciding a card is bad
            // enough to bin needs to know what it is.
            Pending::Arrange {
                cards,
                piles,
                prompt: ArrangePrompt::Scry | ArrangePrompt::Surveil,
                ..
            } if piles.len() == 2 => {
                let away = self.send_away(view, &cards).unwrap_or_default();
                let top = cards
                    .iter()
                    .copied()
                    .filter(|c| !away.contains(c))
                    .collect();
                PlayerAction::Arrange {
                    piles: vec![top, away],
                }
            }
            // Library of Leng: a card on top of the library comes back as
            // the next draw, and one in the graveyard is gone for the game,
            // so every card this seat is losing goes on top, as it was
            // offered. A card worth throwing away needs the card reader
            // this agent does not have.
            Pending::Arrange {
                cards,
                piles,
                prompt: ArrangePrompt::DiscardToLibrary,
                ..
            } if piles.len() == 2 => PlayerAction::Arrange {
                piles: vec![Vec::new(), cards],
            },
            Pending::Arrange { cards, piles, .. } => PlayerAction::Arrange {
                piles: default_arrangement(&cards, &piles).unwrap_or_else(|| vec![cards.clone()]),
            },
            Pending::YesNo { prompt, .. } => match prompt {
                YesNoPrompt::PayLifeOrEnterTapped { amount } | YesNoPrompt::PayLife { amount } => {
                    PlayerAction::YesNo(
                        view.seat(player)
                            .is_some_and(|s| s.life > i32::from(amount) + 5),
                    )
                }
                YesNoPrompt::Miracle { card } => PlayerAction::YesNo(Self::miracle(view, card)),
                // What refusing a tax costs is not the same question for
                // every tax, so it is asked of the effect rather than of the
                // prompt: ward counters the spell this seat has just cast.
                YesNoPrompt::PayTax { mana } => PlayerAction::YesNo(policy::pays_tax(
                    view,
                    baylee_core::mana::ManaCost::ZERO.with_more_generic(u32::from(mana)),
                    context,
                )),
                YesNoPrompt::PayMana { cost } => {
                    PlayerAction::YesNo(policy::pays_tax(view, cost, context))
                }
                // Kicker and "you may waterbend" alike: paid when the pool
                // already covers it, because the engine pays from the pool
                // alone and a short one loses the whole cast.
                YesNoPrompt::Kicker => PlayerAction::YesNo(policy::kicks(view, context)),
                // A draw is declined because the house AI has no match score
                // to protect, so accepting would only ever be a game given
                // away. Untapping at the price of a whole turn needs a
                // turn-trade evaluator, not the free optional-effect default.
                YesNoPrompt::DrawOffer { .. } | YesNoPrompt::SkipTurn { .. } => {
                    PlayerAction::YesNo(false)
                }
                // Both yes, for reasons that happen to agree. An optional
                // effect is written on a card this seat chose to play, so
                // taking it is the default. And a commander goes home
                // (CR 903.9a) because the command zone is the one zone
                // nobody can reach into, and the {2} on the next cast is
                // cheaper than the deck's whole plan being milled or
                // exiled — a seat that would rather reanimate it needs the
                // evaluator this agent does not have yet. And the top of the
                // library for a card of this seat's that somebody else's
                // ability is sending away: on top it is the next draw, on
                // the bottom it is gone for the game. And a discovered card
                // (CR 701.57a): it is only offered when it can be cast, and a
                // spell for nothing is worth more than the card in hand.
                // A spell for nothing (cascade) is taken too. So is a card
                // this seat's own ability offered to cast (Conduit of
                // Worlds): the activation was the choice, and a window it
                // cannot fill casts nothing and costs nothing.
                YesNoPrompt::MayDo
                | YesNoPrompt::CommanderZone { .. }
                | YesNoPrompt::TopOfLibrary { .. }
                | YesNoPrompt::Discover { .. }
                | YesNoPrompt::CastWithoutPaying { .. }
                | YesNoPrompt::CastPaying { .. }
                | YesNoPrompt::Generic
                // A pact: attempt payment; the owed-mana planner handles the
                // window.
                | YesNoPrompt::PayPact { .. } => PlayerAction::YesNo(true),
                // CR 903.9b answers itself from the destination, which is
                // why the prompt carries it. A library is the same loss the
                // graveyard would have been, so it goes home. A *hand* is
                // strictly better than the command zone: the card is just
                // as castable and CR 903.8 taxes only the command zone, so
                // taking the redirect there would be paying {2} for
                // nothing.
                YesNoPrompt::CommanderReplace { to_library, .. } => PlayerAction::YesNo(to_library),
            },
            Pending::GameOver(_) => PlayerAction::PassPriority, // unreachable in the driver
        }
    }
}

/// The `n` costliest cards in the seat's own hand.
fn costliest(view: &PlayerView, n: usize) -> Vec<ObjectId> {
    let mut hand: Vec<(u32, ObjectId)> = baylee_client_core::decision::hand(view)
        .iter()
        .map(|card| (card.mana_value, card.id))
        .collect();
    hand.sort_by_key(|(mv, id)| (u32::MAX - mv, *id));
    hand.iter().take(n).map(|(_, id)| *id).collect()
}

/// How much a player's board threatens: a point per permanent plus its
/// power, which reads an army of small creatures and one huge one as
/// comparably dangerous.
fn board_pressure(view: &PlayerView, player: PlayerId) -> i32 {
    view.battlefield_of(player)
        .map(|o| 1 + i32::from(o.power.unwrap_or(0)))
        .sum()
}

/// Mana floating in the acting seat's pool (cmc units).
/// Which version of the policy-seed derivation this is.
///
/// Inside the hash rather than beside it, so a change to the recipe changes
/// every seed it produces: a seed is *recorded* — a replay has to reproduce
/// the chair as well as the shuffle — and two recipes agreeing on a value by
/// accident would be a replay that silently plays a different game.
pub const POLICY_SEED_VERSION: u64 = 1;

/// The randomness an AI chair plays with, derived from what the whole table
/// can already see.
///
/// Not the game's seed. That stream dealt the hands and shuffled the
/// libraries, and an agent drawing from it is correlated with the hidden
/// state it is supposed to be guessing at — a leak no seat boundary catches,
/// because nothing crosses one (#87). For a heuristic that only breaks ties
/// it is untidy; for anything that samples a belief it is the whole problem.
/// The invariant this exists to make true: with the same authorized
/// observations, the same policy seed and the same budget, changing the real
/// hidden state or the real RNG cannot change the answer.
///
/// `game` is the **public** identifier a host already tells every seat, so
/// two tables differ and two runs of one table do not. The seat goes in as a
/// fixed one-byte suffix, which is what makes `game ‖ seat` unambiguous
/// without a length or a separator: every byte before the last belongs to
/// the identifier, so no two pairs can write the same input. FNV-1a and not
/// `DefaultHasher`: the latter's algorithm is stable only within a process,
/// so a recorded seed would drift on a toolchain bump.
#[must_use]
pub fn policy_seed(game: &str, seat: u8) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    POLICY_SEED_VERSION
        .to_le_bytes()
        .iter()
        .chain(game.as_bytes())
        .chain(std::iter::once(&seat))
        .fold(OFFSET, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(PRIME)
        })
}

/// What the seat could make by tapping, as the house reads it: the offered,
/// simple, unrestricted mana taps, one source per permanent, free modes
/// before priced ones.
///
/// The reader the agent plans its own casts with, handed out so a seat that
/// is not the house can ask the same question — "could this hand be paid for
/// from what is untapped?" — without a second reader that disagrees with
/// this one (the seat bridge's standing orders, `baylee-seat`). What it
/// leaves out is left out for every caller alike: restricted mana, which
/// can pay only for some spells, and any ability whose price is more than a
/// tap it cannot plan.
#[must_use]
pub fn mana_sources(
    view: &PlayerView,
    legal: &baylee_engine::choice::LegalActions,
) -> Vec<baylee_client_core::manaplan::Source> {
    policy::sources(view, legal)
}

/// Whether an offered activation can do nothing but make mana: a mana
/// ability (CR 605.1a) whose price is its tap and mana at most, or a mana
/// ability a permanent is granted, which costs its tap by construction.
///
/// The engine offers every mana ability at every priority (CR 605.3a), so
/// an offer of nothing else reads as something to do and is not: mana made
/// with nothing to spend it on empties as the step ends (CR 500.5). A mana
/// ability with a price beyond the tap (a sacrifice, life, a discard) is not
/// one of these, because selling a creature for mana in answer to a removal
/// spell is a decision. Read by the same lookup the agent activates with
/// (the object's printed list, a copy's by what it copies).
#[must_use]
pub fn only_makes_mana(view: &PlayerView, object: ObjectId, index: u32) -> bool {
    if let Some(slot) = baylee_engine::choice::granted_slot(index) {
        return view
            .object(object)
            .and_then(|o| o.granted_mana.as_ref())
            .is_some_and(|mana| mana.slot == slot);
    }
    match activate::printed(view, object, index) {
        Some(
            baylee_cards_dsl::AbilityDef::Activated {
                cost,
                mana_ability: true,
                ..
            }
            | baylee_cards_dsl::AbilityDef::ActivatedConditional {
                cost,
                mana_ability: true,
                ..
            },
        ) => !policy::priced(cost),
        _ => false,
    }
}

/// The player who must answer a pending choice.
#[must_use]
pub fn pending_player(pending: &Pending) -> Option<PlayerId> {
    match pending {
        Pending::ChooseManaAbility { player, .. }
        | Pending::Mulligan { player, .. }
        | Pending::MulliganBottom { player, .. }
        | Pending::Priority { player, .. }
        | Pending::ChooseAttackers { player, .. }
        | Pending::ChooseBlockers { player, .. }
        | Pending::DiscardChoice { player, .. }
        | Pending::LegendChoice { player, .. }
        | Pending::ChooseCards { player, .. }
        | Pending::ChooseTargets { player, .. }
        | Pending::ChooseSubtype { player, .. }
        | Pending::ChooseCardName { player }
        | Pending::ChooseColor { player, .. }
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
