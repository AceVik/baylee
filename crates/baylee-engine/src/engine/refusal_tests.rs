//! A refused answer changes nothing, and every question has an answer.
//!
//! Two promises `apply` makes and no single test can hold it to.
//!
//! **A refused answer leaves the engine exactly as it was.** A game record
//! keeps the answers the engine accepted and nothing else, and a replay
//! applies them to a fresh engine. An answer that was refused *and* moved the
//! game is a step the record does not have: the table played on from a state
//! the replay never reaches, and the next recorded answer is put to the wrong
//! question. That is how a refused miracle broke replays (r001 games 1581,
//! 3288 and 3554): "yes" to a Banishing Stroke with nothing to target was
//! refused after the offer had already been spent.
//!
//! **Every question the engine asks has at least one answer it accepts.**
//! A question with none stops the table: a house seat's proposal and its
//! fallback are both refused, and a player can only concede. Dig Through
//! Time asked for two cards from a library of one (r002 games 368, 2675).
//!
//! The sweep plays house decks against each other with a seeded driver that
//! answers from what the question enumerates. At every decision it first
//! puts answers the engine must refuse — the wrong seat, a mode past the
//! end, a card nobody offered — and then the enumerated ones in a random
//! order until one is taken. Every refusal, whichever kind, is compared
//! against a print of every field the engine has (`Engine::fingerprint`),
//! not against `snapshot_hash`, which leaves out on purpose what a replay
//! rebuilds by itself.

use super::testkit::{
    Duel, RegistryLookup, basic_forest, card_index, keep_mulligans, on_battlefield,
    reach_main_phase, tap_all_mana,
};
use super::*;
use crate::choice::{AttackerBound, BlockOption, CardTotal, PlayerAction, default_arrangement};
use baylee_core::ids::Defender;

/// A seeded stream for the driver: splitmix64, small and dependency-free.
struct Dice(u64);

impl Dice {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }

    fn chance(&mut self, percent: u64) -> bool {
        self.next() % 100 < percent
    }

    fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.below(i + 1);
            items.swap(i, j);
        }
    }

    fn pick<T: Copy>(&mut self, items: &[T], count: usize) -> Vec<T> {
        let mut all = items.to_vec();
        self.shuffle(&mut all);
        all.truncate(count);
        all
    }
}

/// The floor under [`rings`]' decisions: 20,799 measured on 2026-09-29,
/// less a tenth.
const RINGS_FLOOR: usize = 18_700;

/// The floors under the bounds deck's sixteen games: decisions, crew totals
/// asked and menace bounds asked, measured on 2026-09-29 (10,069, 76 and
/// 44), each less a tenth.
const BOUNDS_FLOOR: usize = 9_000;
const BOUNDS_TOTALS_FLOOR: usize = 68;
const BOUNDS_BLOCKS_FLOOR: usize = 39;

/// How often, in percent of decisions, the refusals are compared against
/// the whole print rather than the light one.
const WHOLE_PERCENT: u64 = 20;

/// The house decks, by file and by the name their header gives them.
const DECKS: [(&str, &str); 9] = [
    ("allytifact.txt", "Allytifact"),
    ("victory.txt", "Victory"),
    ("maik.txt", "Euro-Highlander"),
    ("kess.txt", "Kess, Dissident Mage"),
    ("schwarzrand.txt", "Schwarzrand"),
    ("weltenbaum.txt", "Weltenbaum"),
    ("breya.txt", "Breya, Etherium Shaper"),
    ("kenrith.txt", "Kenrith, the Returned King"),
    ("tayam.txt", "Tayam, Luminous Enigma"),
];

