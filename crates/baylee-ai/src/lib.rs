//! baylee-ai — heuristic AI controllers with difficulty profiles (M3).
//!
//! An AI seat is a client. It receives exactly what a networked player
//! receives — a hidden-information-filtered [`PlayerView`] plus the
//! [`Pending`] choice addressed to it — and answers with a
//! [`PlayerAction`]. Difficulty is parameterized through [`AIProfile`],
//! never duplicated logic.
//!
//! That boundary is the point, and it is enforced by the type system: this
//! crate cannot see an opponent's hand, the contents of any library, or a
//! face-down permanent, because none of them exist in what it is handed.
//! It used to take `&Engine` and read the whole `GameState`, and the note
//! here said "convention, not enforcement".

#![warn(missing_docs)]

use baylee_core::ids::{Defender, ObjectId, PlayerId};
pub use baylee_core::preset::AIProfile;
use baylee_core::preset::Politics;
use baylee_engine::choice::{Pending, PlayerAction, YesNoPrompt};
use baylee_view::PlayerView;

/// A greedy one-ply heuristic controller. Deterministic given the same
/// view (the engine's seeded RNG does all randomness).
#[derive(Clone, Debug)]
pub struct HeuristicAgent {
    /// Difficulty knobs. `politics` steers who this seat attacks; the
    /// evaluation knobs (lookahead, temperature, mulligan skill, hold-up)
    /// are still read by nobody and wait on the evaluator.
    profile: AIProfile,
    /// Which side each seat plays for, in seat order. Empty means a table
    /// with no teams on it, where every seat is a side of its own.
    teams: Vec<Option<u8>>,
}

impl HeuristicAgent {
    /// A default-profile agent at a table with no teams.
    #[must_use]
    pub fn new(profile: AIProfile) -> Self {
        Self {
            profile,
            teams: Vec::new(),
        }
    }

    /// Tells the agent which side each seat plays for, in seat order.
    ///
    /// Teams are part of the *setup*, not of the state, which is why they
    /// arrive here rather than in a [`PlayerView`]: they are as public as the
    /// format itself, so this hands the agent nothing a networked seat lacks.
    #[must_use]
    pub fn with_teams(mut self, teams: Vec<Option<u8>>) -> Self {
        self.teams = teams;
        self
    }

