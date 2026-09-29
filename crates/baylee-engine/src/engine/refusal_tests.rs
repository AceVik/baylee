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
use crate::choice::{PlayerAction, default_arrangement};
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
        Pending::ChooseBlockers { blockers, .. } => {
            for _ in 0..3 {
                let mut chosen: Vec<(ObjectId, ObjectId)> = Vec::new();
                for b in blockers {
                    if !b.attackers.is_empty() && dice.chance(50) {
                        chosen.push((b.blocker, b.attackers[dice.below(b.attackers.len())]));
                    }
                }
                out.push(PlayerAction::DeclareBlockers { blockers: chosen });
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
            options, min, max, ..
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
        }
        Pending::ChooseTargets {
            options,
            player_options,
            min,
            max,
            ..
        } => {
            let (min, max) = (usize::from(*min), usize::from(*max));
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

/// Answers the engine must refuse, whatever the question: a seat that is
/// not being asked, a mode past the end of every list, a card nobody was
/// offered, an answer of the wrong kind.
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
    /// An answer the question did not offer, taken anyway.
    accepted: Vec<String>,
    /// Decisions made, so the sweep cannot pass by reaching nothing.
    decisions: usize,
}

impl Findings {
    fn absorb(&mut self, other: Findings) {
        self.changed.extend(other.changed);
        self.unanswerable.extend(other.unanswerable);
        self.malformed.extend(other.malformed);
        self.accepted.extend(other.accepted);
        self.decisions += other.decisions;
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
        } if usize::from(*min) > options.len() + player_options.len() || min > max => {
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

/// Plays one game to its end or `cap` decisions.
fn play(
    a: &baylee_cards::decks::LoadedDeck,
    b: &baylee_cards::decks::LoadedDeck,
    seed: u64,
    cap: usize,
) -> Findings {
    let preset = baylee_cards::decks::preset_for(seed, a, b);
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
        let pending = engine.pending().clone();
        let Some(asked) = pending.asked() else {
            break; // game over
        };
        found.decisions += 1;
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
            if engine.apply(asked, action.clone()).is_ok() {
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
    let threads = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
    // Games differ in length by a factor of ten, so each thread takes the
    // next game when it is free rather than a fixed slice: a slice of long
    // games was the whole wall time. What a game finds does not depend on
    // which thread played it, and the findings are sorted below.
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results: Vec<Findings> = std::thread::scope(|scope| {
        let (decks, next) = (&decks, &next);
        let handles: Vec<_> = (0..threads.min(games.len()))
            .map(|_| {
                scope.spawn(move || {
                    let mut all = Findings::default();
                    loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some(&(a, b, seed)) = games.get(i) else {
                            break;
                        };
                        all.absorb(play(&decks[a], &decks[b], seed, cap));
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
            && found.accepted.is_empty(),
        "refusals that changed the engine: {:#?}\nquestions nothing answered: {:#?}\nquestions with no answer by their own numbers: {:#?}\nanswers taken that no question offered: {:#?}",
        found.changed,
        found.unanswerable,
        found.malformed,
        found.accepted
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