fn house_deck(file: &str, name: &str) -> baylee_cards::decks::LoadedDeck {
    let path = format!("{}/../../data/decks/{file}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    // The house lists name a printing after each card ("1 Arid Mesa (MH2)
    // 244") and a commander without a count; the acceptance reader wants
    // neither, and the printing changes nothing a rules test can see.
    let rows: String = text
        .lines()
        .map(|line| {
            let line = line.trim();
            let bare = line.split(" (").next().unwrap_or(line);
            if line.is_empty() || line.starts_with('[') || line.starts_with('#') {
                format!("{line}\n")
            } else if bare.starts_with(|c: char| c.is_ascii_digit()) {
                format!("{bare}\n")
            } else {
                format!("1 {bare}\n")
            }
        })
        .collect();
    baylee_cards::decks::load_acceptance(&rows, name).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The answers a question enumerates, as the driver samples them: every
/// shape of answer the question offers, the ones a player would pick most
/// often first, never more than a handful.
// One arm per kind of question, and a kind left out is a question the
// sweep cannot answer.
#[allow(clippy::too_many_lines)]
fn offered(
    engine: &Engine<RegistryLookup>,
    pending: &Pending,
    dice: &mut Dice,
) -> Vec<PlayerAction> {
    let mut out = Vec::new();
    match pending {
        Pending::ChooseDamageEffect {
            choice, options, ..
        } => {
            out.extend(
                options
                    .iter()
                    .map(|option| PlayerAction::ChooseDamageEffect {
                        choice: *choice,
                        effect: option.id,
                    }),
            );
        }
        Pending::AllocatePrevention {
            choice,
            damage,
            total,
            ..
        } => {
            for _ in 0..3 {
                let mut parts = damage.clone();
                dice.shuffle(&mut parts);
                let mut left = *total;
                let allocation = parts
                    .iter()
                    .map(|part| {
                        let n = left.min(part.amount);
                        left -= n;
                        (part.id, n)
                    })
                    .collect();
                out.push(PlayerAction::AllocatePrevention {
                    choice: *choice,
                    allocation,
                });
            }
        }
        Pending::Mulligan { taken, .. } => {
            out.push(PlayerAction::MulliganKeep);
            if *taken < 2 && dice.chance(20) {
                out.insert(0, PlayerAction::MulliganTake);
            }
        }
        Pending::MulliganBottom { player, count } | Pending::DiscardChoice { player, count } => {
            let hand = engine
                .state()
                .zones
                .list(ZoneLocation::Hand(*player))
                .clone();
            for _ in 0..3 {
                out.push(PlayerAction::ChooseObjects {
                    objects: dice.pick(&hand, usize::from(*count)),
                });
            }
        }
        Pending::Priority { legal, .. } => {
            let mut deeds: Vec<PlayerAction> = Vec::new();
            deeds.extend(
                legal
                    .lands
                    .iter()
                    .map(|&card| PlayerAction::PlayLand { card }),
            );
            deeds.extend(
                legal
                    .castable
                    .iter()
                    .map(|&card| PlayerAction::CastSpell { card }),
            );
            deeds.extend(legal.abilities.iter().map(|&(source, ability_index)| {
                PlayerAction::ActivateAbility {
                    source,
                    ability_index,
                }
            }));
            deeds.extend(
                legal
                    .suspendable
                    .iter()
                    .map(|&card| PlayerAction::Suspend { card }),
            );
            dice.shuffle(&mut deeds);
            // Most of the time a seat that can do something does it: a
            // driver that mostly passes plays games where nothing is cast.
            if !deeds.is_empty() && dice.chance(70) {
                out.extend(deeds);
                if legal.can_pass {
                    out.push(PlayerAction::PassPriority);
                }
            } else {
                if legal.can_pass {
                    out.push(PlayerAction::PassPriority);
                }
                out.extend(deeds);
            }
            if !legal.mana_abilities.is_empty() && dice.chance(5) {
                let source = legal.mana_abilities[dice.below(legal.mana_abilities.len())];
                out.insert(0, PlayerAction::ActivateManaAbility { source });
            }
        }
        Pending::ChooseAttackers {
            attackers,
            defenders,
            ..
        } => {
            for _ in 0..3 {
                let mut chosen: Vec<(ObjectId, Defender)> = Vec::new();
                for &a in attackers {
                    if !defenders.is_empty() && dice.chance(60) {
                        chosen.push((a, defenders[dice.below(defenders.len())]));
                    }
                }
                out.push(PlayerAction::DeclareAttackers { attackers: chosen });
            }
            out.push(PlayerAction::DeclareAttackers {
                attackers: attackers
                    .iter()
                    .filter_map(|&a| defenders.first().map(|&d| (a, d)))
                    .collect(),
            });
            out.push(PlayerAction::DeclareAttackers { attackers: vec![] });
        }
        Pending::ChooseBlockers {
            blockers, bounds, ..
        } => {
            for _ in 0..3 {
                let mut chosen: Vec<(ObjectId, ObjectId)> = Vec::new();
                for b in blockers {
                    if !b.attackers.is_empty() && dice.chance(50) {
                        chosen.push((b.blocker, b.attackers[dice.below(b.attackers.len())]));
                    }
                }
                let kept = within_bounds(blockers, bounds, chosen.clone());
                out.push(PlayerAction::DeclareBlockers { blockers: chosen });
                if out.last()
                    != Some(&PlayerAction::DeclareBlockers {
                        blockers: kept.clone(),
                    })
                {
                    out.push(PlayerAction::DeclareBlockers { blockers: kept });
                }
            }
            out.push(PlayerAction::DeclareBlockers { blockers: vec![] });
        }
        Pending::LegendChoice { options, .. } => {
            for &keep in &dice.pick(options, options.len()) {
                out.push(PlayerAction::ChooseObjects {
                    objects: vec![keep],
                });
            }
        }
        Pending::ChooseCards {
            options,
            min,
            max,
            total,
            ..
        } => {
            let (min, max) = (usize::from(*min), usize::from(*max).min(options.len()));
            for _ in 0..4 {
                let count = if max >= min {
                    min + dice.below(max - min + 1)
                } else {
                    min
                };
                out.push(PlayerAction::ChooseObjects {
                    objects: dice.pick(options, count),
                });
            }
            out.push(PlayerAction::ChooseObjects {
                objects: options.iter().copied().take(min).collect(),
            });
            // A count inside the bounds is not yet an answer to a stated
            // total, and the draws above may keep it by luck or not at all.
            if let Some(total) = total
                && let Some(objects) = reaching(options, (min, max), total, dice)
            {
                out.push(PlayerAction::ChooseObjects { objects });
            }
        }
        Pending::ChooseTargets {
            options,
            player_options,
            min,
            max,
            ..
        } => {
            let (min, max) = (
                usize::try_from(*min).unwrap(),
                usize::try_from(*max).unwrap(),
            );
            let everything: Vec<Result<ObjectId, PlayerId>> = options
                .iter()
                .map(|&o| Ok(o))
                .chain(player_options.iter().map(|&p| Err(p)))
                .collect();
            let reach = max.min(everything.len());
            for _ in 0..4 {
                let count = if reach >= min {
                    min + dice.below(reach - min + 1)
                } else {
                    min
                };
                let chosen = dice.pick(&everything, count);
                out.push(PlayerAction::ChooseTargets {
                    objects: chosen.iter().filter_map(|c| c.ok()).collect(),
                    players: chosen.iter().filter_map(|c| c.err()).collect(),
                });
            }
        }
        Pending::ChooseSubtype { options, .. } => {
            for &s in &dice.pick(options, 3) {
                out.push(PlayerAction::ChooseSubtype(s));
            }
        }
        Pending::ChooseColor { options, .. } => {
            for &c in &dice.pick(options, options.len()) {
                out.push(PlayerAction::ChooseColor(c));
            }
        }
        Pending::YesNo { .. } => {
            let first = dice.chance(60);
            out.push(PlayerAction::YesNo(first));
            out.push(PlayerAction::YesNo(!first));
        }
        Pending::ChooseCastMode { options, .. } => {
            let indices: Vec<usize> = options.iter().map(|o| o.index as usize).collect();
            for &i in &dice.pick(&indices, indices.len()) {
                out.push(PlayerAction::ChooseMode(i));
            }
        }
        Pending::ChooseNumber { min, max, .. } => {
            let top = (*max).min(min.saturating_add(8));
            for _ in 0..3 {
                let span = u64::from(top - (*min).min(top)) + 1;
                out.push(PlayerAction::ChooseNumber(
                    min + u32::try_from(dice.next() % span).unwrap_or(0),
                ));
            }
            out.push(PlayerAction::ChooseNumber(*min));
        }
        Pending::ChoosePlayer { options, .. } => {
            for &p in &dice.pick(options, options.len()) {
                out.push(PlayerAction::ChoosePlayer(p));
            }
        }
        Pending::Arrange { cards, piles, .. } => {
            if let Some(piles) = default_arrangement(cards, piles) {
                out.push(PlayerAction::Arrange { piles });
            }
        }
        Pending::ChoosePile { piles, .. } => {
            let indices: Vec<usize> = (0..piles.len()).collect();
            for &i in &dice.pick(&indices, indices.len()) {
                out.push(PlayerAction::ChooseMode(i));
            }
        }
        // Any card's name is an answer (CR 201.4); the one the asking
        // permanent is printed with is always a card of the pool.
        Pending::ChooseCardName { .. } => {
            if let Some(PlanKind::ChooseCardName { object }) = engine.pending_plan
                && let Some(card) = engine.state().object(object).and_then(|o| o.card)
            {
                out.push(PlayerAction::ChooseCardName {
                    card: card.index,
                    face: 0,
                });
            }
        }
        Pending::GameOver(_) => {}
    }
    out
}

/// A choice keeping a stated total: the options in a random order, the
/// heaviest first, taken until there are `min` of them and the total
/// reaches its least, and never more than `max`. `None` where that walk
/// keeps no answer, which for a total with only a least means none does.
fn reaching(
    options: &[ObjectId],
    (min, max): (usize, usize),
    total: &CardTotal,
    dice: &mut Dice,
) -> Option<Vec<ObjectId>> {
    let mut order: Vec<usize> = (0..options.len()).collect();
    dice.shuffle(&mut order);
    order.sort_by_key(|&i| std::cmp::Reverse(total.weights.get(i).copied().unwrap_or(0)));
    let mut chosen = Vec::new();
    for i in order {
        if chosen.len() >= min && total.fault(options, &chosen).is_none() || chosen.len() == max {
            break;
        }
        chosen.push(options[i]);
    }
    ((min..=max).contains(&chosen.len()) && total.fault(options, &chosen).is_none())
        .then_some(chosen)
}

/// `chosen` brought inside every stated blocker bound: an attacker blocked
/// by too few gets more of the creatures offered against it that block
/// nothing yet, or, where too few are left, loses its blockers; one blocked
/// by too many keeps the first `max_blockers`.
fn within_bounds(
    blockers: &[BlockOption],
    bounds: &[AttackerBound],
    mut chosen: Vec<(ObjectId, ObjectId)>,
) -> Vec<(ObjectId, ObjectId)> {
    for bound in bounds {
        let on = |chosen: &[(ObjectId, ObjectId)]| {
            chosen
                .iter()
                .filter(|(_, attacker)| *attacker == bound.attacker)
                .count()
        };
        let least = usize::try_from(bound.min_blockers).unwrap_or(usize::MAX);
        let most = usize::try_from(bound.max_blockers).unwrap_or(usize::MAX);
        let count = on(&chosen);
        if count == 0 {
            continue;
        }
        if count < least {
            for option in blockers {
                if on(&chosen) >= least {
                    break;
                }
                if option.attackers.contains(&bound.attacker)
                    && !chosen.iter().any(|(blocker, _)| *blocker == option.blocker)
                {
                    chosen.push((option.blocker, bound.attacker));
                }
            }
            if on(&chosen) < least {
                chosen.retain(|(_, attacker)| *attacker != bound.attacker);
            }
        } else if count > most {
            let mut kept = 0;
            chosen.retain(|(_, attacker)| {
                if *attacker != bound.attacker {
                    return true;
                }
                kept += 1;
                kept <= most
            });
        }
    }
    chosen
}

/// Answers inside what the question enumerates that break a bound it
/// states: one card whose weight alone falls short of a total, one creature
/// blocking an attacker that takes two or more, a mulligan past the last.
fn stated_faults(pending: &Pending) -> Vec<PlayerAction> {
    let mut out = Vec::new();
    match pending {
        Pending::Mulligan {
            can_take: false, ..
        } => out.push(PlayerAction::MulliganTake),
        Pending::ChooseCards {
            options,
            min,
            max,
            total: Some(total),
            ..
        } if *min <= 1 && *max >= 1 => {
            if let Some(&short) = options
                .iter()
                .find(|&&o| total.fault(options, &[o]).is_some())
            {
                out.push(PlayerAction::ChooseObjects {
                    objects: vec![short],
                });
            }
        }
        Pending::ChooseBlockers {
            blockers, bounds, ..
        } => {
            for bound in bounds.iter().filter(|b| b.min_blockers > 1) {
                if let Some(one) = blockers
                    .iter()
                    .find(|b| b.attackers.contains(&bound.attacker))
                {
                    out.push(PlayerAction::DeclareBlockers {
                        blockers: vec![(one.blocker, bound.attacker)],
                    });
                }
            }
        }
        _ => {}
    }
    out
}

/// The question with the bounds it states taken away, as an engine that
/// held them without saying so would ask it: what [`play`] is told when a
/// test shows that the sweep notices such an engine.
fn unstated(pending: &Pending) -> Pending {
    let mut told = pending.clone();
    match &mut told {
        Pending::ChooseCards { total, .. } => *total = None,
        Pending::ChooseBlockers { bounds, .. } => bounds.clear(),
        Pending::Mulligan { can_take, .. } => *can_take = true,
        _ => {}
    }
    told
}

/// Answers the engine must refuse, whatever the question: a seat that is
/// not being asked, a mode past the end of every list, a card nobody was
/// offered, an answer of the wrong kind; and the answers the question
/// itself names a fault in ([`stated_faults`]).
fn refused(
    engine: &Engine<RegistryLookup>,
    asked: PlayerId,
    pending: &Pending,
) -> Vec<(PlayerId, PlayerAction)> {
    let mut out = Vec::new();
    let bystander = engine
        .state()
        .players
        .iter()
        .map(|p| p.id)
        .find(|&p| p != asked && !engine.state().has_left(p));
    if let Some(other) = bystander
        && engine.mulligans.is_none()
    {
        out.push((other, PlayerAction::PassPriority));
        out.push((other, PlayerAction::YesNo(true)));
        out.push((other, PlayerAction::ChooseObjects { objects: vec![] }));
    }
    let nobody = ObjectId::new(ObjectId::SLOT_MASK - 7, 0);
    out.push((asked, PlayerAction::ChooseMode(usize::MAX / 2)));
    out.push((
        asked,
        PlayerAction::ChooseObjects {
            objects: vec![nobody],
        },
    ));
    out.push((
        asked,
        PlayerAction::ChooseTargets {
            objects: vec![nobody],
            players: vec![],
        },
    ));
    out.push((asked, PlayerAction::CastSpell { card: nobody }));
    out.push((
        asked,
        PlayerAction::ActivateAbility {
            source: nobody,
            ability_index: 0,
        },
    ));
    out.push((asked, PlayerAction::PlayLand { card: nobody }));
    if !matches!(pending, Pending::ChooseNumber { .. }) {
        out.push((asked, PlayerAction::ChooseNumber(u32::MAX)));
    }
    if !matches!(pending, Pending::YesNo { .. }) {
        out.push((asked, PlayerAction::YesNo(true)));
    }
    if !matches!(pending, Pending::Arrange { .. }) {
        out.push((asked, PlayerAction::Arrange { piles: vec![] }));
    }
    out.extend(stated_faults(pending).into_iter().map(|a| (asked, a)));
    out
}

/// What went wrong in one game, collected rather than asserted so a run
/// names every offender.
#[derive(Default)]
struct Findings {
    /// A refused answer that changed a field: (where, the answer, the fields).
    changed: Vec<String>,
    /// A question none of whose sampled answers was accepted.
    unanswerable: Vec<String>,
    /// A question whose own numbers say it has no answer.
    malformed: Vec<String>,
    /// An answer the question did not offer, or one it names a fault in
    /// (`Pending::answer_fault`), taken anyway.
    accepted: Vec<String>,
    /// An answer inside every bound the question states, refused: a
    /// constraint `apply` holds and the question does not say.
    refused_clean: Vec<String>,
    /// Decisions made, so the sweep cannot pass by reaching nothing.
    decisions: usize,
    /// Decisions whose question states a total over the chosen cards
    /// (`ChooseCards::total`).
    totals: usize,
    /// Decisions whose question states a blocker bound on an attacker
    /// (`ChooseBlockers::bounds`).
    bounds: usize,
}

impl Findings {
    fn absorb(&mut self, other: Findings) {
        self.changed.extend(other.changed);
        self.unanswerable.extend(other.unanswerable);
        self.malformed.extend(other.malformed);
        self.accepted.extend(other.accepted);
        self.refused_clean.extend(other.refused_clean);
        self.decisions += other.decisions;
        self.totals += other.totals;
        self.bounds += other.bounds;
    }
}

/// The question's own arithmetic, where it can say there is no answer
/// without trying one.
fn malformed(pending: &Pending) -> Option<String> {
    match pending {
        Pending::ChooseCards {
            options, min, max, ..
        } if usize::from(*min) > options.len() || min > max => Some(format!(
            "ChooseCards min {min} max {max} over {} options",
            options.len()
        )),
        Pending::ChooseTargets {
            options,
            player_options,
            min,
            max,
            ..
        } if usize::try_from(*min).unwrap() > options.len() + player_options.len() || min > max => {
            Some(format!(
                "ChooseTargets min {min} max {max} over {} + {} options",
                options.len(),
                player_options.len()
            ))
        }
        Pending::ChooseNumber { min, max, .. } if min > max => {
            Some(format!("ChooseNumber min {min} > max {max}"))
        }
        Pending::LegendChoice { options, .. } if options.is_empty() => {
            Some("LegendChoice with no options".into())
        }
        Pending::ChooseSubtype { options, .. } if options.is_empty() => {
            Some("ChooseSubtype with no options".into())
        }
        Pending::ChooseColor { options, .. } if options.is_empty() => {
            Some("ChooseColor with no options".into())
        }
        Pending::ChooseCastMode { options, .. } if options.is_empty() => {
            Some("ChooseCastMode with no options".into())
        }
        Pending::ChoosePlayer { options, .. } if options.is_empty() => {
            Some("ChoosePlayer with no options".into())
        }
        _ => None,
    }
}

/// How a sweep's games differ from a plain game of two decks.
#[derive(Clone, Copy)]
struct Rig {
    /// What the driver is told each question is: the engine's own, or, to
    /// show that the sweep notices a bound the question leaves out, one with
    /// its bounds taken away ([`unstated`]).
    told: fn(&Pending) -> Pending,
    /// What each seat has on the battlefield as the game begins.
    board: fn(&mut baylee_core::preset::GamePreset),
}

/// The house decks as they are dealt, asked as the engine asks.
const PLAIN: Rig = Rig {
    told: Pending::clone,
    board: |_| {},
};

/// Plays one game to its end or `cap` decisions.
///
/// Every sampled answer is held to the question both ways: one the question
/// finds no fault in (`Pending::answer_fault`) must be taken, and one it
/// names a fault in must be refused. A sampled answer that is refused is
/// otherwise unremarkable (the driver samples wide on purpose), so long as
/// it changed nothing.
fn play(
    a: &baylee_cards::decks::LoadedDeck,
    b: &baylee_cards::decks::LoadedDeck,
    seed: u64,
    cap: usize,
    rig: Rig,
) -> Findings {
    let mut preset = baylee_cards::decks::preset_for(seed, a, b);
    (rig.board)(&mut preset);
    let mut engine = Engine::new(&preset, RegistryLookup).expect("house decks start a game");
    let mut dice = Dice(seed ^ 0x5EED);
    let mut found = Findings::default();
    let tag = |engine: &Engine<RegistryLookup>, decision: usize| {
        format!(
            "{} v {} seed {seed} decision {decision} turn {}",
            a.name,
            b.name,
            engine.state().turn.number
        )
    };
    for decision in 0..cap {
        let pending = (rig.told)(engine.pending());
        let Some(asked) = pending.asked() else {
            break; // game over
        };
        found.decisions += 1;
        match &pending {
            Pending::ChooseCards { total: Some(_), .. } => found.totals += 1,
            Pending::ChooseBlockers { bounds, .. } if !bounds.is_empty() => found.bounds += 1,
            _ => {}
        }
        if let Some(what) = malformed(&pending) {
            found
                .malformed
                .push(format!("{}: {what}", tag(&engine, decision)));
        }
        // The light print at every refusal, the whole one at a sample of
        // decisions: the whole print is half a megabyte of text in a long
        // game, and the fields it adds (the arena, the base cache, the
        // names) are the ones a refusal is least likely to reach.
        let whole = dice.chance(WHOLE_PERCENT).then(|| engine.fingerprint());
        let light = engine.fingerprint_light();
        let unchanged = |engine: &Engine<RegistryLookup>, found: &mut Findings, what: String| {
            let mut fields = light.differing(&engine.fingerprint_light());
            if fields.is_empty()
                && let Some(whole) = &whole
            {
                fields = whole.differing(&engine.fingerprint());
            }
            if fields.is_empty() {
                return true;
            }
            found.changed.push(format!("{what} changed {fields:?}"));
            false
        };
        for (seat, action) in refused(&engine, asked, &pending) {
            if engine.apply(seat, action.clone()).is_ok() {
                found.accepted.push(format!(
                    "{}: {action:?} by {seat:?} to {pending:?}",
                    tag(&engine, decision)
                ));
                return found;
            }
            let fields = light.differing(&engine.fingerprint_light());
            if !fields.is_empty() {
                found.changed.push(format!(
                    "{}: {action:?} by {seat:?} to {pending:?} changed {fields:?}",
                    tag(&engine, decision)
                ));
                return found;
            }
        }
        let what = format!("{}: the refusals to {pending:?}", tag(&engine, decision));
        if !unchanged(&engine, &mut found, what) {
            return found;
        }
        let mut answered = false;
        for action in offered(&engine, &pending, &mut dice) {
            let fault = pending.answer_fault(&action);
            let applied = engine.apply(asked, action.clone());
            match (fault, &applied) {
                (Some(fault), Ok(())) => {
                    found.accepted.push(format!(
                        "{}: {action:?}, {fault:?} by the question, taken: {pending:?}",
                        tag(&engine, decision)
                    ));
                    return found;
                }
                (None, Err(e)) => found.refused_clean.push(format!(
                    "{}: {action:?} refused ({e:?}): {pending:?}",
                    tag(&engine, decision)
                )),
                _ => {}
            }
            if applied.is_ok() {
                answered = true;
                break;
            }
            let what = format!("{}: {action:?} to {pending:?}", tag(&engine, decision));
            if !unchanged(&engine, &mut found, what) {
                return found;
            }
        }
        if !answered {
            found
                .unanswerable
                .push(format!("{}: {pending:?}", tag(&engine, decision)));
            return found;
        }
    }
    found
}

fn sweep(games: &[(usize, usize, u64)], cap: usize) -> Findings {
    let decks: Vec<_> = DECKS.iter().map(|(f, n)| house_deck(f, n)).collect();
    sweep_over(&decks, games, cap, PLAIN)
}

/// [`sweep`] over any decks, `games` naming them by position.
fn sweep_over(
    decks: &[baylee_cards::decks::LoadedDeck],
    games: &[(usize, usize, u64)],
    cap: usize,
    rig: Rig,
) -> Findings {
    let threads = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
    // Games differ in length by a factor of ten, so each thread takes the
    // next game when it is free rather than a fixed slice: a slice of long
    // games was the whole wall time. What a game finds does not depend on
    // which thread played it, and the findings are sorted below.
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results: Vec<Findings> = std::thread::scope(|scope| {
        let next = &next;
        let handles: Vec<_> = (0..threads.min(games.len()))
            .map(|_| {
                crate::engine::testkit::spawn_named(scope, move || {
                    let mut all = Findings::default();
                    loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some(&(a, b, seed)) = games.get(i) else {
                            break;
                        };
                        all.absorb(play(&decks[a], &decks[b], seed, cap, rig));
                    }
                    all
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("a sweep thread does not panic"))
            .collect()
    });
    let mut all = Findings::default();
    for f in results {
        all.absorb(f);
    }
    for list in [
        &mut all.changed,
        &mut all.unanswerable,
        &mut all.malformed,
        &mut all.accepted,
        &mut all.refused_clean,
    ] {
        list.sort();
    }
    all
}

/// Every pairing of house decks, `seeds` seeds each.
fn pairings(seeds: u64) -> Vec<(usize, usize, u64)> {
    let mut games = Vec::new();
    for a in 0..DECKS.len() {
        for b in 0..DECKS.len() {
            if a != b {
                for s in 0..seeds {
                    games.push((a, b, seed(a, b, s)));
                }
            }
        }
    }
    games
}

fn seed(a: usize, b: usize, s: u64) -> u64 {
    1_000 + s * 97 + (a * 10 + b) as u64
}

/// Two rings over the decks: every deck plays twice from each chair, against
/// its neighbour and against the deck four along. Eighteen games, a quarter
/// of [`pairings`] at one seed, so the gate pays for the promise and not for
/// the whole grid.
fn rings() -> Vec<(usize, usize, u64)> {
    let n = DECKS.len();
    (0..n)
        .flat_map(|a| [(a, (a + 1) % n), (a, (a + 4) % n)])
        .map(|(a, b)| (a, b, seed(a, b, 0)))
        .collect()
}

fn assert_clean(found: &Findings, floor: usize) {
    assert!(
        found.changed.is_empty()
            && found.unanswerable.is_empty()
            && found.malformed.is_empty()
            && found.accepted.is_empty()
            && found.refused_clean.is_empty(),
        "refusals that changed the engine: {:#?}\nquestions nothing answered: {:#?}\nquestions with no answer by their own numbers: {:#?}\nanswers taken that no question offered or that it faulted: {:#?}\nanswers inside every stated bound, refused: {:#?}",
        found.changed,
        found.unanswerable,
        found.malformed,
        found.accepted,
        found.refused_clean
    );
    // A sweep that reaches nothing passes exactly as loudly as one that
    // reaches everything.
    assert!(
        found.decisions >= floor,
        "the sweep reached only {} decisions",
        found.decisions
    );
}

#[test]
fn a_refused_answer_changes_nothing_and_every_question_has_an_answer() {
    let found = sweep(&rings(), 1_500);
    assert_clean(&found, RINGS_FLOOR);
}

/// The whole grid, three seeds a pairing: 216 games, 368,696 decisions and
/// 115 s in a debug build on 2026-09-29, against 18 games and 4.6 s for
/// [`rings`]. `cargo test -p baylee-engine --lib -- --ignored refusal_tests`.
#[test]
#[ignore = "every pairing of house decks at three seeds; two minutes"]
fn every_pairing_refuses_cleanly_and_answers_every_question() {
    let found = sweep(&pairings(3), 3_000);
    // Measured as above, less a tenth.
    assert_clean(&found, 331_800);
}

/// A deck whose games ask the bounds a question states: Vehicles to crew
/// (Crew 2, CR 702.122a: "Tap any number of other untapped creatures you
/// control with total power N or greater") among creatures of power 1 to 3,
/// and menace (CR 702.111b) printed, granted by an Aura and granted to every
/// creature by Goblin War Drums. None of the house decks crews, and few of
/// their creatures have menace.
const BOUNDS_DECK: &str = "[deck:Bounds]
10 Mountain
8 Forest
6 Unlicensed Hearse
4 Goblin War Drums
4 Imposing Visage
4 Viashino Runner
4 Fearful Villager
8 Llanowar Elves
4 Aurochs
8 Wall of Roots
";

/// What each seat of a bounds game begins with on the battlefield: Goblin
/// War Drums, so every creature it controls attacks with menace from the
/// first turn, two Walls of Roots to block one of the other seat's, an
/// Unlicensed Hearse and two Llanowar Elves to crew it or attack. Dealt
/// from the library alone, a game reached a menace attacker two creatures
/// could block once in twelve.
fn bounds_board(preset: &mut baylee_core::preset::GamePreset) {
    const BOARD: [&str; 8] = [
        "Goblin War Drums",
        "Wall of Roots",
        "Wall of Roots",
        "Unlicensed Hearse",
        "Llanowar Elves",
        "Llanowar Elves",
        "Viashino Runner",
        "Viashino Runner",
    ];
    for seat in &mut preset.seats {
        seat.starting_battlefield = BOARD
            .iter()
            .map(|name| {
                let card = baylee_cards::decks::by_name(name).expect("a card of the pool");
                // The deck's own entry, so the permanent has the deck's printing.
                *seat
                    .deck
                    .iter()
                    .find(|e| e.card == card)
                    .expect("the bounds deck lists every card of its board")
            })
            .collect();
    }
}

/// A bounds game as the engine asks it.
const BOUNDS: Rig = Rig {
    told: Pending::clone,
    board: bounds_board,
};

/// A bounds game with the driver told each question less its bounds.
const BOUNDS_UNSTATED: Rig = Rig {
    told: unstated,
    board: bounds_board,
};

/// The bounds deck against itself, `seeds` games.
fn bounds_games(
    seeds: u64,
) -> (
    Vec<baylee_cards::decks::LoadedDeck>,
    Vec<(usize, usize, u64)>,
) {
    let deck = baylee_cards::decks::load_acceptance(BOUNDS_DECK, "Bounds")
        .unwrap_or_else(|e| panic!("the bounds deck: {e}"));
    (vec![deck], (0..seeds).map(|s| (0, 0, 7_000 + s)).collect())
}

/// Every answer inside the bounds a question states is taken, where those
/// bounds are asked: crew's total power and menace's two blockers.
///
/// The trained AI's fuzzer, which answers inside what the question states,
/// was refused 38 crew answers and 11 menace blocks in 2000 games before the
/// question stated them. Each game here also puts, before its sampled
/// answers, one the question names a fault in (a lone Elf to Crew 2, one
/// creature against a menace attacker), and that one must be refused.
#[test]
fn every_answer_inside_a_stated_bound_is_taken_where_vehicles_crew_and_menace_attacks() {
    let (decks, games) = bounds_games(16);
    let found = sweep_over(&decks, &games, 1_500, BOUNDS);
    assert_clean(&found, BOUNDS_FLOOR);
    assert!(
        found.totals >= BOUNDS_TOTALS_FLOOR && found.bounds >= BOUNDS_BLOCKS_FLOOR,
        "the games asked {} crew totals and {} menace bounds",
        found.totals,
        found.bounds
    );
}

/// The sweep notices a bound `apply` holds and the question does not state.
///
/// The same games, with the driver told each question less its stated
/// bounds (what the engine asked before it stated them): the answers the
/// driver then samples inside what it was told include short crews and lone
/// menace blockers, `apply` refuses them, and the sweep names each as an
/// answer inside every stated bound, refused. A sweep blind to that would
/// have passed the engine the fuzzer found.
#[test]
fn the_sweep_names_a_bound_apply_holds_and_the_question_does_not_state() {
    let (decks, games) = bounds_games(16);
    let found = sweep_over(&decks, &games, 1_500, BOUNDS_UNSTATED);
    let named = |what: &str| found.refused_clean.iter().any(|f| f.contains(what));
    assert!(
        named("CostCrew") && named("DeclareBlockers"),
        "an unstated crew total and an unstated menace bound are both named: {:#?}",
        found.refused_clean
    );
    assert!(
        found.accepted.is_empty() && found.changed.is_empty(),
        "and the rest of the promise holds: {:#?} {:#?}",
        found.accepted,
        found.changed
    );
}

/// The watch for endless loops and the latches beside it are put back when
/// an answer is refused.
///
/// `apply` moved them as the answer arrived, before anything could tell
/// whether it would be taken, and kept them when it was not: every refused
/// answer counted one more step towards a loop the game was not in. The
/// sweep above found it at the first decision of every game.
#[test]
fn a_refused_answer_moves_no_field_of_the_engine() {
    let deck = house_deck(DECKS[0].0, DECKS[0].1);
    let preset = baylee_cards::decks::preset_for(3, &deck, &deck);
    let mut engine = Engine::new(&preset, RegistryLookup).expect("the table builds");
    let asked = engine.pending().asked().expect("a mulligan is asked");
    let before = engine.fingerprint();
    assert!(engine.apply(asked, PlayerAction::ChooseMode(99)).is_err());
    let moved = before.differing(&engine.fingerprint());
    assert!(moved.is_empty(), "a refused answer moved {moved:?}");
}

/// A press refused after its checklist began moves nothing either.
///
/// `start_activation` reads the ability's list before anything is paid and
/// checks the payment after it (CR 602.2b, 601.2h), so a press refused at the
/// payment had written the list down already, and kept it: the record has no
/// such press, and a replay no such list. Nothing the offer allows reaches
/// that refusal today. The pool is emptied behind the offer's back here, so
/// that the promise is held where it is kept, in `apply`, and not in two
/// probes agreeing.
#[test]
fn a_press_refused_at_its_payment_moves_no_field_of_the_engine() {
    let seat = PlayerId::new(0);
    let map = card_index("0b55eac6-a745-4bf4-8926-5ce83bc38d7d"); // Treasure Map
    let mut engine = Duel::new(267, basic_forest())
        .battlefield(0, &[basic_forest(), map])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, seat);
    tap_all_mana(&mut engine, seat);
    let source = on_battlefield(&engine, seat, map).expect("the map is out");
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(source, 0)),
        "the map's ability is offered off a floating {{G}}"
    );
    engine.state.players[0].mana_pool = baylee_core::mana::ManaPool::new();
    let before = engine.fingerprint();
    let press = PlayerAction::ActivateAbility {
        source,
        ability_index: 0,
    };
    assert!(engine.apply(seat, press).is_err(), "nothing pays the {{1}}");
    let moved = before.differing(&engine.fingerprint());
    assert!(moved.is_empty(), "a refused press moved {moved:?}");
}

/// A land the offer lists in `mana_abilities` is taken when it is pressed.
///
/// The fuzzer's repro, Allytifact v Schwarzrand at seed 1198: at decision
/// 1251 the sweep's driver pressed a dual land the offer listed for
/// Chromatic Lantern's grant, and `apply` refused it. The offer asked whether
/// the CR 305.6 shortcut had a colour to give; `apply` asked only whether the
/// land could be tapped, and a dual's shortcut is empty because the card
/// prints both colours. Where in a game such a land is first listed moves
/// with every card and every house answer (a merge of card rounds moved it
/// out of that game), so the pairing is played on the driver's answers,
/// seed after seed, until the offer lists one, and that one is pressed.
#[test]
fn a_dual_land_listed_for_the_lanterns_grant_is_taken_when_pressed() {
    let (a, b) = (0, 4);
    let first = house_deck(DECKS[a].0, DECKS[a].1);
    let second = house_deck(DECKS[b].0, DECKS[b].1);
    for s in 0..8 {
        let seed = seed(a, b, s);
        let preset = baylee_cards::decks::preset_for(seed, &first, &second);
        let mut engine = Engine::new(&preset, RegistryLookup).expect("house decks start a game");
        let mut dice = Dice(seed ^ 0x5EED);
        for decision in 0..3_000 {
            let pending = engine.pending().clone();
            let Some(asked) = pending.asked() else {
                break; // game over
            };
            let listed = match &pending {
                Pending::Priority { legal, .. } => {
                    legal.mana_abilities.iter().copied().find(|&source| {
                        crate::casting::can_activate_mana(engine.state(), asked, source)
                            && crate::casting::intrinsic_mana_offer(
                                engine.state(),
                                &RegistryLookup,
                                source,
                            )
                            .is_empty()
                    })
                }
                _ => None,
            };
            if let Some(source) = listed {
                engine
                    .apply(asked, PlayerAction::ActivateManaAbility { source })
                    .unwrap_or_else(|e| {
                        panic!("seed {seed} decision {decision}: a press the offer listed is taken: {e:?}")
                    });
                let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
                    panic!("the Lantern asks for a colour: {:?}", engine.pending())
                };
                assert_eq!(options.len(), 5, "any colour: {options:?}");
                engine
                    .apply(asked, PlayerAction::ChooseColor(options[0]))
                    .expect("a colour the engine offered");
                assert!(
                    engine
                        .state()
                        .object(source)
                        .is_some_and(|o| o.status.contains(crate::object::Status::TAPPED)),
                    "the land paid its {{T}}"
                );
                return;
            }
            // `play`'s draws, in `play`'s order: the whole print, then the answers.
            let _ = dice.chance(WHOLE_PERCENT);
            let answered = offered(&engine, &pending, &mut dice)
                .into_iter()
                .any(|action| engine.apply(asked, action).is_ok());
            assert!(answered, "seed {seed} decision {decision} has an answer");
        }
    }
    panic!("no game of the pairing listed a dual land for the Lantern's grant");
}