    /// Whether `seat` is an *opponent* of `me` â a different side, not merely
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
                    .wrapping_add(u64::from(view.seat.get()))
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
    #[allow(clippy::too_many_lines)] // the pending taxonomy is one flat table
    pub fn act(&self, view: &PlayerView, pending: &Pending) -> PlayerAction {
        let player = view.seat;
        match pending.clone() {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::MulliganBottom { count, .. } | Pending::DiscardChoice { count, .. } => {
                // Bottom (or pitch) the highest-cost cards; keep lands and
                // cheap plays.
                PlayerAction::ChooseObjects {
                    objects: costliest(view, count as usize),
                }
            }
            Pending::Priority { legal, .. } => {
                // 1. Play a land.
                if let Some(&card) = legal.lands.first() {
                    return PlayerAction::PlayLand { card };
                }
                // 2. Tap mana while holding anything castable-but-unpaid
                //    or while unspent mana could matter (simple: always
                //    tap before casting, never float into the pass).
                if !legal.castable.is_empty() && !legal.mana_abilities.is_empty() {
                    let floating = mana_available(view);
                    let best_unpaid = legal
                        .castable
                        .iter()
                        .any(|id| mana_value(view, *id) > floating);
                    if best_unpaid {
                        return PlayerAction::ActivateManaAbility {
                            source: legal.mana_abilities[0],
                        };
                    }
                }
                // 3. Cast the costliest castable spell.
                if let Some(card) = legal
                    .castable
                    .iter()
                    .max_by_key(|id| mana_value(view, **id))
                    .copied()
                {
                    return PlayerAction::CastSpell { card };
                }
                // 4. Activated abilities are NOT used by the v1
                //    heuristic — blind activation loops on free no-op
                //    abilities. (Equipment/loyalty use comes with
                //    evaluation in a later difficulty tier.)
                PlayerAction::PassPriority
            }
            // Who may attack and what may be attacked both come from the
            // choice: the engine is the only thing that knows a Wall may
            // not swing and which planeswalker is attackable (CR 508.1a).
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
                let defender = aim_at(view, victim, &squad, &defenders);
                let attackers = squad.into_iter().map(|id| (id, defender)).collect();
                PlayerAction::DeclareAttackers { attackers }
            }
            Pending::ChooseBlockers { .. } => PlayerAction::DeclareBlockers { blockers: vec![] },
            Pending::LegendChoice { options, .. } => PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
            Pending::ChooseCards {
                options, min, max, ..
            } => {
                let n = if max <= 2 { max } else { min };
                PlayerAction::ChooseObjects {
                    objects: options[..(n as usize).min(options.len())].to_vec(),
                }
            }
            Pending::ChooseTargets {
                options,
                player_options,
                min,
                max,
                ..
            } => {
                let n = (if max <= 2 { max } else { min }) as usize;
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
                for id in &options {
                    if !ordered.contains(id) {
                        ordered.push(*id);
                    }
                }
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
                // Ally tribal decks: prefer ALLY, else the first type.
                let ally = baylee_core::generated::subtypes::creature::ALLY;
                PlayerAction::ChooseSubtype(if options.contains(&ally) {
                    ally
                } else {
                    options[0]
                })
            }
            Pending::ChooseColor { options, .. } => PlayerAction::ChooseColor(options[0]),
            Pending::ChooseNumber { min, .. } => PlayerAction::ChooseNumber(min),
            Pending::ChoosePlayer { options, .. } => PlayerAction::ChoosePlayer(
                options
                    .iter()
                    .copied()
                    .find(|p| self.hostile(*p, player))
                    .or_else(|| options.iter().copied().find(|p| *p != player))
                    .unwrap_or(options[0]),
            ),
            Pending::ChooseCastMode { options, .. } => PlayerAction::ChooseMode(
                options
                    .iter()
                    .position(|o| matches!(o.kind, baylee_engine::choice::CastModeKind::Normal))
                    .unwrap_or(0),
            ),
            Pending::OrderObjects { objects, .. } => PlayerAction::OrderObjects { objects },
            Pending::YesNo { prompt, .. } => match prompt {
                YesNoPrompt::PayLifeOrEnterTapped { amount } => PlayerAction::YesNo(
                    view.seat(player)
                        .is_some_and(|s| s.life > i32::from(amount) + 5),
                ),
                YesNoPrompt::Miracle { .. } => PlayerAction::YesNo(mana_available(view) >= 2),
                // Kicker and tax are declined to keep the mana; a draw is
                // declined because the house AI has no match score to protect,
                // so accepting would only ever be a game given away.
                YesNoPrompt::Kicker
                | YesNoPrompt::PayTax { .. }
                | YesNoPrompt::DrawOffer { .. } => PlayerAction::YesNo(false),
                YesNoPrompt::Generic => PlayerAction::YesNo(true),
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

/// What a card the seat may cast costs.
///
/// Anything castable is either in hand or somewhere public (a graveyard
/// with flashback, exile with a wish), and both are in the view.
fn mana_value(view: &PlayerView, id: ObjectId) -> u32 {
    if let Some(card) = view.hand.iter().find(|c| c.id == id) {
        return card.mana_value;
    }
    view.object(id).map_or(0, |o| o.mana_value)
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
fn mana_available(view: &PlayerView) -> u32 {
    view.seat(view.seat).map_or(0, |s| s.mana_pool.total())
}

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
    use baylee_core::color::ColorSet;
    use baylee_core::types::{SubtypeSet, SupertypeSet, TypeSet};
    use baylee_view::{
        CombatView, CounterEntry, CounterKind, ObjectStatus, PlayerView, PublicObject, SeatView,
    };

    fn obj(slot: u32) -> ObjectId {
        ObjectId::new(slot, 0)
    }

    /// One permanent on the battlefield, as the seat sees it.
    fn permanent(id: ObjectId, controller: PlayerId, power: i16) -> PublicObject {
        PublicObject {
            id,
            card: None,
            name: "Creature".into(),
            controller,
            owner: controller,
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
            loyalty: None,
            damage: 0,
            counters: vec![],
            attached_to: None,
            targets: vec![],
            stack_item: None,
            summoning_sick: false,
        }
    }

    /// A planeswalker with `loyalty` counters on it.
    fn walker(id: ObjectId, controller: PlayerId, loyalty: u16) -> PublicObject {
        PublicObject {
            name: "Walker".into(),
            types: TypeSet::PLANESWALKER,
            power: None,
            toughness: None,
            loyalty: Some(loyalty),
            counters: vec![CounterEntry {
                kind: CounterKind::Loyalty,
                count: loyalty,
            }],
            ..permanent(id, controller, 0)
        }
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
                commander_casts: vec![],
            })
            .collect();
        PlayerView {
            seq: 7,
            seat: PlayerId::new(seat),
            turn: 3,
            phase: baylee_view::Phase::Combat,
            step: baylee_view::Step::DeclareAttackers,
            active: PlayerId::new(seat),
            priority: None,
            monarch: None,
            seats,
            hand: vec![],
            battlefield,
            stack: vec![],
            graveyards: vec![vec![]; lives.len()],
            exile: vec![vec![]; lives.len()],
            command: vec![vec![]; lives.len()],
            combat: CombatView::default(),
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
}
