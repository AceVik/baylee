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
pub mod combat;
mod filter;
pub mod intelligence;
mod policy;
pub mod search;
mod tactics;

use baylee_core::ids::{Defender, ObjectId, PlayerId};
pub use baylee_core::preset::AIProfile;
use baylee_core::preset::Politics;
use baylee_engine::choice::{ChoicePrompt, Pending, PlayerAction, YesNoPrompt};
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
        }
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
        // Most questions are priority. Borrow its potentially large offer
        // instead of allocating a duplicate box and every legal-action list.
        if let Pending::Priority { legal, .. } = pending {
            return self.priority(view, legal);
        }
        self.choice(
            view,
            pending.clone(),
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
        if let Pending::Priority { legal, .. } = pending {
            return self.priority(view, legal);
        }
        self.choice(view, pending.clone(), context)
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
        //    the rest by an explicit whitelist.
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
                ..
            } => {
                let opponents: Vec<PlayerId> = defenders
                    .iter()
                    .filter_map(|d| match d {
                        Defender::Player(p) => Some(*p),
                        Defender::Planeswalker(_) => None,
                    })
                    .collect();
                if squad.is_empty() || opponents.is_empty() {
                    return PlayerAction::DeclareAttackers { attackers: vec![] };
                }
                let victim = self.pick_defender(view, &opponents);
                let report = search::attackers(view, &squad, victim, self.profile);
                let going = report.attackers;
                if going.is_empty() {
                    return PlayerAction::DeclareAttackers { attackers: vec![] };
                }
                // What they aim at is decided by the squad that is actually
                // going, not by the whole board: a walker is only worth
                // attacking when the attack kills it, and the creatures
                // staying home add nothing to that sum.
                let defender = if report.lethal && defenders.contains(&Defender::Player(victim)) {
                    Defender::Player(victim)
                } else {
                    aim_at(view, victim, &going, &defenders)
                };
                let attackers = going.into_iter().map(|id| (id, defender)).collect();
                PlayerAction::DeclareAttackers { attackers }
            }
            Pending::ChooseBlockers { blockers, .. } => PlayerAction::DeclareBlockers {
                blockers: search::blockers(
                    view,
                    &blockers,
                    view.seat(player).map_or(0, |s| s.life),
                    self.profile,
                ),
            },
            Pending::LegendChoice { options, .. } => PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
            Pending::ChooseCards {
                options,
                min,
                max,
                prompt,
                ..
            } => {
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
                // A surveil is the second, and it is worse than the untap
                // step's: a card put into the graveyard does not come back,
                // and a surveil is *usually* 1 or 2, so the `max <= 2`
                // shortcut would have milled the top of this agent's own
                // library every time a surveil resolved. Keeping the card
                // is always legal and costs nothing; deciding a card is bad
                // enough to bin needs to know what it is, which this
                // heuristic does not.
                let n = match prompt {
                    ChoicePrompt::Delve => max,
                    ChoicePrompt::LeaveTapped | ChoicePrompt::SurveilGraveyard => min,
                    _ if max <= 2 => max,
                    _ => min,
                };
                PlayerAction::ChooseObjects {
                    objects: options[..(n as usize).min(options.len())].to_vec(),
                }
            }
            // This prompt pays for a cast, despite sharing the target-choice
            // shape. Selecting one friendly permanent can underpay and roll
            // the whole cast back forever (paired match seed 41). Use the
            // offered reduction, as with delve, until the view carries the
            // outstanding cost needed to reserve any of these permanents.
            Pending::ChooseTargets {
                options,
                max,
                reason: baylee_engine::choice::TargetPrompt::Convoke,
                ..
            } => PlayerAction::ChooseTargets {
                objects: options.into_iter().take(usize::from(max)).collect(),
                players: vec![],
            },
            Pending::ChooseTargets {
                options,
                player_options,
                min,
                max,
                ..
            } => {
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
                // do something, so an unbounded choice takes every enemy it
                // was offered, and never fewer than one.
                let n = if max <= 2 {
                    max as usize
                } else if min == 0 {
                    enemies.max(1)
                } else {
                    min as usize
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

            Pending::ChooseColor { options, .. } => {
                PlayerAction::ChooseColor(self.color(view, &options))
            }
            Pending::ChooseNumber { min, max, .. } => {
                PlayerAction::ChooseNumber(self.number(view, min, max, context))
            }
            Pending::ChoosePlayer { options, .. } => {
                PlayerAction::ChoosePlayer(self.player_target(view, &options, context))
            }
            Pending::ChooseCastMode {
                object, options, ..
            } => PlayerAction::ChooseMode(self.cast_mode(view, object, &options)),
            Pending::OrderObjects { objects, .. } => PlayerAction::OrderObjects { objects },
            Pending::YesNo { prompt, .. } => match prompt {
                YesNoPrompt::PayLifeOrEnterTapped { amount } => PlayerAction::YesNo(
                    view.seat(player)
                        .is_some_and(|s| s.life > i32::from(amount) + 5),
                ),
                YesNoPrompt::Miracle { card } => PlayerAction::YesNo(Self::miracle(view, card)),
                // What refusing a tax costs is not the same question for
                // every tax, so it is asked of the effect rather than of the
                // prompt: ward counters the spell this seat has just cast.
                YesNoPrompt::PayTax { mana } => {
                    PlayerAction::YesNo(policy::pays_tax(view, mana, context))
                }
                // Kicker is declined to keep the mana; a draw is declined
                // because the house AI has no match score to protect, so
                // accepting would only ever be a game given away.
                YesNoPrompt::Kicker | YesNoPrompt::DrawOffer { .. } => PlayerAction::YesNo(false),
                // Both yes, for reasons that happen to agree. An optional
                // effect is written on a card this seat chose to play, so
                // taking it is the default. And a commander goes home
                // (CR 903.9a) because the command zone is the one zone
                // nobody can reach into, and the {2} on the next cast is
                // cheaper than the deck's whole plan being milled or
                // exiled — a seat that would rather reanimate it needs the
                // evaluator this agent does not have yet.
                YesNoPrompt::MayDo | YesNoPrompt::CommanderZone { .. } | YesNoPrompt::Generic => {
                    PlayerAction::YesNo(true)
                }
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
    let mut hand: Vec<(u32, ObjectId)> = view
        .hand
        .iter()
        .map(|card| (card.mana_value, card.id))
        .collect();
    hand.sort_by_key(|(mv, id)| (u32::MAX - mv, *id));
    hand.iter().take(n).map(|(_, id)| *id).collect()
}

/// Chooses what the squad actually swings at once politics has picked the
/// victim: one of their planeswalkers if this attack can finish it off,
/// otherwise the player.
///
/// Killing a walker is worth more than a few points of life, but only if
/// it actually dies — chipping a loyalty counter off a big planeswalker
/// while the controller's life total goes untouched is the worst of both.
/// So the bar is "total attacking power is at least its loyalty", and
/// among the walkers that clear it the cheapest one to kill wins.
///
/// The blockers the defender has not declared yet are not modelled; this
/// is the same one-ply optimism the rest of the heuristic runs on.
fn aim_at(
    view: &PlayerView,
    victim: PlayerId,
    squad: &[ObjectId],
    defenders: &[Defender],
) -> Defender {
    let power: i32 = squad
        .iter()
        .filter_map(|id| view.object(*id))
        .map(|o| i32::from(o.power.unwrap_or(0)))
        .sum();
    defenders
        .iter()
        .copied()
        .filter_map(|d| {
            let Defender::Planeswalker(id) = d else {
                return None;
            };
            let walker = view.object(id)?;
            if walker.controller != victim {
                return None;
            }
            let loyalty = i32::from(walker.counter_count(baylee_view::CounterKind::Loyalty));
            (loyalty > 0 && loyalty <= power).then_some((loyalty, d))
        })
        .min_by_key(|(loyalty, _)| *loyalty)
        .map_or(Defender::Player(victim), |(_, d)| d)
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
/// The player who must answer a pending choice.
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
    use baylee_cards_dsl::AbilityDef;
    use baylee_core::color::ColorSet;
    use baylee_core::types::{SubtypeSet, SupertypeSet, TypeSet};
    use baylee_view::{
        CombatView, CounterEntry, CounterKind, ObjectStatus, PlayerView, PublicObject, SeatView,
    };

    fn obj(slot: u32) -> ObjectId {
        ObjectId::new(slot, 0)
    }

    fn hand_card(slot: u32, name: &str) -> baylee_view::HandObject {
        let index = baylee_cards::decks::by_name(name).unwrap();
        let face = &baylee_cards::by_index(index).unwrap().faces[0];
        baylee_view::HandObject {
            id: obj(slot),
            card: baylee_view::CardIdentity {
                index,
                print: baylee_core::ids::PrintRef::new(0),
                face: 0,
            },
            name: name.into(),
            mana_value: face.mana_cost.cmc(),
            colors: face.mana_cost.colors(),
            types: face.types,
            commander: false,
        }
    }

    #[test]
    fn third_iteration_subtype_follows_the_cards_being_played() {
        use baylee_core::generated::subtypes::creature::{ALLY, BIRD};
        let mut v = view(0, &[20, 20], vec![]);
        v.hand = vec![hand_card(1, "Baleful Strix")];
        assert_eq!(
            HeuristicAgent::new(AIProfile::EXPERT).act(
                &v,
                &Pending::ChooseSubtype {
                    player: v.seat,
                    options: vec![ALLY, BIRD]
                }
            ),
            PlayerAction::ChooseSubtype(BIRD)
        );
    }

    #[test]
    fn third_iteration_jace_bounces_a_threat_instead_of_blindly_ticking_up() {
        let jace = carded(
            permanent(obj(1), PlayerId::new(0), 0),
            "Jace, the Mind Sculptor",
            TypeSet::PLANESWALKER,
        );
        let v = view(
            0,
            &[5, 20],
            vec![jace, permanent(obj(2), PlayerId::new(1), 6)],
        );
        let legal = baylee_engine::choice::LegalActions {
            abilities: vec![(obj(1), 0), (obj(1), 1), (obj(1), 2)],
            ..Default::default()
        };
        assert_eq!(
            HeuristicAgent::new(AIProfile::EXPERT).act(
                &v,
                &Pending::Priority {
                    player: v.seat,
                    legal: Box::new(legal)
                }
            ),
            PlayerAction::ActivateAbility {
                source: obj(1),
                ability_index: 2
            }
        );
    }

    /// A price the player may decline is a price this agent pays.
    ///
    /// "… unless you return a land you control to its owner's hand" asks
    /// with `min: 0`, because naming nothing *is* the refusal — so an agent
    /// answering it at `min` would sacrifice every Karoo land it played,
    /// and silently, since declining is a legal answer and nothing logs it.
    ///
    /// It does not, and **not because the prompt is handled**:
    /// `ChoicePrompt::CostReturn` and `CostTap` reach neither arm of
    /// `policy::select_cards` and fall out of its `_`, and the answer comes
    /// from the fallback's `_ if max <= 2 => max`. That is a correct outcome
    /// resting on an unrelated shortcut, which is exactly the kind of thing
    /// that is right until somebody tidies it. Adding the two prompts to
    /// the policy was tried and reverted: with every option a land of the
    /// same rank, the ordering it would impose is the one already there,
    /// and a change no test can see fall is not a change.
    ///
    /// Both directions are checked, because "pay it" alone would pass on an
    /// agent that pays every cost question at `max`: an activation cost
    /// asks with `min: 1` and must still take one permanent, not two.
    #[test]
    fn a_price_that_may_be_declined_is_paid_and_an_activation_cost_is_not_overpaid() {
        use baylee_engine::choice::ChoicePrompt;
        let v = view(
            0,
            &[20, 20],
            vec![
                permanent(obj(1), PlayerId::new(0), 0),
                permanent(obj(2), PlayerId::new(0), 0),
            ],
        );
        for prompt in [ChoicePrompt::CostReturn, ChoicePrompt::CostTap] {
            let action = HeuristicAgent::new(AIProfile::EXPERT).act(
                &v,
                &Pending::ChooseCards {
                    player: v.seat,
                    options: vec![obj(1), obj(2)],
                    min: 0,
                    max: 1,
                    prompt,
                },
            );
            let PlayerAction::ChooseObjects { objects } = action else {
                panic!("expected a card choice for {prompt:?}, got {action:?}")
            };
            assert_eq!(
                objects.len(),
                1,
                "{prompt:?} is a price worth paying, and naming nothing pays none of it"
            );
        }

        let action = HeuristicAgent::new(AIProfile::EXPERT).act(
            &v,
            &Pending::ChooseCards {
                player: v.seat,
                options: vec![obj(1), obj(2)],
                min: 1,
                max: 2,
                prompt: ChoicePrompt::CostSacrifice,
            },
        );
        let PlayerAction::ChooseObjects { objects } = action else {
            panic!("expected a card choice, got {action:?}")
        };
        assert_eq!(
            objects.len(),
            1,
            "an activation cost takes what it asks for and not one permanent more"
        );
    }

    #[test]
    fn third_iteration_counter_sign_decides_which_team_to_target() {
        use baylee_cards_dsl::{Amount, CounterKind as Counter, Effect};
        use baylee_engine::engine::DecisionContext;
        let v = view(
            0,
            &[20, 20],
            vec![
                permanent(obj(1), PlayerId::new(1), 6),
                permanent(obj(2), PlayerId::new(0), 4),
            ],
        );
        let pending = Pending::ChooseTargets {
            player: v.seat,
            options: vec![obj(1), obj(2)],
            player_options: vec![],
            min: 1,
            max: 1,
            reason: baylee_engine::choice::TargetPrompt::Targets,
        };
        for (kind, expected) in [(Counter::P1P1, obj(2)), (Counter::M1M1, obj(1))] {
            let effects = [Effect::AddCounter {
                kind,
                amount: Amount::Fixed(1),
            }];
            let context = DecisionContext {
                effects: &effects,
                ..Default::default()
            };
            assert_eq!(
                HeuristicAgent::new(AIProfile::EXPERT).act_with_context(&v, &pending, &context),
                PlayerAction::ChooseTargets {
                    objects: vec![expected],
                    players: vec![]
                }
            );
        }
    }

    /// A Roaming Throne with ward {2} beside a plain 2/2, and one Plains.
    ///
    /// The Throne is the better card to kill and the agent used to say so
    /// and nothing else: material ranked it first, the ward countered the
    /// removal, and the 2/2 that could have been exiled for free was still
    /// there afterwards. Targets are chosen before mana is paid (CR 601.2),
    /// so the seat has to cover the spell and the tax out of the same
    /// untapped lands — with a third Plains it can, and then the bigger
    /// threat is worth the toll again.
    #[test]
    fn removal_goes_around_a_ward_it_cannot_pay_and_through_one_it_can() {
        use baylee_cards_dsl::{Effect, Filter, TargetSpec};
        use baylee_engine::engine::DecisionContext;
        let throne = carded(
            permanent(obj(1), PlayerId::new(1), 4),
            "Roaming Throne",
            TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        );
        let bear = permanent(obj(2), PlayerId::new(1), 2);
        let plains = |id| {
            let mut land = carded(permanent(id, PlayerId::new(0), 0), "Plains", TypeSet::LAND);
            land.subtypes
                .insert(baylee_core::generated::subtypes::land::PLAINS);
            land.power = None;
            land.toughness = None;
            land
        };
        let pending = Pending::ChooseTargets {
            player: PlayerId::new(0),
            options: vec![obj(1), obj(2)],
            player_options: vec![],
            min: 1,
            max: 1,
            reason: baylee_engine::choice::TargetPrompt::Targets,
        };
        let effects = [Effect::Exile {
            target: TargetSpec::Object(&Filter::CREATURE),
        }];
        let swords = DecisionContext {
            effects: &effects,
            cost: Some(baylee_core::mana::ManaCost::parse("{W}")),
            ..Default::default()
        };

        let one = view(
            0,
            &[20, 20],
            vec![throne.clone(), bear.clone(), plains(obj(3))],
        );
        assert_eq!(
            agent().act_with_context(&one, &pending, &swords),
            PlayerAction::ChooseTargets {
                objects: vec![obj(2)],
                players: vec![]
            },
            "one Plains pays for the spell and not for ward {{2}}, so the \
             Throne is a card thrown away and the bear is a creature exiled"
        );

        let three = view(
            0,
            &[20, 20],
            vec![throne, bear, plains(obj(3)), plains(obj(4)), plains(obj(5))],
        );
        assert_eq!(
            agent().act_with_context(&three, &pending, &swords),
            PlayerAction::ChooseTargets {
                objects: vec![obj(1)],
                players: vec![]
            },
            "with the tax covered the bigger threat is worth {{2}} again"
        );
    }

    /// Ward's question arrives at the caster as `YesNoPrompt::PayTax`, and
    /// the agent used to answer it in the same arm as a kicker and an
    /// offered draw: no, always. Declining a kicker costs nothing and
    /// declining this counters the agent's own spell — with two untapped
    /// lands sitting on the table.
    ///
    /// What separates the two is not the prompt but what refusing it does,
    /// which the resolving effect says out loud: ward's alternative counters
    /// the spell, a Rhystic tax's gives an opponent a card. The second is
    /// still declined here, because spare mana and needed mana look alike to
    /// a stateless policy and a card is the cheaper of the two to give up.
    #[test]
    fn a_warded_spell_is_paid_for_instead_of_being_countered() {
        use baylee_cards_dsl::{Amount, Effect, PlayerRel};
        use baylee_engine::engine::DecisionContext;
        let mut forest = carded(
            permanent(obj(1), PlayerId::new(0), 0),
            "Forest",
            TypeSet::LAND,
        );
        forest
            .subtypes
            .insert(baylee_core::generated::subtypes::land::FOREST);
        let mut island = carded(
            permanent(obj(2), PlayerId::new(0), 0),
            "Island",
            TypeSet::LAND,
        );
        island
            .subtypes
            .insert(baylee_core::generated::subtypes::land::ISLAND);
        let pending = Pending::YesNo {
            player: PlayerId::new(0),
            prompt: YesNoPrompt::PayTax { mana: 2 },
            source: None,
        };
        let ward = [Effect::PlayerMayPayOr {
            player: PlayerRel::ControllerOfTarget,
            mana: Amount::Fixed(2),
            effect: &Effect::CounterTargetSpellOrAbility,
        }];
        let ward = DecisionContext {
            effects: &ward,
            ..Default::default()
        };

        let v = view(0, &[20, 20], vec![forest, island]);
        assert_eq!(
            agent().act_with_context(&v, &pending, &ward),
            PlayerAction::YesNo(true),
            "two untapped lands and the spell dies if the tax goes unpaid"
        );

        let bare = view(0, &[20, 20], vec![]);
        assert_eq!(
            agent().act_with_context(&bare, &pending, &ward),
            PlayerAction::YesNo(false),
            "nothing to tap, so promising the mana only spends the window"
        );

        let rhystic = [Effect::PlayerMayPayOr {
            player: PlayerRel::ControllerOfTarget,
            mana: Amount::Fixed(2),
            effect: &Effect::DrawCards {
                amount: Amount::Fixed(1),
            },
        }];
        let rhystic = DecisionContext {
            effects: &rhystic,
            ..Default::default()
        };
        assert_eq!(
            agent().act_with_context(&v, &pending, &rhystic),
            PlayerAction::YesNo(false),
            "a tax that only draws them a card is not worth mana this policy \
             cannot tell it has to spare"
        );
    }

    /// Urza's Saga at chapter I, one on each side of the table.
    ///
    /// A lore counter carries no sign of its own: it advances whichever Saga
    /// it lands on, so the seat that wants it is that Saga's controller and
    /// the opponent's copy is the one place it must not go. Before this,
    /// `Time`, `Lore` and `Custom` all scored zero, `targets` declined to
    /// express any preference, and the fallback — an opponent's permanents
    /// first, which is right for almost every other spell — handed the
    /// opponent their next chapter.
    #[test]
    fn a_lore_counter_goes_on_my_own_saga_and_never_the_opponents() {
        use baylee_cards_dsl::{Amount, CounterKind as Counter, Effect};
        use baylee_engine::engine::DecisionContext;
        let v = view(
            0,
            &[20, 20],
            vec![
                saga(obj(1), PlayerId::new(1), 1),
                saga(obj(2), PlayerId::new(0), 1),
            ],
        );
        let pending = Pending::ChooseTargets {
            player: v.seat,
            options: vec![obj(1), obj(2)],
            player_options: vec![],
            min: 1,
            max: 1,
            reason: baylee_engine::choice::TargetPrompt::Targets,
        };
        let effects = [Effect::AddCounter {
            kind: Counter::Lore,
            amount: Amount::Fixed(1),
        }];
        let context = DecisionContext {
            effects: &effects,
            ..Default::default()
        };

        assert_eq!(
            HeuristicAgent::new(AIProfile::EXPERT).act_with_context(&v, &pending, &context),
            PlayerAction::ChooseTargets {
                objects: vec![obj(2)],
                players: vec![]
            },
            "a lore counter advances the Saga it lands on, so it belongs on my own"
        );
    }

    /// Two of the opponent's suspended cards, one of them a single upkeep
    /// from casting itself for nothing.
    ///
    /// A time counter delays whatever it sits on, so every suspended card is
    /// a hostile target — but they are not equally hostile, and the fallback
    /// could not tell them apart because it never ranked them: it took the
    /// first enemy object it was offered. The clock about to run out is the
    /// one worth another turn.
    #[test]
    fn a_time_counter_delays_the_suspended_card_that_is_about_to_cast() {
        use baylee_cards_dsl::{Amount, CounterKind as Counter, Effect};
        use baylee_engine::engine::DecisionContext;
        let mut v = view(0, &[20, 20], vec![]);
        v.exile[1] = vec![
            suspended(obj(1), PlayerId::new(1), 4),
            suspended(obj(2), PlayerId::new(1), 1),
        ];
        let pending = Pending::ChooseTargets {
            player: v.seat,
            options: vec![obj(1), obj(2)],
            player_options: vec![],
            min: 1,
            max: 1,
            reason: baylee_engine::choice::TargetPrompt::Targets,
        };
        let effects = [Effect::AddCounter {
            kind: Counter::Time,
            amount: Amount::Fixed(1),
        }];
        let context = DecisionContext {
            effects: &effects,
            ..Default::default()
        };

        assert_eq!(
            HeuristicAgent::new(AIProfile::EXPERT).act_with_context(&v, &pending, &context),
            PlayerAction::ChooseTargets {
                objects: vec![obj(2)],
                players: vec![]
            },
            "one time counter left is one upkeep from a free cast; four is not"
        );
    }

    #[test]
    fn third_iteration_burn_finishes_the_player_before_killing_a_creature() {
        use baylee_cards_dsl::{Amount, Effect, TargetSpec};
        let v = view(0, &[20, 3], vec![permanent(obj(1), PlayerId::new(1), 1)]);
        let effects = [Effect::DealDamage {
            amount: Amount::Fixed(3),
            target: TargetSpec::AnyTarget,
        }];
        let context = baylee_engine::engine::DecisionContext {
            effects: &effects,
            ..Default::default()
        };
        let pending = Pending::ChooseTargets {
            player: v.seat,
            options: vec![obj(1)],
            player_options: vec![v.seat, PlayerId::new(1)],
            min: 1,
            max: 1,
            reason: baylee_engine::choice::TargetPrompt::Targets,
        };
        assert_eq!(
            HeuristicAgent::new(AIProfile::EXPERT).act_with_context(&v, &pending, &context),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![PlayerId::new(1)]
            }
        );
    }

    #[test]
    fn third_iteration_seeded_variation_is_repeatable_and_breaks_equal_spell_ties() {
        let mut v = view(0, &[20, 20], vec![]);
        v.hand = vec![hand_card(1, "Brainstorm"), hand_card(2, "Brainstorm")];
        let pending = Pending::Priority {
            player: v.seat,
            legal: Box::new(baylee_engine::choice::LegalActions {
                castable: vec![obj(1), obj(2)],
                ..Default::default()
            }),
        };
        for profile in [AIProfile::SHARP, AIProfile::EXPERT] {
            let choices: Vec<_> = (0..32)
                .map(|seed| {
                    let agent = HeuristicAgent::new(profile).with_seed(seed);
                    let action = agent.act(&v, &pending);
                    for _ in 0..5 {
                        assert_eq!(action, agent.act(&v, &pending));
                    }
                    action
                })
                .collect();
            assert!(choices.iter().any(|a| *a != choices[0]));
        }
    }

    #[test]
    fn third_iteration_scouted_sweeper_changes_deployment_but_not_the_base_agent() {
        use intelligence::{DeckIntel, ScoutedSeat, ScoutingReport};
        let mut v = view(
            0,
            &[20, 20],
            vec![
                permanent(obj(1), PlayerId::new(0), 2),
                permanent(obj(2), PlayerId::new(0), 2),
            ],
        );
        v.hand = vec![hand_card(3, "Baleful Strix"), hand_card(4, "Brainstorm")];
        let own = DeckIntel::new(vec![v.hand[0].card.index; 20], vec![]);
        let enemy = DeckIntel::new(
            vec![baylee_cards::decks::by_name("Toxic Deluge").unwrap(); 20],
            vec![],
        );
        let report = ScoutingReport {
            seats: vec![
                ScoutedSeat {
                    player: v.seat,
                    deck: &own,
                    hand: None,
                    library: None,
                    sideboard: None,
                },
                ScoutedSeat {
                    player: PlayerId::new(1),
                    deck: &enemy,
                    hand: Some(enemy.cards[..1].to_vec()),
                    library: None,
                    sideboard: None,
                },
            ],
        };
        let pending = Pending::Priority {
            player: v.seat,
            legal: Box::new(baylee_engine::choice::LegalActions {
                castable: vec![obj(3), obj(4)],
                ..Default::default()
            }),
        };
        let agent = HeuristicAgent::new(AIProfile::EXPERT);
        let ordinary = agent.act(&v, &pending);
        assert_eq!(ordinary, PlayerAction::CastSpell { card: obj(3) });
        assert_eq!(
            agent.act_with_scouting(
                &v,
                &pending,
                &baylee_engine::engine::DecisionContext::default(),
                &report
            ),
            PlayerAction::CastSpell { card: obj(4) }
        );
        assert_eq!(agent.act(&v, &pending), ordinary);
    }

    #[test]
    fn third_iteration_commander_damage_is_a_separate_loss_condition() {
        let mut commander = permanent(obj(1), PlayerId::new(1), 2);
        commander.commander = true;
        let mut v = view(
            0,
            &[40, 40],
            vec![commander, permanent(obj(2), PlayerId::new(0), 1)],
        );
        v.seats[0].commander_damage = vec![baylee_view::CommanderDamage {
            source: obj(1),
            amount: 19,
        }];
        v.combat.attackers = vec![baylee_view::AttackerView {
            creature: obj(1),
            defending: Defender::Player(v.seat),
            blocked: false,
        }];
        let pending = Pending::ChooseBlockers {
            player: v.seat,
            attacker: PlayerId::new(1),
            blockers: vec![baylee_engine::choice::BlockOption {
                blocker: obj(2),
                attackers: vec![obj(1)],
            }],
        };
        let agent = HeuristicAgent::new(AIProfile::EXPERT);
        assert_eq!(
            agent.act(&v, &pending),
            PlayerAction::DeclareBlockers {
                blockers: vec![(obj(2), obj(1))]
            }
        );
        v.seats[0].commander_damage[0].source = obj(99);
        assert_eq!(
            agent.act(&v, &pending),
            PlayerAction::DeclareBlockers { blockers: vec![] },
            "a different commander's damage must not combine with this one"
        );
    }

    #[test]
    fn third_iteration_x_uses_affordable_coloured_mana() {
        let mut v = view(0, &[20, 20], vec![]);
        v.seats[0].mana_pool.blue = 3;
        let context = baylee_engine::engine::DecisionContext {
            cost: Some("{X}{U}".parse().unwrap()),
            ..Default::default()
        };
        let pending = Pending::ChooseNumber {
            player: v.seat,
            min: 0,
            max: 50,
        };
        assert_eq!(
            HeuristicAgent::new(AIProfile::EXPERT).act_with_context(&v, &pending, &context),
            PlayerAction::ChooseNumber(2)
        );
    }

    #[test]
    fn third_iteration_miracle_needs_floating_mana_not_untapped_lands() {
        let mut v = view(
            0,
            &[20, 20],
            vec![
                carded(
                    permanent(obj(2), PlayerId::new(0), 0),
                    "Island",
                    TypeSet::LAND,
                ),
                carded(
                    permanent(obj(3), PlayerId::new(0), 0),
                    "Island",
                    TypeSet::LAND,
                ),
            ],
        );
        v.hand = vec![hand_card(1, "Temporal Mastery")];
        let pending = Pending::YesNo {
            player: v.seat,
            prompt: YesNoPrompt::Miracle { card: obj(1) },
            source: None,
        };
        let agent = HeuristicAgent::new(AIProfile::EXPERT);
        assert_eq!(agent.act(&v, &pending), PlayerAction::YesNo(false));
        v.seats[0].mana_pool.black = 2;
        assert_eq!(agent.act(&v, &pending), PlayerAction::YesNo(false));
        v.seats[0].mana_pool.black = 0;
        v.seats[0].mana_pool.blue = 2;
        assert_eq!(agent.act(&v, &pending), PlayerAction::YesNo(true));
    }

    #[test]
    fn a_counterspell_does_not_counter_its_own_spell_to_answer_an_enemy_ability() {
        let mut v = view(0, &[20, 20], vec![]);
        v.hand = vec![hand_card(3, "Counterspell")];
        let mut friendly = carded(permanent(obj(1), v.seat, 0), "Brainstorm", TypeSet::INSTANT);
        friendly.stack_item = Some(baylee_view::StackItem::Spell);
        let mut enemy = permanent(obj(2), PlayerId::new(1), 0);
        enemy.stack_item = Some(baylee_view::StackItem::Ability {
            source: obj(99),
            ability: None,
            text: None,
        });
        v.stack = vec![friendly, enemy];
        let pending = Pending::Priority {
            player: v.seat,
            legal: Box::new(baylee_engine::choice::LegalActions {
                castable: vec![obj(3)],
                ..Default::default()
            }),
        };
        assert_eq!(
            HeuristicAgent::new(AIProfile::EXPERT).act(&v, &pending),
            PlayerAction::PassPriority
        );
    }

    #[test]
    fn expert_keeps_a_blocker_against_lethal_commander_retaliation() {
        let mut enemy = permanent(obj(2), PlayerId::new(1), 2);
        enemy.commander = true;
        enemy.status = ObjectStatus::TAPPED;
        let mut v = view(
            0,
            &[40, 40],
            vec![permanent(obj(1), PlayerId::new(0), 6), enemy],
        );
        v.seats[0].commander_damage = vec![baylee_view::CommanderDamage {
            source: obj(2),
            amount: 19,
        }];
        let pending = Pending::ChooseAttackers {
            player: v.seat,
            attackers: vec![obj(1)],
            defenders: vec![Defender::Player(PlayerId::new(1))],
        };
        assert_eq!(
            HeuristicAgent::new(AIProfile::EXPERT).act(&v, &pending),
            PlayerAction::DeclareAttackers { attackers: vec![] }
        );
    }

    #[test]
    fn skilled_mulligans_refuse_a_landless_seven_but_stop_at_four() {
        let mut v = view(0, &[20, 20], vec![]);
        v.hand = (0..7).map(|i| hand_card(i, "Brainstorm")).collect();
        let decision = |taken| Pending::Mulligan {
            player: v.seat,
            taken,
            next_is_free: taken == 0,
        };
        assert_eq!(
            HeuristicAgent::new(AIProfile::SHARP).act(&v, &decision(0)),
            PlayerAction::MulliganTake
        );
        assert_eq!(
            HeuristicAgent::new(AIProfile::SHARP).act(&v, &decision(4)),
            PlayerAction::MulliganKeep
        );
    }

    #[test]
    fn a_mana_choice_completes_a_cast_instead_of_counting_unaffordable_pips() {
        let mut swamp = carded(
            permanent(obj(1), PlayerId::new(0), 0),
            "Swamp",
            TypeSet::LAND,
        );
        swamp
            .subtypes
            .insert(baylee_core::generated::subtypes::land::SWAMP);
        let mut v = view(0, &[20, 20], vec![swamp]);
        v.phase = baylee_view::Phase::FirstMain;
        v.hand = vec![hand_card(2, "Baleful Strix")];
        v.hand
            .extend((3..7).map(|id| hand_card(id, "Loran of the Third Path")));
        let pending = Pending::ChooseColor {
            player: v.seat,
            options: vec![
                baylee_core::mana::ManaColor::White,
                baylee_core::mana::ManaColor::Blue,
                baylee_core::mana::ManaColor::Black,
            ],
        };
        for profile in [AIProfile::STEADY, AIProfile::SHARP, AIProfile::EXPERT] {
            assert_eq!(
                HeuristicAgent::new(profile).act(&v, &pending),
                PlayerAction::ChooseColor(baylee_core::mana::ManaColor::Blue)
            );
        }
    }

    #[test]
    fn mana_color_follows_the_spell_in_hand() {
        let mut v = view(0, &[20, 20], vec![]);
        v.hand = vec![hand_card(1, "Brainstorm")];
        let pending = Pending::ChooseColor {
            player: v.seat,
            options: vec![
                baylee_core::mana::ManaColor::White,
                baylee_core::mana::ManaColor::Blue,
            ],
        };
        assert_eq!(
            HeuristicAgent::new(AIProfile::SHARP).act(&v, &pending),
            PlayerAction::ChooseColor(baylee_core::mana::ManaColor::Blue)
        );
    }

    #[test]
    fn lethal_power_is_not_lethal_through_a_larger_blocker() {
        let v = view(
            0,
            &[20, 5],
            vec![
                permanent(obj(1), PlayerId::new(0), 6),
                permanent(obj(2), PlayerId::new(1), 7),
            ],
        );
        let pending = Pending::ChooseAttackers {
            player: v.seat,
            attackers: vec![obj(1)],
            defenders: vec![Defender::Player(PlayerId::new(1))],
        };
        assert_eq!(
            HeuristicAgent::new(AIProfile::SHARP).act(&v, &pending),
            PlayerAction::DeclareAttackers { attackers: vec![] }
        );
    }

    #[test]
    fn combat_lifelink_can_save_a_seat_from_an_unblockable_attacker() {
        use baylee_cards_dsl::KeywordSet;
        let mut flyer = permanent(obj(2), PlayerId::new(1), 8);
        flyer.keywords = KeywordSet::FLYING.bits();
        let mut lifelinker = permanent(obj(3), PlayerId::new(0), 2);
        lifelinker.keywords = KeywordSet::LIFELINK.bits();
        let mut v = view(
            0,
            &[8, 20],
            vec![permanent(obj(1), PlayerId::new(1), 3), flyer, lifelinker],
        );
        v.combat.attackers = (1..=2)
            .map(|id| baylee_view::AttackerView {
                creature: obj(id),
                defending: Defender::Player(v.seat),
                blocked: false,
            })
            .collect();
        let pending = Pending::ChooseBlockers {
            player: v.seat,
            attacker: PlayerId::new(1),
            blockers: vec![baylee_engine::choice::BlockOption {
                blocker: obj(3),
                attackers: vec![obj(1)],
            }],
        };
        for profile in [AIProfile::SHARP, AIProfile::EXPERT] {
            assert_eq!(
                HeuristicAgent::new(profile).act(&v, &pending),
                PlayerAction::DeclareBlockers {
                    blockers: vec![(obj(3), obj(1))]
                }
            );
        }
    }

    #[test]
    fn combat_lifelink_after_lethal_first_strike_is_too_late() {
        use baylee_cards_dsl::KeywordSet;
        let mut first = permanent(obj(1), PlayerId::new(1), 3);
        first.keywords = KeywordSet::FIRST_STRIKE.bits();
        let mut other = permanent(obj(2), PlayerId::new(1), 1);
        other.toughness = Some(20);
        let mut lifelinker = permanent(obj(3), PlayerId::new(0), 3);
        lifelinker.toughness = Some(2);
        lifelinker.keywords = KeywordSet::LIFELINK.bits();
        let mut v = view(0, &[3, 20], vec![first, other, lifelinker]);
        v.combat.attackers = (1..=2)
            .map(|id| baylee_view::AttackerView {
                creature: obj(id),
                defending: Defender::Player(v.seat),
                blocked: false,
            })
            .collect();
        let pending = Pending::ChooseBlockers {
            player: v.seat,
            attacker: PlayerId::new(1),
            blockers: vec![baylee_engine::choice::BlockOption {
                blocker: obj(3),
                attackers: vec![obj(1), obj(2)],
            }],
        };
        for profile in [AIProfile::SHARP, AIProfile::EXPERT] {
            assert_eq!(
                HeuristicAgent::new(profile).act(&v, &pending),
                PlayerAction::DeclareBlockers {
                    blockers: vec![(obj(3), obj(1))]
                }
            );
        }
    }

    #[test]
    fn combat_retaliation_counts_a_defender_not_offered_as_an_attacker() {
        let mut wall = permanent(obj(3), PlayerId::new(0), 0);
        wall.toughness = Some(6);
        wall.keywords = baylee_cards_dsl::KeywordSet::DEFENDER.bits();
        let v = view(
            0,
            &[4, 20],
            vec![
                permanent(obj(1), PlayerId::new(0), 6),
                permanent(obj(2), PlayerId::new(1), 5),
                wall,
            ],
        );
        let pending = Pending::ChooseAttackers {
            player: v.seat,
            attackers: vec![obj(1)],
            defenders: vec![Defender::Player(PlayerId::new(1))],
        };
        let agent = HeuristicAgent::new(AIProfile::EXPERT);
        let attack = PlayerAction::DeclareAttackers {
            attackers: vec![(obj(1), Defender::Player(PlayerId::new(1)))],
        };
        assert_eq!(agent.act(&v, &pending), attack);
        let mut v = v;
        v.battlefield[2].summoning_sick = true;
        assert_eq!(
            agent.act(&v, &pending),
            attack,
            "summoning sickness does not stop a block"
        );
        for status in [ObjectStatus::TAPPED, ObjectStatus::PHASED_OUT] {
            v.battlefield[2].status = status;
            assert_eq!(
                agent.act(&v, &pending),
                PlayerAction::DeclareAttackers { attackers: vec![] }
            );
        }
        v.battlefield[2].status = ObjectStatus::NONE;
        v.battlefield[1].keywords = baylee_cards_dsl::KeywordSet::FLYING.bits();
        v.battlefield[0].keywords = baylee_cards_dsl::KeywordSet::REACH.bits();
        assert_eq!(
            agent.act(&v, &pending),
            PlayerAction::DeclareAttackers { attackers: vec![] }
        );
    }

    #[test]
    fn combat_a_winning_attack_does_not_get_redirected_to_a_planeswalker() {
        let v = view(
            0,
            &[20, 3],
            vec![
                permanent(obj(1), PlayerId::new(0), 5),
                walker(obj(2), PlayerId::new(1), 2),
            ],
        );
        let pending = Pending::ChooseAttackers {
            player: v.seat,
            attackers: vec![obj(1)],
            defenders: vec![
                Defender::Player(PlayerId::new(1)),
                Defender::Planeswalker(obj(2)),
            ],
        };
        for profile in [AIProfile::SHARP, AIProfile::EXPERT] {
            assert_eq!(
                HeuristicAgent::new(profile).act(&v, &pending),
                PlayerAction::DeclareAttackers {
                    attackers: vec![(obj(1), Defender::Player(PlayerId::new(1)))]
                }
            );
        }
    }

    #[test]
    fn combat_blocks_lethal_player_damage_before_protecting_a_planeswalker() {
        let mut v = view(
            0,
            &[3, 20],
            vec![
                permanent(obj(1), PlayerId::new(1), 3),
                permanent(obj(2), PlayerId::new(1), 10),
                permanent(obj(3), PlayerId::new(0), 1),
                walker(obj(4), PlayerId::new(0), 5),
            ],
        );
        v.combat.attackers = vec![
            baylee_view::AttackerView {
                creature: obj(1),
                defending: Defender::Player(v.seat),
                blocked: false,
            },
            baylee_view::AttackerView {
                creature: obj(2),
                defending: Defender::Planeswalker(obj(4)),
                blocked: false,
            },
        ];
        let pending = Pending::ChooseBlockers {
            player: v.seat,
            attacker: PlayerId::new(1),
            blockers: vec![baylee_engine::choice::BlockOption {
                blocker: obj(3),
                attackers: vec![obj(1), obj(2)],
            }],
        };
        for profile in [AIProfile::SHARP, AIProfile::EXPERT] {
            assert_eq!(
                HeuristicAgent::new(profile).act(&v, &pending),
                PlayerAction::DeclareBlockers {
                    blockers: vec![(obj(3), obj(1))]
                }
            );
        }
    }

    #[test]
    fn every_adjacent_difficulty_changes_a_real_decision() {
        assert_eq!(AIProfile::NAMED.len(), 5);
        let mut v = view(0, &[20, 20], vec![]);
        v.hand = (0..7).map(|i| hand_card(i, "Brainstorm")).collect();
        let pending = Pending::Mulligan {
            player: v.seat,
            taken: 0,
            next_is_free: true,
        };
        let answer = |profile, view: &PlayerView, pending: &Pending| {
            HeuristicAgent::new(profile).act(view, pending)
        };
        assert_ne!(
            answer(AIProfile::NOVICE, &v, &pending),
            answer(AIProfile::CASUAL, &v, &pending)
        );
        v.hand = vec![hand_card(0, "Island"), hand_card(1, "Island")];
        v.hand
            .extend((2..7).map(|i| hand_card(i, "Darksteel Forge")));
        assert_ne!(
            answer(AIProfile::CASUAL, &v, &pending),
            answer(AIProfile::STEADY, &v, &pending)
        );
        let v = view(
            0,
            &[20, 5],
            vec![
                permanent(obj(1), PlayerId::new(0), 6),
                permanent(obj(2), PlayerId::new(1), 7),
            ],
        );
        let pending = Pending::ChooseAttackers {
            player: v.seat,
            attackers: vec![obj(1)],
            defenders: vec![Defender::Player(PlayerId::new(1))],
        };
        assert_ne!(
            answer(AIProfile::STEADY, &v, &pending),
            answer(AIProfile::SHARP, &v, &pending)
        );
        let v = view(
            0,
            &[4, 20],
            vec![
                permanent(obj(1), PlayerId::new(0), 6),
                permanent(obj(2), PlayerId::new(1), 5),
            ],
        );
        assert_ne!(
            answer(AIProfile::SHARP, &v, &pending),
            answer(AIProfile::EXPERT, &v, &pending)
        );
    }

    #[test]
    fn mana_planning_taps_the_right_color_and_does_not_tap_for_an_unaffordable_spell() {
        let mut forest = carded(
            permanent(obj(1), PlayerId::new(0), 0),
            "Forest",
            TypeSet::LAND,
        );
        forest
            .subtypes
            .insert(baylee_core::generated::subtypes::land::FOREST);
        let mut island = carded(
            permanent(obj(2), PlayerId::new(0), 0),
            "Island",
            TypeSet::LAND,
        );
        island
            .subtypes
            .insert(baylee_core::generated::subtypes::land::ISLAND);
        let mut v = view(0, &[20, 20], vec![forest, island]);
        v.phase = baylee_view::Phase::FirstMain;
        v.hand = vec![hand_card(3, "Brainstorm")];
        let pending = Pending::Priority {
            player: v.seat,
            legal: Box::new(baylee_engine::choice::LegalActions {
                can_pass: true,
                mana_abilities: vec![obj(1), obj(2)],
                ..Default::default()
            }),
        };
        assert_eq!(
            agent().act(&v, &pending),
            PlayerAction::ActivateManaAbility { source: obj(2) }
        );
        v.hand = vec![hand_card(3, "Darksteel Forge")];
        assert_eq!(agent().act(&v, &pending), PlayerAction::PassPriority);
    }

    #[test]
    fn a_free_counter_with_an_unpayable_future_cost_is_declined() {
        let spell = carded(
            permanent(obj(1), PlayerId::new(1), 0),
            "Brainstorm",
            TypeSet::INSTANT,
        );
        let mut v = view(0, &[20, 20], vec![]);
        v.stack.push(spell);
        v.hand = vec![hand_card(2, "Pact of Negation")];
        let pending = Pending::Priority {
            player: v.seat,
            legal: Box::new(baylee_engine::choice::LegalActions {
                can_pass: true,
                castable: vec![obj(2)],
                ..Default::default()
            }),
        };
        assert_eq!(agent().act(&v, &pending), PlayerAction::PassPriority);
    }

    #[test]
    fn commander_mana_planning_reads_the_command_zone_and_its_tax() {
        use baylee_core::generated::subtypes::land;
        let mut lands = Vec::new();
        for (i, subtype) in [land::PLAINS, land::ISLAND, land::SWAMP]
            .into_iter()
            .enumerate()
        {
            let mut source = permanent(obj(u32::try_from(i).unwrap() + 1), PlayerId::new(0), 0);
            source.types = TypeSet::LAND;
            source.subtypes.insert(subtype);
            lands.push(source);
        }
        let mut v = view(0, &[20, 20], lands);
        v.phase = baylee_view::Phase::FirstMain;
        let mut commander = carded(
            walker(obj(10), v.seat, 3),
            "Aminatou, the Fateshifter",
            TypeSet::PLANESWALKER,
        );
        commander.commander = true;
        v.seats[0].commanders.push(baylee_view::CommanderView {
            object: commander.id,
            card: commander.card,
            name: commander.name.clone(),
            casts: 0,
        });
        v.command[0].push(commander);
        let pending = Pending::Priority {
            player: v.seat,
            legal: Box::new(baylee_engine::choice::LegalActions {
                can_pass: true,
                mana_abilities: (1..=3).map(obj).collect(),
                ..Default::default()
            }),
        };
        assert!(
            matches!(
                agent().act(&v, &pending),
                PlayerAction::ActivateManaAbility { .. }
            ),
            "the command zone must participate in affordable spell plans"
        );
        v.seats[0].commanders[0].casts = 1;
        assert_eq!(
            agent().act(&v, &pending),
            PlayerAction::PassPriority,
            "three sources cannot pay the recast tax"
        );
        v.seats[0].commanders[0].casts = 0;
        v.command[0].clear();
        assert_eq!(
            agent().act(&v, &pending),
            PlayerAction::PassPriority,
            "a listed commander in another zone is not a cast candidate"
        );
    }

    #[test]
    fn convoke_pays_with_the_offered_permanents_instead_of_targeting_one() {
        let v = view(
            0,
            &[20, 20],
            (1..=6)
                .map(|i| permanent(obj(i), PlayerId::new(0), 2))
                .collect(),
        );
        let offered: Vec<_> = (1..=6).map(obj).collect();
        let pending = Pending::ChooseTargets {
            player: v.seat,
            options: offered.clone(),
            player_options: vec![],
            min: 0,
            max: 6,
            reason: baylee_engine::choice::TargetPrompt::Convoke,
        };
        for (_, profile) in AIProfile::NAMED {
            assert_eq!(
                HeuristicAgent::new(profile).act(&v, &pending),
                PlayerAction::ChooseTargets {
                    objects: offered.clone(),
                    players: vec![]
                },
            );
        }
    }

    #[test]
    fn score_noise_changes_choices_without_changing_replays() {
        let mut v = view(0, &[20, 20], vec![]);
        v.phase = baylee_view::Phase::FirstMain;
        v.seats[0].mana_pool.blue = 2;
        v.hand = vec![hand_card(1, "Brainstorm"), hand_card(2, "Brainstorm")];
        let pending = Pending::Priority {
            player: v.seat,
            legal: Box::new(baylee_engine::choice::LegalActions {
                can_pass: true,
                castable: vec![obj(1), obj(2)],
                ..Default::default()
            }),
        };
        let agent = HeuristicAgent::new(AIProfile::NOVICE);
        let first = agent.act(&v, &pending);
        let mut different = false;
        for seq in 0..64 {
            v.seq = seq;
            let action = agent.act(&v, &pending);
            assert_eq!(action, agent.act(&v, &pending));
            different |= action != first;
        }
        assert!(different);
    }

    #[test]
    fn holding_up_interaction_changes_a_tap_out() {
        let mut v = view(0, &[20, 20], vec![permanent(obj(1), PlayerId::new(0), 2)]);
        v.phase = baylee_view::Phase::FirstMain;
        v.seats[0].mana_pool.blue = 2;
        v.hand = vec![hand_card(2, "Sol Ring"), hand_card(3, "Mana Drain")];
        let pending = Pending::Priority {
            player: v.seat,
            legal: Box::new(baylee_engine::choice::LegalActions {
                can_pass: true,
                castable: vec![obj(2)],
                ..Default::default()
            }),
        };
        assert_eq!(
            HeuristicAgent::new(AIProfile::STEADY).act(&v, &pending),
            PlayerAction::PassPriority
        );
        assert_eq!(
            HeuristicAgent::new(AIProfile {
                hold_up: baylee_core::preset::HoldUp::None,
                ..AIProfile::STEADY
            })
            .act(&v, &pending),
            PlayerAction::CastSpell { card: obj(2) }
        );
        // Threat-aware releases the reserve at an empty opposing table.
        assert_eq!(
            HeuristicAgent::new(AIProfile::SHARP).act(&v, &pending),
            PlayerAction::CastSpell { card: obj(2) }
        );
    }

    #[test]
    fn search_is_repeatable_bounded_and_preserves_unseen_identities() {
        let mut v = view(
            0,
            &[20, 20],
            (1..=6)
                .map(|i| permanent(obj(i), PlayerId::new(0), 2))
                .chain((10..=15).map(|i| permanent(obj(i), PlayerId::new(1), 3)))
                .collect(),
        );
        let squad: Vec<_> = (1..=6).map(obj).collect();
        let victim = PlayerId::new(1);
        for (_, profile) in AIProfile::NAMED {
            let first = search::attackers(&v, &squad, victim, profile);
            assert!(first.nodes <= profile.node_budget());
            for _ in 0..3 {
                let again = search::attackers(&v, &squad, victim, profile);
                assert_eq!(first.attackers, again.attackers);
                assert_eq!(first.nodes, again.nodes);
            }
        }
        // All identities here are None, including the opposing creatures.
        // Presentation changes cannot supply a hidden rules identity.
        let before = search::attackers(&v, &squad, victim, AIProfile::EXPERT).attackers;
        for o in &mut v.battlefield {
            o.name = "unseen".into();
        }
        assert_eq!(
            before,
            search::attackers(&v, &squad, victim, AIProfile::EXPERT).attackers
        );
    }

    /// The five shipped profiles, so a rule is not proved on one of them.
    const PROFILES: [(&str, AIProfile); 5] = [
        ("NOVICE", AIProfile::NOVICE),
        ("CASUAL", AIProfile::CASUAL),
        ("STEADY", AIProfile::STEADY),
        ("SHARP", AIProfile::SHARP),
        ("EXPERT", AIProfile::EXPERT),
    ];

    /// The position #123 was reported from: one attacker with first strike
    /// and no evasion, `blockers` untapped creatures that may all legally
    /// block it, and a life total it beats on its own.
    ///
    /// `power` is what the view says about the attacker, so `None` is the
    /// case where the view carries the attack but cannot describe what is
    /// in it.
    fn lethal_attack(
        power: Option<i16>,
        in_view: bool,
        life: i32,
        blockers: u32,
    ) -> (PlayerView, Pending) {
        let defender = PlayerId::new(0);
        let attacker = PlayerId::new(1);
        let mut battlefield: Vec<PublicObject> = (0..blockers)
            .map(|i| permanent(obj(10 + i), defender, 2))
            .collect();
        if in_view {
            let mut a = permanent(obj(1), attacker, power.unwrap_or(75));
            a.power = power;
            a.keywords = baylee_cards_dsl::KeywordSet::FIRST_STRIKE.bits();
            battlefield.push(a);
        }
        let mut v = view(0, &[life, 20], battlefield);
        v.active = attacker;
        v.step = baylee_view::Step::DeclareBlockers;
        v.combat.attackers = vec![baylee_view::AttackerView {
            creature: obj(1),
            defending: Defender::Player(defender),
            blocked: false,
        }];
        let pending = Pending::ChooseBlockers {
            player: defender,
            attacker,
            blockers: (0..blockers)
                .map(|i| baylee_engine::choice::BlockOption {
                    blocker: obj(10 + i),
                    attackers: vec![obj(1)],
                })
                .collect(),
        };
        (v, pending)
    }

    fn blocks(profile: AIProfile, v: &PlayerView, pending: &Pending) -> usize {
        match HeuristicAgent::new(profile).act(v, pending) {
            PlayerAction::DeclareBlockers { blockers } => blockers.len(),
            other => panic!("not a block answer: {other:?}"),
        }
    }

    /// #123, the scenario: a lethal attacker is chumped, first strike and
    /// all. Eight blockers, one attacker — one of them is enough, and
    /// spending a second on it would be the opposite error.
    #[test]
    fn a_lethal_attacker_is_chump_blocked_by_every_profile() {
        let (v, pending) = lethal_attack(Some(75), true, 20, 8);
        for (name, profile) in PROFILES {
            assert_eq!(blocks(profile, &v, &pending), 1, "{name} took the damage");
        }
    }

    /// #123, the negative that keeps the rule honest. The same board with
    /// the attacker below lethal: blocking loses a 2/2 to kill nothing, so
    /// every profile stays home. Without this, "always block" passes the
    /// test above wearing rule 1's clothes.
    #[test]
    fn a_bad_trade_is_declined_while_the_seat_is_not_dying() {
        let (mut v, pending) = lethal_attack(Some(4), true, 20, 8);
        v.battlefield.last_mut().unwrap().toughness = Some(4);
        for (name, profile) in PROFILES {
            assert_eq!(
                blocks(profile, &v, &pending),
                0,
                "{name} chumped for nothing"
            );
        }
    }

    /// #123, first strike specifically: it is the one keyword the reported
    /// attacker carried, and it changes [`combat::exchange`] without
    /// changing legality (CR 702.7). A 4/4 first striker into a 4/4 is a
    /// block that kills our creature and nothing of theirs — declined while
    /// the seat can afford it, and made once the seat cannot.
    #[test]
    fn first_strike_changes_the_exchange_and_not_the_legality() {
        let (mut v, pending) = lethal_attack(Some(4), true, 20, 1);
        v.battlefield.last_mut().unwrap().toughness = Some(4);
        for (name, profile) in PROFILES {
            assert_eq!(
                blocks(profile, &v, &pending),
                0,
                "{name} fed a first striker"
            );
        }
        let (mut v, pending) = lethal_attack(Some(4), true, 4, 1);
        v.battlefield.last_mut().unwrap().toughness = Some(4);
        for (name, profile) in PROFILES {
            assert_eq!(blocks(profile, &v, &pending), 1, "{name} died to a 4/4");
        }
    }

    /// #123, the rule this ticket turned out to be about: an attacker the
    /// view cannot describe is **unknown**, not absent.
    ///
    /// `Fighter::of` is three `?` in a row — the object, its power, its
    /// toughness — and each `None` used to leave the attacker out of the
    /// damage sum *and* out of every blocker's candidate list. So the seat
    /// read a lethal attack as no attack at all and declined every block:
    /// the engine's own pairings said a creature was there, and the agent
    /// answered as though the board were empty.
    ///
    /// Both halves of the decision are pinned, because they fail
    /// separately: `choose_blocks` reads the attack out of the view, and
    /// `search::blockers` used to fall back to it on exactly this condition
    /// — a fallback onto the same blind spot, which is not a fallback.
    /// `NOVICE`/`CASUAL` take the first, the rest the second.
    #[test]
    fn an_attacker_the_view_cannot_describe_is_still_blocked() {
        // The control: the same position, readable. Without it the two
        // below would also pass against an agent that blocks with anything.
        let (v, pending) = lethal_attack(Some(75), true, 20, 8);
        for (name, profile) in PROFILES {
            assert_eq!(blocks(profile, &v, &pending), 1, "{name}, readable");
        }
        // The view carries the attack and the engine offers the pairings,
        // but the attacker has no body on it.
        let (v, pending) = lethal_attack(None, true, 20, 8);
        for (name, profile) in PROFILES {
            assert_eq!(blocks(profile, &v, &pending), 1, "{name}, power unread");
        }
        // And the attacker is in no zone this seat can see at all.
        let (v, pending) = lethal_attack(Some(75), false, 20, 8);
        for (name, profile) in PROFILES {
            assert_eq!(blocks(profile, &v, &pending), 1, "{name}, attacker unseen");
        }
    }

    #[test]
    fn the_search_finds_a_menace_gang_block() {
        let mut attacker = permanent(obj(1), PlayerId::new(1), 4);
        attacker.keywords = baylee_cards_dsl::KeywordSet::MENACE.bits();
        let mut v = view(
            0,
            &[3, 20],
            vec![
                attacker,
                permanent(obj(2), PlayerId::new(0), 2),
                permanent(obj(3), PlayerId::new(0), 2),
            ],
        );
        v.combat.attackers.push(baylee_view::AttackerView {
            creature: obj(1),
            defending: Defender::Player(v.seat),
            blocked: false,
        });
        let pending = Pending::ChooseBlockers {
            player: v.seat,
            attacker: PlayerId::new(1),
            blockers: vec![
                baylee_engine::choice::BlockOption {
                    blocker: obj(2),
                    attackers: vec![obj(1)],
                },
                baylee_engine::choice::BlockOption {
                    blocker: obj(3),
                    attackers: vec![obj(1)],
                },
            ],
        };
        assert_eq!(
            HeuristicAgent::new(AIProfile::SHARP).act(&v, &pending),
            PlayerAction::DeclareBlockers {
                blockers: vec![(obj(2), obj(1)), (obj(3), obj(1))]
            }
        );
    }

    /// A default-profile agent at a table with no teams.
    fn agent() -> HeuristicAgent {
        HeuristicAgent::new(AIProfile::default())
    }

    /// One permanent on the battlefield, as the seat sees it.
    fn permanent(id: ObjectId, controller: PlayerId, power: i16) -> PublicObject {
        PublicObject {
            id,
            card: None,
            name: "Creature".into(),
            controller,
            owner: controller,
            commander: false,
            status: ObjectStatus::default(),
            types: TypeSet::CREATURE,
            supertypes: SupertypeSet::EMPTY,
            subtypes: SubtypeSet::EMPTY,
            token: None,
            colors: ColorSet::EMPTY,
            mana_value: 1,
            keywords: 0,
            power: Some(power),
            toughness: Some(power),
            base_power: None,
            base_toughness: None,
            loyalty: None,
            damage: 0,
            counters: vec![],
            attached_to: None,
            targets: vec![],
            stack_item: None,
            summoning_sick: false,
            granted_mana: None,
            board_mana: None,
        }
    }

    /// A planeswalker with `loyalty` counters on it.
    fn walker(id: ObjectId, controller: PlayerId, loyalty: u16) -> PublicObject {
        PublicObject {
            name: "Walker".into(),
            types: TypeSet::PLANESWALKER,
            power: None,
            toughness: None,
            base_power: None,
            base_toughness: None,
            loyalty: Some(loyalty),
            counters: vec![CounterEntry {
                kind: CounterKind::Loyalty,
                count: loyalty,
            }],
            ..permanent(id, controller, 0)
        }
    }

    /// Urza's Saga with `lore` lore counters on it, which is chapter `lore`.
    ///
    /// A real registry card, for the reason [`carded`] gives and one more:
    /// how many chapters a Saga has is printed on the card and nowhere in
    /// the view, so the agent reads it back out of the pool.
    fn saga(id: ObjectId, controller: PlayerId, lore: u16) -> PublicObject {
        let mut object = carded(
            permanent(id, controller, 0),
            "Urza's Saga",
            TypeSet::LAND.union(TypeSet::ENCHANTMENT),
        );
        object.name = "Urza's Saga".into();
        object.power = None;
        object.toughness = None;
        object.counters = vec![CounterEntry {
            kind: CounterKind::Lore,
            count: lore,
        }];
        object
    }

    /// Ancestral Vision in exile with `time` time counters left on it —
    /// suspend 4, and a free three-card draw when the last one comes off.
    fn suspended(id: ObjectId, owner: PlayerId, time: u16) -> PublicObject {
        let mut object = carded(
            permanent(id, owner, 0),
            "Ancestral Vision",
            TypeSet::SORCERY,
        );
        object.name = "Ancestral Vision".into();
        object.power = None;
        object.toughness = None;
        object.counters = vec![CounterEntry {
            kind: CounterKind::Time,
            count: time,
        }];
        object
    }

    /// A view of `seats` (life totals) with `battlefield` on the table.
    fn view(seat: u8, lives: &[i32], battlefield: Vec<PublicObject>) -> PlayerView {
        let seats: Vec<SeatView> = lives
            .iter()
            .enumerate()
            .map(|(i, life)| SeatView {
                player: PlayerId::new(i as u8),
                life: *life,
                poison: 0,
                energy: 0,
                hand_count: 0,
                library_count: 40,
                graveyard_count: 0,
                has_lost: false,
                mana_pool: baylee_view::ManaPoolView::default(),
                commanders: vec![],
                commander_damage: vec![],
            })
            .collect();
        PlayerView {
            seq: 7,
            seat: PlayerId::new(seat),
            turn: 3,
            phase: baylee_view::Phase::Combat,
            step: baylee_view::Step::DeclareAttackers,
            active: PlayerId::new(seat),
            awaiting: None,
            decision_remaining_ms: None,
            priority_held: false,
            monarch: None,
            day_night: None,
            seats,
            hand: vec![],
            battlefield,
            stack: vec![],
            graveyards: vec![vec![]; lives.len()],
            exile: vec![vec![]; lives.len()],
            command: vec![vec![]; lives.len()],
            combat: CombatView::default(),
            looking_at: Vec::new(),
            owed: None,
            sorcery_lock: None,
        }
    }

    /// The two threat policies read the same table differently: one goes for
    /// the player who is winning the race, the other for the biggest board.
    #[test]
    fn politics_decides_who_gets_attacked() {
        // Seat 1 is ahead on life with nothing out; seat 2 is on 5 life with
        // three creatures.
        let board = vec![
            permanent(obj(10), PlayerId::new(2), 2),
            permanent(obj(11), PlayerId::new(2), 2),
            permanent(obj(12), PlayerId::new(2), 2),
        ];
        let v = view(0, &[40, 40, 5], board);
        let defenders = [PlayerId::new(1), PlayerId::new(2)];

        let leader = HeuristicAgent::new(AIProfile {
            politics: Politics::AttackLeader,
            ..AIProfile::default()
        });
        assert_eq!(
            leader.pick_defender(&v, &defenders),
            PlayerId::new(1),
            "attack-leader goes for the player on 40 life"
        );

        let archenemy = HeuristicAgent::new(AIProfile {
            politics: Politics::Archenemy,
            ..AIProfile::default()
        });
        assert_eq!(
            archenemy.pick_defender(&v, &defenders),
            PlayerId::new(2),
            "archenemy goes for the board, not the life total"
        );
    }

    /// "Random" must still be a function of the game state — a real RNG here
    /// would make replays and the soak diverge.
    #[test]
    fn random_politics_stays_deterministic() {
        let v = view(0, &[40, 40, 40], vec![]);
        let defenders = [PlayerId::new(1), PlayerId::new(2)];
        let agent = HeuristicAgent::new(AIProfile {
            politics: Politics::Random,
            ..AIProfile::default()
        });
        let first = agent.pick_defender(&v, &defenders);
        for _ in 0..10 {
            assert_eq!(agent.pick_defender(&v, &defenders), first);
        }
        assert!(defenders.contains(&first));
    }

    /// A planeswalker is worth attacking only when the attack kills it:
    /// three 1/1s finish a 3-loyalty walker, so they go for the walker.
    #[test]
    fn a_squad_that_can_finish_a_planeswalker_goes_for_it() {
        let victim = PlayerId::new(1);
        let squad = vec![obj(1), obj(2), obj(3)];
        let mut board: Vec<PublicObject> = squad
            .iter()
            .map(|id| permanent(*id, PlayerId::new(0), 1))
            .collect();
        board.push(walker(obj(20), victim, 3));
        let v = view(0, &[20, 20], board);
        let defenders = [Defender::Player(victim), Defender::Planeswalker(obj(20))];

        assert_eq!(
            aim_at(&v, victim, &squad, &defenders),
            Defender::Planeswalker(obj(20)),
            "three power went to the player instead of killing the walker"
        );
    }

    /// Two 1/1s only chip it, which is the worst of both — so they hit the
    /// player instead.
    #[test]
    fn a_squad_that_would_only_chip_a_planeswalker_hits_the_player() {
        let victim = PlayerId::new(1);
        let squad = vec![obj(1), obj(2)];
        let mut board: Vec<PublicObject> = squad
            .iter()
            .map(|id| permanent(*id, PlayerId::new(0), 1))
            .collect();
        board.push(walker(obj(20), victim, 3));
        let v = view(0, &[20, 20], board);
        let defenders = [Defender::Player(victim), Defender::Planeswalker(obj(20))];

        assert_eq!(
            aim_at(&v, victim, &squad, &defenders),
            Defender::Player(victim),
            "the squad chipped a walker it could not kill"
        );
    }

    /// A teammate's creature is a legal target and the wrong one. The engine
    /// offers both (CR 115.4); picking is the agent's job.
    #[test]
    fn removal_goes_past_a_teammate_to_an_opponent() {
        let mine = permanent(obj(1), PlayerId::new(0), 2);
        let partner = permanent(obj(2), PlayerId::new(1), 2);
        let enemy = permanent(obj(3), PlayerId::new(2), 2);
        let v = view(0, &[20, 20, 20], vec![mine, partner, enemy]);
        let agent =
            HeuristicAgent::new(AIProfile::default()).with_teams(vec![Some(1), Some(1), Some(2)]);
        let pending = Pending::ChooseTargets {
            player: PlayerId::new(0),
            options: vec![obj(1), obj(2), obj(3)],
            player_options: vec![PlayerId::new(0), PlayerId::new(1), PlayerId::new(2)],
            min: 1,
            max: 1,
            reason: baylee_engine::choice::TargetPrompt::Targets,
        };

        let PlayerAction::ChooseTargets { objects, .. } = agent.act(&v, &pending) else {
            panic!("the agent answered a target choice with something else");
        };
        assert_eq!(objects, vec![obj(3)], "the agent shot its own side");
    }

    /// The same rule for the face: a burn spell goes at an opponent, never at
    /// the partner whose life total is half the team's problem.
    #[test]
    fn burn_goes_at_an_opponent_and_not_at_a_teammate() {
        let v = view(0, &[20, 20, 20], vec![]);
        let agent =
            HeuristicAgent::new(AIProfile::default()).with_teams(vec![Some(1), Some(1), Some(2)]);
        let pending = Pending::ChooseTargets {
            player: PlayerId::new(0),
            options: vec![],
            player_options: vec![PlayerId::new(0), PlayerId::new(1), PlayerId::new(2)],
            min: 1,
            max: 1,
            reason: baylee_engine::choice::TargetPrompt::Targets,
        };

        let PlayerAction::ChooseTargets { players, .. } = agent.act(&v, &pending) else {
            panic!("the agent answered a target choice with something else");
        };
        assert_eq!(
            players,
            vec![PlayerId::new(2)],
            "the agent burned its partner"
        );
    }

    /// And "choose a player" is the same question asked without a target.
    #[test]
    fn choosing_a_player_skips_the_teammate() {
        let v = view(0, &[20, 20, 20], vec![]);
        let agent =
            HeuristicAgent::new(AIProfile::default()).with_teams(vec![Some(1), Some(1), Some(2)]);
        let pending = Pending::ChoosePlayer {
            player: PlayerId::new(0),
            options: vec![PlayerId::new(0), PlayerId::new(1), PlayerId::new(2)],
        };

        assert_eq!(
            agent.act(&v, &pending),
            PlayerAction::ChoosePlayer(PlayerId::new(2))
        );
    }

    /// Facing lethal, the seat blocks with whatever it has — even a 1/1
    /// under a 5/5, which dies and stops the game being over.
    ///
    /// Asked through `act` and not through `choose_blocks`, because the
    /// bug this replaces was not in the maths: `Pending::ChooseBlockers`
    /// was answered `vec![]` and no combat function was ever called.
    #[test]
    fn a_seat_facing_lethal_chump_blocks() {
        let attacker = permanent(obj(1), PlayerId::new(1), 5);
        let chump = permanent(obj(2), PlayerId::new(0), 1);
        let mut v = view(0, &[4, 20], vec![attacker, chump]);
        v.combat = CombatView {
            attackers: vec![baylee_view::AttackerView {
                creature: obj(1),
                defending: Defender::Player(PlayerId::new(0)),
                blocked: false,
            }],
            blockers: vec![],
        };
        let pending = Pending::ChooseBlockers {
            player: PlayerId::new(0),
            attacker: PlayerId::new(1),
            blockers: vec![baylee_engine::choice::BlockOption {
                blocker: obj(2),
                attackers: vec![obj(1)],
            }],
        };

        assert_eq!(
            agent().act(&v, &pending),
            PlayerAction::DeclareBlockers {
                blockers: vec![(obj(2), obj(1))],
            },
            "the seat took five to the face on four life"
        );
    }

    /// The same 1/1 does not block the same 5/5 at a comfortable life
    /// total: the creature is worth more than three points of life.
    #[test]
    fn the_same_block_is_declined_when_it_is_not_lethal() {
        let attacker = permanent(obj(1), PlayerId::new(1), 5);
        let chump = permanent(obj(2), PlayerId::new(0), 1);
        let mut v = view(0, &[20, 20], vec![attacker, chump]);
        v.combat = CombatView {
            attackers: vec![baylee_view::AttackerView {
                creature: obj(1),
                defending: Defender::Player(PlayerId::new(0)),
                blocked: false,
            }],
            blockers: vec![],
        };
        let pending = Pending::ChooseBlockers {
            player: PlayerId::new(0),
            attacker: PlayerId::new(1),
            blockers: vec![baylee_engine::choice::BlockOption {
                blocker: obj(2),
                attackers: vec![obj(1)],
            }],
        };

        assert_eq!(
            agent().act(&v, &pending),
            PlayerAction::DeclareBlockers { blockers: vec![] },
            "a 1/1 was thrown under a 5/5 for nothing"
        );
    }

    /// A 1/1 does not run into an untapped 4/4; the 4/4 on the same board
    /// does attack, because nothing over there kills it.
    #[test]
    fn only_the_creature_that_survives_the_block_attacks() {
        let small = permanent(obj(1), PlayerId::new(0), 1);
        let big = permanent(obj(2), PlayerId::new(0), 4);
        let wall = permanent(obj(3), PlayerId::new(1), 3);
        let v = view(0, &[20, 20], vec![small, big, wall]);
        let pending = Pending::ChooseAttackers {
            player: PlayerId::new(0),
            attackers: vec![obj(1), obj(2)],
            defenders: vec![Defender::Player(PlayerId::new(1))],
        };

        let PlayerAction::DeclareAttackers { attackers } = agent().act(&v, &pending) else {
            panic!("the agent answered an attack declaration with something else");
        };
        assert_eq!(
            attackers,
            vec![(obj(2), Defender::Player(PlayerId::new(1)))],
            "the 1/1 charged a 3/3, or the 4/4 stayed home"
        );
    }

    /// With no teams at the table nothing changes: every other seat is an
    /// opponent and the first one still gets it.
    #[test]
    fn a_table_with_no_teams_chooses_as_it_did_before() {
        let v = view(0, &[20, 20, 20], vec![]);
        let agent = HeuristicAgent::new(AIProfile::default());
        let pending = Pending::ChoosePlayer {
            player: PlayerId::new(0),
            options: vec![PlayerId::new(0), PlayerId::new(1), PlayerId::new(2)],
        };

        assert_eq!(
            agent.act(&v, &pending),
            PlayerAction::ChoosePlayer(PlayerId::new(1))
        );
    }

    /// The same permanent, but backed by a real registry card so the
    /// activation policy can read what its abilities cost and do.
    ///
    /// Named, not numbered. An index is assigned over the whole card corpus
    /// rather than over this pool, so the literal that was Arid Mesa is now
    /// some other card — and a test that reads the abilities off whatever
    /// landed there fails for a reason that has nothing to do with the agent.
    fn carded(mut object: PublicObject, name: &str, types: TypeSet) -> PublicObject {
        object.card = Some(baylee_view::CardIdentity {
            index: baylee_cards::decks::by_name(name).expect("a card of that name in the pool"),
            print: baylee_core::ids::PrintRef::new(0),
            face: 0,
        });
        object.types = types;
        object
    }

    /// Priority with exactly these abilities on offer and nothing else.
    fn offering(abilities: Vec<(ObjectId, u32)>) -> Pending {
        Pending::Priority {
            player: PlayerId::new(0),
            legal: Box::new(baylee_engine::choice::LegalActions {
                can_pass: true,
                abilities,
                ..Default::default()
            }),
        }
    }

    /// Arid Mesa is `{T}, Pay 1 life, Sacrifice this: search`. An agent that
    /// leaves it alone has played a land that makes no mana whatsoever, which
    /// is what every fetchland in the acceptance decks did until now.
    #[test]
    fn a_fetchland_is_cracked() {
        let mesa = carded(
            permanent(obj(1), PlayerId::new(0), 0),
            "Arid Mesa",
            TypeSet::LAND,
        );
        let v = view(0, &[20, 20], vec![mesa]);

        assert_eq!(
            agent().act(&v, &offering(vec![(obj(1), 0)])),
            PlayerAction::ActivateAbility {
                source: obj(1),
                ability_index: 0,
            },
            "the fetchland was left on the battlefield doing nothing"
        );
    }

    /// The same land at six life stays put. One life is not what stops it —
    /// the margin after it is, and it is the same margin the pay-life-or-
    /// enter-tapped answer keeps.
    #[test]
    fn a_fetchland_is_not_cracked_on_a_low_life_total() {
        let mesa = carded(
            permanent(obj(1), PlayerId::new(0), 0),
            "Arid Mesa",
            TypeSet::LAND,
        );
        let v = view(0, &[6, 20], vec![mesa]);

        assert_eq!(
            agent().act(&v, &offering(vec![(obj(1), 0)])),
            PlayerAction::PassPriority
        );
    }

    /// Jace's ultimate is his −12, and at three loyalty the engine offers
    /// only the first three abilities. "The most negative thing available"
    /// would take the −1 every turn until he was gone; reading the ultimate
    /// off the card instead sees it is not on offer, and pluses.
    #[test]
    fn a_walker_pluses_when_its_ultimate_is_out_of_reach() {
        let jace = carded(
            walker(obj(1), PlayerId::new(0), 3),
            "Jace, the Mind Sculptor",
            TypeSet::PLANESWALKER,
        );
        let v = view(0, &[20, 20], vec![jace]);

        assert_eq!(
            agent().act(&v, &offering(vec![(obj(1), 0), (obj(1), 1), (obj(1), 2)])),
            PlayerAction::ActivateAbility {
                source: obj(1),
                ability_index: 0,
            },
            "the walker spent loyalty it should have gained"
        );
    }

    /// Offered the ultimate, it takes the ultimate — the engine only lists a
    /// negative ability the walker can actually pay for.
    #[test]
    fn a_walker_takes_its_ultimate_when_it_is_offered() {
        let jace = carded(
            walker(obj(1), PlayerId::new(0), 12),
            "Jace, the Mind Sculptor",
            TypeSet::PLANESWALKER,
        );
        let v = view(0, &[20, 20], vec![jace]);

        assert_eq!(
            agent().act(
                &v,
                &offering(vec![(obj(1), 0), (obj(1), 1), (obj(1), 2), (obj(1), 3)])
            ),
            PlayerAction::ActivateAbility {
                source: obj(1),
                ability_index: 3,
            }
        );
    }

    /// Delve is a cost, so the agent pays it; every other pile is declined.
    ///
    /// The `ChooseCards` rule answers `min` whenever the list is longer than
    /// two, which is right for a search, a scry and a wish — declining any of
    /// them loses nothing. Delve is the one prompt in that family the engine
    /// asks *during a cast*, and `casting::can_cast` counted the graveyard
    /// when it offered the spell: answering zero leaves a cast that cannot
    /// pay, which is reversed whole and hands priority back with the same
    /// `LegalActions`. A deterministic agent then casts it again, forever.
    ///
    /// Both halves are here on purpose. The first alone would pass just as
    /// well if the arm had been widened for every prompt at once, and an
    /// agent that bottomed its whole hand to a scry would be a worse player
    /// than one that never delved.
    #[test]
    fn the_delve_question_is_a_cost_and_the_agent_pays_it() {
        let graveyard: Vec<ObjectId> = (10..17).map(obj).collect();
        let v = view(0, &[20, 20], vec![]);
        let pile = |prompt| Pending::ChooseCards {
            player: PlayerId::new(0),
            options: graveyard.clone(),
            min: 0,
            max: 6,
            prompt,
        };

        let PlayerAction::ChooseObjects { objects } = agent().act(&v, &pile(ChoicePrompt::Delve))
        else {
            panic!("expected a card choice")
        };
        assert_eq!(
            objects.len(),
            6,
            "the whole reduction the spell was offered on"
        );

        let PlayerAction::ChooseObjects { objects } =
            agent().act(&v, &pile(ChoicePrompt::ScryBottom))
        else {
            panic!("expected a card choice")
        };
        assert!(
            objects.is_empty(),
            "a pile that costs nothing to decline is still declined"
        );

        // A surveil is the one where the `max <= 2` shortcut is actively
        // harmful, because the pile it names goes to a graveyard and does
        // not come back. It is also the shape the shortcut would have
        // caught: a surveil is 1 or 2 on every card in the pool.
        let small = Pending::ChooseCards {
            player: PlayerId::new(0),
            options: graveyard[..2].to_vec(),
            min: 0,
            max: 2,
            prompt: ChoicePrompt::SurveilGraveyard,
        };
        let PlayerAction::ChooseObjects { objects } = agent().act(&v, &small) else {
            panic!("expected a card choice")
        };
        assert!(
            objects.is_empty(),
            "the agent keeps what it cannot read, so a surveil mills it nothing"
        );
    }

    /// The untap step's determination, where naming a permanent is what
    /// costs something.
    ///
    /// A one-permanent menu is the shape the storage lands ask with, and it
    /// is the shape the `max <= 2` shortcut answers with `max`: the agent
    /// would have named the land every turn and never untapped it again.
    /// The second half is the counter-test — the same tiny menu under a
    /// different prompt is still answered with `max`, so this is about the
    /// prompt and not about the size.
    #[test]
    fn the_untap_determination_is_answered_by_untapping() {
        let v = view(0, &[20, 20], vec![]);
        let menu = |prompt| Pending::ChooseCards {
            player: PlayerId::new(0),
            options: vec![obj(10)],
            min: 0,
            max: 1,
            prompt,
        };

        let PlayerAction::ChooseObjects { objects } =
            agent().act(&v, &menu(ChoicePrompt::LeaveTapped))
        else {
            panic!("expected a card choice")
        };
        assert!(
            objects.is_empty(),
            "naming it would leave it tapped for the rest of the game"
        );

        let PlayerAction::ChooseObjects { objects } =
            agent().act(&v, &menu(ChoicePrompt::SearchLibrary))
        else {
            panic!("expected a card choice")
        };
        assert_eq!(objects.len(), 1, "a short menu is otherwise taken whole");
    }

    /// The three modes of Sheoldred's Edict, as the engine offers them.
    ///
    /// No `Normal` option, because every effect the card has sits under a
    /// mode and the wizard refuses a mode-less cast for such a card
    /// (CR 700.2). That is what made the old answer — the position of
    /// `Normal`, else nought — take the first printed mode at every table.
    fn edict_modes() -> Vec<baylee_engine::choice::CastModeDesc> {
        use baylee_engine::choice::{CastModeDesc, CastModeKind};
        (0..3)
            .map(|i| CastModeDesc {
                index: u8::try_from(i).unwrap(),
                kind: CastModeKind::Mode(i),
                cost: baylee_core::mana::ManaCost::ZERO,
            })
            .collect()
    }

    /// A view with Sheoldred's Edict on the stack and `theirs` opposite it.
    fn edict_table(theirs: Vec<PublicObject>) -> PlayerView {
        let mut v = view(0, &[20, 20], theirs);
        v.stack = vec![carded(
            permanent(obj(9), PlayerId::new(0), 0),
            "Sheoldred's Edict",
            TypeSet::INSTANT,
        )];
        v
    }

    fn token_creature(id: ObjectId, controller: PlayerId) -> PublicObject {
        let mut o = permanent(id, controller, 2);
        o.token = Some(1);
        o
    }

    /// "Choose one —" is chosen by what the mode reaches, not by where it is
    /// printed.
    ///
    /// Sheoldred's Edict is the pool's clearest case: all three modes are
    /// always offered, because none of them targets — an edict names no
    /// target at all (CR 115.1) — so the engine's own "a mode that cannot
    /// find its targets is not offered" filter says nothing about any of
    /// them. Against a lone planeswalker the printed first mode asks for a
    /// nontoken creature and does nothing whatsoever, and that is what this
    /// agent used to choose every time.
    ///
    /// The nontoken case is the control: it is the answer the old code gave
    /// as well, so a test containing only it would pass against the defect.
    /// It is a *carded* creature rather than the bare fixture, because
    /// `IsToken` is only readable of an object that carries one of the two
    /// handles — a bare permanent is the pair the view cannot tell apart, and
    /// the control would then agree by falling back instead of by reading.
    #[test]
    fn a_modal_spell_takes_the_mode_that_reaches_something() {
        let them = PlayerId::new(1);
        let theirs = carded(
            permanent(obj(1), them, 2),
            "Baleful Strix",
            TypeSet::CREATURE,
        );
        for (what, board, expected) in [
            ("a lone planeswalker", vec![walker(obj(1), them, 4)], 2),
            ("a lone token", vec![token_creature(obj(1), them)], 1),
            ("a nontoken creature", vec![theirs.clone()], 0),
        ] {
            let v = edict_table(board);
            assert_eq!(
                agent().act(
                    &v,
                    &Pending::ChooseCastMode {
                        player: v.seat,
                        object: obj(9),
                        options: edict_modes(),
                    }
                ),
                PlayerAction::ChooseMode(expected),
                "against {what}, only mode {expected} of Sheoldred's Edict does anything"
            );
        }
    }

    /// An empty board is read, and the reading is that nothing is reached.
    ///
    /// Every mode scores nought, so the printed order is all that is left and
    /// the answer is the old one. Worth pinning because the tie-break is the
    /// half that keeps this change invisible everywhere it has nothing to
    /// say: `max_by_key` returns the *last* maximum, so without the
    /// `Reverse(position)` in the key an empty table would answer 2.
    #[test]
    fn a_modal_spell_with_nothing_to_reach_keeps_its_printed_order() {
        let v = edict_table(vec![]);
        assert_eq!(
            agent().act(
                &v,
                &Pending::ChooseCastMode {
                    player: v.seat,
                    object: obj(9),
                    options: edict_modes(),
                }
            ),
            PlayerAction::ChooseMode(0)
        );
    }

    /// A card that can also be cast normally still is.
    ///
    /// The ranking decides between modes and never between a mode and the
    /// printed cast: a mode is offered exactly when it is affordable, and
    /// overload is the shape that makes "a mode" and "the expensive one" the
    /// same thing. So a `Normal` option ends the question wherever there is
    /// one.
    ///
    /// The options are built by hand because no card in the pool offers this
    /// pair today. All four `ModalSpell` cards here — Sheoldred's Edict,
    /// Heliod's Intervention, Cyclonic Rift, Damn — put *every* effect under
    /// a mode, so the wizard refuses them a `Normal` option (CR 700.2a) and
    /// the two overload cards write their printed cast as `Mode(0)` beside
    /// the overloaded `Mode(1)`. The guard exists for the card that does not,
    /// and a fixture that cannot be printed yet is the only way to hold it
    /// before one is.
    #[test]
    fn a_normal_cast_is_not_traded_for_a_mode() {
        use baylee_engine::choice::{CastModeDesc, CastModeKind};
        let v = edict_table(vec![walker(obj(1), PlayerId::new(1), 4)]);
        let options = vec![
            CastModeDesc {
                index: 0,
                kind: CastModeKind::Mode(0),
                cost: baylee_core::mana::ManaCost::ZERO,
            },
            CastModeDesc {
                index: 1,
                kind: CastModeKind::Normal,
                cost: baylee_core::mana::ManaCost::ZERO,
            },
        ];
        assert_eq!(
            agent().act(
                &v,
                &Pending::ChooseCastMode {
                    player: v.seat,
                    object: obj(9),
                    options,
                }
            ),
            PlayerAction::ChooseMode(1),
            "the printed cast is the answer even when a mode outscores it"
        );
    }

    /// What the view cannot see, the reader says it cannot see.
    ///
    /// Each of the three refusals is a `Filter` whose answer lives in a
    /// `GameState` field the projection has no counterpart for, and the
    /// reason each one is a refusal rather than a gap is in this module's
    /// documentation. `Some(false)` here would be the fault the three-valued
    /// answer exists to prevent: a caller cannot tell a read "no" from an
    /// unread one, so it would act on a guess wearing a reading's clothes.
    ///
    /// `This` and `Another` join them when the caller has no source object,
    /// which is a different kind of unknown with the same honest answer.
    #[test]
    fn a_filter_the_view_cannot_answer_is_not_answered_no() {
        use baylee_cards_dsl::{Filter, ZoneRef};
        let me = PlayerId::new(0);
        let a = agent();
        let mine = permanent(obj(1), me, 2);
        let v = view(0, &[20, 20], vec![mine.clone()]);
        let read = |f: &Filter, this| a.filter_matches(f, &v, &mine, ZoneRef::Battlefield, this);

        for filter in [
            Filter::MatchesChosenTypeOfSource,
            Filter::AttachedToBySource,
            Filter::SharesSubtypeWithCommander,
        ] {
            assert_eq!(
                read(&filter, Some(obj(1))),
                None,
                "{filter:?} reads a field no view carries, and saying `false` \
                 would look exactly like a read answer"
            );
        }
        // `IsToken` is the fourth, and only for the object that carries
        // neither handle: a face-down permanent and a token copying a card
        // are one shape in a view and opposite answers in the rules.
        assert_eq!(read(&Filter::IsToken, Some(obj(1))), None);
        let mut registry = mine.clone();
        registry.token = Some(1);
        assert_eq!(
            a.filter_matches(&Filter::IsToken, &v, &registry, ZoneRef::Battlefield, None),
            Some(true)
        );
        let seen = carded(mine.clone(), "Baleful Strix", TypeSet::CREATURE);
        assert_eq!(
            a.filter_matches(&Filter::IsToken, &v, &seen, ZoneRef::Battlefield, None),
            Some(false)
        );

        assert_eq!(read(&Filter::This, None), None);
        assert_eq!(read(&Filter::Another, None), None);
        assert_eq!(read(&Filter::This, Some(obj(1))), Some(true));
        assert_eq!(read(&Filter::Another, Some(obj(1))), Some(false));
        assert_eq!(read(&Filter::CREATURE, None), Some(true));
    }

    /// An unknown part does not settle a question the rest of it settles.
    ///
    /// Kleene's three-valued `and`/`or`, which is the only combination rule
    /// that keeps a `None` honest: a false conjunct makes an `And` false
    /// however much of the rest is unreadable, a true disjunct makes an `Or`
    /// true the same way, and an unknown wins everywhere else. Reading the
    /// unknown as `false` would answer "nontoken creature that shares a
    /// subtype with your commander" with a confident no on every board.
    #[test]
    fn an_unreadable_part_settles_a_filter_only_where_it_decides_it() {
        use baylee_cards_dsl::{Filter, ZoneRef};
        static UNREADABLE: Filter = Filter::SharesSubtypeWithCommander;
        static AND_FALSE: Filter = Filter::And(&[UNREADABLE, Filter::PLANESWALKER]);
        static AND_TRUE: Filter = Filter::And(&[UNREADABLE, Filter::CREATURE]);
        static OR_TRUE: Filter = Filter::Or(&[UNREADABLE, Filter::CREATURE]);
        static OR_FALSE: Filter = Filter::Or(&[UNREADABLE, Filter::PLANESWALKER]);

        let a = agent();
        let mine = permanent(obj(1), PlayerId::new(0), 2);
        let v = view(0, &[20, 20], vec![mine.clone()]);
        let read = |f: &Filter| a.filter_matches(f, &v, &mine, ZoneRef::Battlefield, Some(obj(1)));

        assert_eq!(read(&AND_FALSE), Some(false), "a false conjunct settles it");
        assert_eq!(read(&AND_TRUE), None, "a true one leaves the unknown");
        assert_eq!(read(&OR_TRUE), Some(true), "a true disjunct settles it");
        assert_eq!(read(&OR_FALSE), None, "a false one leaves the unknown");
        assert_eq!(read(&Filter::Not(&UNREADABLE)), None);
    }

    /// A seat that agreed to a price pays it (CR 605.3a).
    ///
    /// The mana window is an ordinary priority round, which is what hid it:
    /// nothing is castable inside one, so every path in `priority` below the
    /// new first step passed, and the agent lost a spell it had already
    /// agreed to pay for. Three questions, because the failure was that the
    /// first one was never asked and the other two are what stop the answer
    /// being reckless.
    #[test]
    fn a_payment_window_is_answered_by_tapping_toward_the_price() {
        use baylee_core::generated::subtypes::land;
        let mut lands = Vec::new();
        for i in 0..2u32 {
            let mut source = permanent(obj(i + 1), PlayerId::new(0), 0);
            source.types = TypeSet::LAND;
            source.subtypes.insert(land::PLAINS);
            lands.push(source);
        }
        let legal = || {
            Box::new(baylee_engine::choice::LegalActions {
                can_pass: true,
                mana_abilities: (1..=2).map(obj).collect(),
                ..Default::default()
            })
        };
        let base = view(0, &[20, 20], lands);

        // Owed {2} with two Plains offered: one tap, and the next round
        // plans one fewer because `plan` spends the pool first.
        let mut v = base.clone();
        v.awaiting = Some(v.seat);
        v.owed = Some(baylee_core::mana::ManaCost::from_symbol_generic(2));
        let pending = Pending::Priority {
            player: v.seat,
            legal: legal(),
        };
        assert!(
            matches!(
                agent().act(&v, &pending),
                PlayerAction::ActivateManaAbility { .. }
            ),
            "the seat agreed to pay {{2}} and was handed priority over two \
             untapped Plains; passing there is how the spell was lost"
        );

        // Owed {3} with two Plains: the price cannot be reached, so nothing
        // is tapped. A seat that taps two of the three lands it needs has
        // lost the mana and the spell both.
        let mut v = base.clone();
        v.awaiting = Some(v.seat);
        v.owed = Some(baylee_core::mana::ManaCost::from_symbol_generic(3));
        let pending = Pending::Priority {
            player: v.seat,
            legal: legal(),
        };
        assert_eq!(
            agent().act(&v, &pending),
            PlayerAction::PassPriority,
            "a window the seat cannot afford must cost it nothing more"
        );

        // The same price, owed by somebody else. Both fields ride in every
        // view, so a reader taking `owed` without `awaiting` would have this
        // seat paying for an opponent's window.
        let mut v = base;
        v.awaiting = Some(PlayerId::new(1));
        v.owed = Some(baylee_core::mana::ManaCost::from_symbol_generic(2));
        let pending = Pending::Priority {
            player: v.seat,
            legal: legal(),
        };
        assert_eq!(
            agent().act(&v, &pending),
            PlayerAction::PassPriority,
            "this seat is not the one being asked for the payment"
        );
    }

    /// A counterspell printed behind "unless you pay" is a counterspell.
    ///
    /// Flusterstorm and Malevolent Hermit both spell their text as
    /// `PlayerMayPayOr { effect: CounterTargetSpell }` — the spell is
    /// countered unless its controller pays — and that variant carries a
    /// single `&'static Effect` rather than a list. Every hand-rolled walker
    /// in this workspace descended into the lists and stopped, so `meaning`
    /// was handed these two cards and read an effect list that says nothing
    /// at all. An agent holding a counterspell it does not know is a
    /// counterspell holds it for ever: `policy` only casts one when there is
    /// an opposing stack entry, and it never asks unless `counter` is set.
    ///
    /// Two real cards and not a constructed effect, because this is the one
    /// reader whose answer the pool actually moves — the census behind #109
    /// found 35 effects behind that clause and these are the two that any
    /// reader here asks about.
    #[test]
    fn a_counterspell_behind_a_price_is_read_as_one() {
        for name in ["Flusterstorm", "Malevolent Hermit"] {
            let index = baylee_cards::decks::by_name(name).expect("card in the pool");
            let def = baylee_cards::by_index(index).expect("card compiles");
            let counters = def.faces.iter().enumerate().any(|(face, _)| {
                def.abilities_for_face(face).iter().any(|ability| {
                    let effects = match ability {
                        AbilityDef::Spell { effects, .. }
                        | AbilityDef::Triggered { effects, .. }
                        | AbilityDef::Activated { effects, .. }
                        | AbilityDef::ActivatedConditional { effects, .. } => *effects,
                        _ => &[],
                    };
                    tactics::meaning(effects, 0).counter
                })
            });
            assert!(
                counters,
                "{name} counters a spell unless its controller pays, and the \
                 agent reads it as an effect list with no meaning"
            );
        }
    }
}
