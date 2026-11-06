//! Counts classes of the evaluated seat's choices, for finding what it still
//! gets wrong: mana left idle, spells that do nothing, attacks and blocks a
//! rule can read, keeps. Every class is a *candidate*: it says what a reader
//! should look at, not that the answer was wrong. `--cases` writes each one
//! with both boards, so a sample can be judged by hand.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufWriter, Write as _};
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use baylee_cards_dsl::KeywordSet;
use baylee_core::ids::{Defender, ObjectId, PlayerId};
use baylee_core::mana::ManaColor;
use baylee_core::types::{SupertypeSet, TypeSet};
use baylee_engine::choice::{BlockOption, LegalActions, Pending, PlayerAction};
use baylee_train::features4::untapped_mana_production;
use baylee_train::netplay4::reading;
use baylee_view::{Phase, PlayerView, PublicObject};
use serde_json::json;

use crate::game::{Decision, Event, Game};
use crate::losses::{self, Which};
use crate::names::Names;
use crate::show;

/// One candidate.
struct Case {
    class: &'static str,
    deal: u64,
    turn: u32,
    lost: bool,
    units: u64,
    cards: Vec<String>,
    text: String,
}

/// A class over the games counted.
#[derive(Default)]
struct Class {
    cases: usize,
    in_lost: usize,
    units: u64,
    games: BTreeSet<u64>,
    cards: BTreeMap<String, usize>,
}

/// What the games showed.
#[derive(Default)]
struct Survey {
    games: usize,
    lost_games: usize,
    turns: usize,
    own_turns: usize,
    decisions: usize,
    kinds: BTreeMap<String, usize>,
    classes: BTreeMap<&'static str, Class>,
    cases: Vec<Case>,
}

impl Survey {
    fn merge(&mut self, other: Self) {
        self.games += other.games;
        self.lost_games += other.lost_games;
        self.turns += other.turns;
        self.own_turns += other.own_turns;
        self.decisions += other.decisions;
        for (kind, count) in other.kinds {
            *self.kinds.entry(kind).or_default() += count;
        }
        for case in other.cases {
            let class = self.classes.entry(case.class).or_default();
            class.cases += 1;
            class.in_lost += usize::from(case.lost);
            class.units += case.units;
            class.games.insert(case.deal);
            for card in &case.cards {
                *class.cards.entry(card.clone()).or_default() += 1;
            }
            self.cases.push(case);
        }
    }

    fn print(&self) {
        println!(
            "{} games ({} lost), {} turns, {} own turns, {} decisions not forced",
            self.games, self.lost_games, self.turns, self.own_turns, self.decisions
        );
        let games = self.games.max(1) as f64;
        let won = (self.games - self.lost_games).max(1) as f64;
        let lost = self.lost_games.max(1) as f64;
        println!(
            "\n{:<52} {:>6} {:>6} {:>6} {:>7} {:>7} {:>7}",
            "class", "cases", "/game", "games", "/won", "/lost", "units"
        );
        let mut rows: Vec<_> = self.classes.iter().collect();
        rows.sort_by_key(|(_, class)| std::cmp::Reverse(class.cases));
        for (name, class) in &rows {
            println!(
                "{name:<52} {:>6} {:>6.2} {:>6} {:>7.2} {:>7.2} {:>7}",
                class.cases,
                class.cases as f64 / games,
                class.games.len(),
                (class.cases - class.in_lost) as f64 / won,
                class.in_lost as f64 / lost,
                class.units
            );
        }
        for (name, class) in &rows {
            let mut cards: Vec<_> = class.cards.iter().collect();
            cards.sort_by_key(|(card, count)| (std::cmp::Reverse(**count), (*card).clone()));
            let top: Vec<String> = cards
                .iter()
                .take(14)
                .map(|(card, count)| format!("{card} {count}"))
                .collect();
            if !top.is_empty() {
                println!("\n{name}: {}", top.join(", "));
            }
        }
        println!("\nQuestion / answer, decisions not forced:");
        let mut kinds: Vec<_> = self.kinds.iter().collect();
        kinds.sort_by_key(|(kind, count)| (std::cmp::Reverse(**count), (*kind).clone()));
        for (kind, count) in kinds.iter().take(30) {
            println!("  {count:>8}  {kind}");
        }
    }

    fn write_cases(&mut self, path: &Path) -> anyhow::Result<()> {
        self.cases.sort_by_key(|case| (case.class, case.deal, case.turn));
        let mut out = BufWriter::new(File::create(path)?);
        for case in &self.cases {
            serde_json::to_writer(
                &mut out,
                &json!({"class": case.class, "deal": case.deal, "turn": case.turn,
                        "lost": case.lost, "units": case.units, "cards": case.cards,
                        "text": case.text}),
            )?;
            out.write_all(b"\n")?;
        }
        Ok(out.flush()?)
    }
}

fn has(object: &PublicObject, keyword: KeywordSet) -> bool {
    object.keywords & keyword.bits() != 0
}

fn power(object: &PublicObject) -> i32 {
    i32::from(object.power.unwrap_or(0))
}

/// Toughness left after the damage already marked.
fn staying(object: &PublicObject) -> i32 {
    i32::from(object.toughness.unwrap_or(0)) - i32::try_from(object.damage).unwrap_or(0)
}

/// Combat damage it deals: twice with double strike.
fn hits(object: &PublicObject) -> i32 {
    power(object).max(0) * if has(object, KeywordSet::DOUBLE_STRIKE) { 2 } else { 1 }
}

fn creature(object: &PublicObject) -> String {
    let tapped = if object.status.is_tapped() { " T" } else { "" };
    match (object.power, object.toughness) {
        (Some(power), Some(toughness)) => format!("{} {power}/{toughness}{tapped}", object.name),
        _ => format!("{}{tapped}", object.name),
    }
}

fn pool(view: &PlayerView, seat: PlayerId) -> u64 {
    view.seat(seat).map_or(0, |info| info.mana_pool.total())
}

fn opponent(view: &PlayerView, seat: PlayerId) -> Option<PlayerId> {
    view.seats
        .iter()
        .map(|info| info.player)
        .find(|player| *player != seat)
}

fn life(view: &PlayerView, player: PlayerId) -> i32 {
    view.seat(player).map_or(0, |info| info.life)
}

/// What a reader weighs: turn, life, mana, hand and both boards.
fn context(view: &PlayerView, seat: PlayerId) -> String {
    let foe = opponent(view, seat).unwrap_or(seat);
    let hand: Vec<&str> = view.hand.iter().map(|card| card.name.as_str()).collect();
    let board = |player: PlayerId, lands: bool| -> String {
        view.battlefield_of(player)
            .filter(|object| object.types.contains(TypeSet::LAND) == lands)
            .map(creature)
            .collect::<Vec<_>>()
            .join(", ")
    };
    let lands = |player: PlayerId| {
        view.battlefield_of(player)
            .filter(|object| object.types.contains(TypeSet::LAND))
            .count()
    };
    format!(
        "T{} {:?} life {}/{} lands {}/{} mana {}+{} hand [{}] mine [{}] theirs [{}]",
        view.turn,
        view.phase,
        life(view, seat),
        life(view, foe),
        lands(seat),
        lands(foe),
        untapped_mana_production(view, seat),
        pool(view, seat),
        hand.join(", "),
        board(seat, false),
        board(foe, false)
    )
}

fn can_block(blocker: &PublicObject, attacker: &PublicObject) -> bool {
    if has(attacker, KeywordSet::FLYING)
        && !has(blocker, KeywordSet::FLYING)
        && !has(blocker, KeywordSet::REACH)
    {
        return false;
    }
    !has(attacker, KeywordSet::SHADOW) || has(blocker, KeywordSet::SHADOW)
}

/// One game's reader.
struct Reader {
    seat: PlayerId,
    deal: u64,
    lost: bool,
    /// Mana floated and lost, or tapped with no sink, this own turn.
    wasted: bool,
    survey: Survey,
}

impl Reader {
    fn note(
        &mut self,
        class: &'static str,
        view: &PlayerView,
        units: u64,
        cards: Vec<String>,
        extra: &str,
    ) {
        let text = if extra.is_empty() {
            context(view, self.seat)
        } else {
            format!("{} · {extra}", context(view, self.seat))
        };
        self.survey.cases.push(Case {
            class,
            deal: self.deal,
            turn: view.turn,
            lost: self.lost,
            units,
            cards,
            text,
        });
    }

    fn pass(&mut self, view: &PlayerView, legal: &LegalActions, names: &Names) {
        let seat = self.seat;
        let floating = pool(view, seat);
        if floating > 0 && view.stack.is_empty() {
            self.wasted = true;
            self.note("pool: passed with mana floating", view, floating, vec![], "");
        }
        if view.active != seat
            || view.phase != Phase::SecondMain
            || !view.stack.is_empty()
            || view.owed.is_some()
        {
            return;
        }
        let reach = reading::reachable_spells(view, legal);
        let instant = |id: &ObjectId| {
            view.hand
                .iter()
                .find(|card| card.id == *id)
                .is_some_and(|card| card.types.contains(TypeSet::INSTANT))
        };
        let (quick, slow): (Vec<ObjectId>, Vec<ObjectId>) = reach.into_iter().partition(instant);
        let (worth, idle): (Vec<ObjectId>, Vec<ObjectId>) = slow
            .iter()
            .partition(|id| reading::worth_casting(view, **id));
        let named = |ids: &[ObjectId]| ids.iter().map(|id| names.name(*id)).collect::<Vec<_>>();
        if !worth.is_empty() {
            let cheapest = worth
                .iter()
                .filter_map(|id| reading::spell_cost(view, *id))
                .min()
                .unwrap_or(0);
            self.note(
                "M2 pass: proactive spell reachable, worth it",
                view,
                u64::from(cheapest),
                named(&worth),
                "",
            );
        } else if !idle.is_empty() {
            self.note(
                "M2 pass: proactive spell reachable, does nothing",
                view,
                0,
                named(&idle),
                "",
            );
        } else if !quick.is_empty() {
            self.note("M2 pass: only instants reachable", view, 0, named(&quick), "");
        }
        let abilities: Vec<String> = legal
            .abilities
            .iter()
            .filter(|(source, index)| {
                !reading::makes_mana(
                    view,
                    &PlayerAction::ActivateAbility {
                        source: *source,
                        ability_index: *index,
                    },
                )
            })
            .map(|(source, _)| names.name(*source))
            .collect();
        if !abilities.is_empty() {
            self.note("M2 pass: paid ability on offer", view, 0, abilities, "");
        }
    }

    fn attack(
        &mut self,
        view: &PlayerView,
        offered: &[ObjectId],
        declared: &[(ObjectId, Defender)],
    ) {
        let seat = self.seat;
        let Some(foe) = opponent(view, seat) else {
            return;
        };
        let walls: Vec<&PublicObject> = view
            .battlefield_of(foe)
            .filter(|object| {
                object.types.contains(TypeSet::CREATURE) && !object.status.is_tapped()
            })
            .collect();
        let swing: i32 = view
            .battlefield_of(foe)
            .filter(|object| object.types.contains(TypeSet::CREATURE))
            .map(hits)
            .sum();
        let attacking = |id: ObjectId| declared.iter().any(|(attacker, _)| *attacker == id);
        let free: Vec<&PublicObject> = offered
            .iter()
            .filter(|id| !attacking(**id))
            .filter_map(|id| view.object(*id))
            .filter(|object| hits(object) > 0)
            .filter(|object| !walls.iter().any(|wall| can_block(wall, object)))
            .collect();
        if !free.is_empty() {
            let cards: Vec<String> = free.iter().map(|object| object.name.clone()).collect();
            let units = free.iter().map(|object| hits(object)).sum::<i32>();
            let class = if swing < life(view, seat) {
                "attack: free attacker stayed home, no lethal back"
            } else {
                "attack: free attacker stayed home, lethal back"
            };
            self.note(class, view, u64::try_from(units).unwrap_or(0), cards, "");
        }
        // What gets through if each untapped creature blocks the biggest
        // attacker it can.
        let through = |ids: &[ObjectId]| -> i32 {
            let mut left: Vec<&PublicObject> = walls.clone();
            let mut attackers: Vec<&PublicObject> =
                ids.iter().filter_map(|id| view.object(*id)).collect();
            attackers.sort_by_key(|object| std::cmp::Reverse(hits(object)));
            let mut total = 0;
            for attacker in attackers {
                if let Some(index) = left.iter().position(|wall| can_block(wall, attacker)) {
                    left.remove(index);
                } else {
                    total += hits(attacker);
                }
            }
            total
        };
        let chosen: Vec<ObjectId> = declared.iter().map(|(id, _)| *id).collect();
        let needed = life(view, foe);
        if through(offered) >= needed && through(&chosen) < needed {
            self.note(
                "attack: lethal on board not taken",
                view,
                u64::try_from(through(offered)).unwrap_or(0),
                vec![],
                "",
            );
        }
        for &(id, _) in declared {
            let Some(attacker) = view.object(id) else {
                continue;
            };
            let eaten = walls.iter().find(|wall| {
                can_block(wall, attacker)
                    && (hits(wall) >= staying(attacker) || has(wall, KeywordSet::DEATHTOUCH))
                    && power(attacker) < staying(wall)
                    && !has(attacker, KeywordSet::DEATHTOUCH)
                    && !has(attacker, KeywordSet::INDESTRUCTIBLE)
            });
            if let Some(wall) = eaten {
                let extra = format!("{} into {}", creature(attacker), creature(wall));
                self.note(
                    "attack: into a free kill",
                    view,
                    u64::from(attacker.mana_value),
                    vec![attacker.name.clone()],
                    &extra,
                );
            }
        }
    }

    fn block(
        &mut self,
        view: &PlayerView,
        options: &[BlockOption],
        declared: &[(ObjectId, ObjectId)],
    ) {
        let seat = self.seat;
        let attackers: Vec<&PublicObject> = view
            .combat
            .attackers
            .iter()
            .filter(|attacker| matches!(attacker.defending, Defender::Player(player) if player == seat))
            .filter_map(|attacker| view.object(attacker.creature))
            .collect();
        let hp = life(view, seat);
        let incoming: i32 = attackers.iter().map(|attacker| hits(attacker)).sum();
        let blocked = |id: ObjectId| declared.iter().any(|(_, attacker)| *attacker == id);
        let used = |id: ObjectId| declared.iter().any(|(blocker, _)| *blocker == id);
        let after: i32 = attackers
            .iter()
            .filter(|attacker| !blocked(attacker.id))
            .map(|attacker| hits(attacker))
            .sum();
        // The best any blocks could do, greedily, biggest attacker first.
        let mut sorted = attackers.clone();
        sorted.sort_by_key(|attacker| std::cmp::Reverse(hits(attacker)));
        let mut taken: Vec<ObjectId> = Vec::new();
        let mut least = incoming;
        for attacker in &sorted {
            if has(attacker, KeywordSet::MENACE) {
                continue;
            }
            let free = options.iter().find(|option| {
                !taken.contains(&option.blocker) && option.attackers.contains(&attacker.id)
            });
            if let Some(option) = free {
                taken.push(option.blocker);
                least -= hits(attacker);
            }
        }
        if after >= hp && least < hp {
            self.note(
                "block: died with a saving block on offer",
                view,
                u64::try_from(after).unwrap_or(0),
                vec![],
                &format!("incoming {incoming} blocked to {after}, could be {least}"),
            );
        }
        let (mut kills, mut safes, mut trades) = (Vec::new(), Vec::new(), Vec::new());
        for option in options.iter().filter(|option| !used(option.blocker)) {
            let Some(blocker) = view.object(option.blocker) else {
                continue;
            };
            for attacker in option
                .attackers
                .iter()
                .filter(|id| !blocked(**id))
                .filter_map(|id| view.object(*id))
                .filter(|attacker| !has(attacker, KeywordSet::MENACE) && hits(attacker) > 0)
            {
                let kill = (hits(blocker) >= staying(attacker)
                    || has(blocker, KeywordSet::DEATHTOUCH))
                    && !has(attacker, KeywordSet::INDESTRUCTIBLE);
                let safe = power(attacker) < staying(blocker)
                    && !has(attacker, KeywordSet::DEATHTOUCH);
                let pair = format!("{} on {}", creature(blocker), creature(attacker));
                match (kill, safe) {
                    (true, true) => kills.push(pair),
                    (false, true) => safes.push(pair),
                    (true, false) if blocker.mana_value <= attacker.mana_value => trades.push(pair),
                    _ => {}
                }
            }
        }
        let units = |pairs: &[String]| u64::try_from(pairs.len()).unwrap_or(0);
        if !kills.is_empty() {
            self.note(
                "block: free kill not taken",
                view,
                units(&kills),
                vec![],
                &kills.join("; "),
            );
        } else if !safes.is_empty() {
            self.note(
                "block: safe block not made",
                view,
                units(&safes),
                vec![],
                &safes.join("; "),
            );
        } else if !trades.is_empty() {
            self.note(
                "block: even trade not made",
                view,
                units(&trades),
                vec![],
                &trades.join("; "),
            );
        }
        for &(blocker, attacker) in declared {
            let (Some(blocker), Some(attacker)) = (view.object(blocker), view.object(attacker))
            else {
                continue;
            };
            if power(attacker) >= staying(blocker)
                && hits(blocker) < staying(attacker)
                && hp - incoming > 4
            {
                let extra = format!("{} chumps {}", creature(blocker), creature(attacker));
                self.note(
                    "block: chump at healthy life",
                    view,
                    u64::from(blocker.mana_value),
                    vec![blocker.name.clone()],
                    &extra,
                );
            }
        }
    }

    fn decide(&mut self, decision: &Decision<'_>) {
        if decision.source == "Forced" {
            return;
        }
        self.survey.decisions += 1;
        let seat = self.seat;
        let fallback = &decision.views[usize::from(seat.get())];
        let view = decision.asked.unwrap_or(fallback);
        let action = format!("{:?}", decision.action);
        let action = action.split([' ', '{', '(']).next().unwrap_or("?");
        *self
            .survey
            .kinds
            .entry(format!("{} / {action}", show::kind(decision.pending)))
            .or_default() += 1;
        let names = Names::of(decision.views);
        match (decision.pending, decision.action) {
            (Pending::Priority { legal, .. }, PlayerAction::PassPriority) => {
                self.pass(view, legal, &names);
            }
            (Pending::Priority { legal, .. }, action) if reading::makes_mana(view, action) => {
                if !reading::mana_has_a_sink(view, legal) {
                    self.wasted = true;
                    self.note("tap: nothing to spend it on", view, 1, vec![], "");
                }
            }
            (Pending::Priority { .. }, PlayerAction::CastSpell { card }) => {
                let name = names.name(*card);
                if !reading::worth_casting(view, *card) {
                    self.note("cast: reads as doing nothing now", view, 1, vec![name.clone()], "");
                }
                let twin = view.battlefield_of(seat).any(|object| {
                    object.name == name && object.supertypes.contains(SupertypeSet::LEGENDARY)
                });
                if twin {
                    self.note("cast: second copy of a legend", view, 1, vec![name], "");
                }
            }
            (
                Pending::ChooseAttackers { attackers, .. },
                PlayerAction::DeclareAttackers {
                    attackers: declared,
                },
            ) => self.attack(view, attackers, declared),
            (
                Pending::ChooseBlockers { blockers, .. },
                PlayerAction::DeclareBlockers {
                    blockers: declared,
                },
            ) => self.block(view, blockers, declared),
            (Pending::Mulligan { .. }, action) => {
                let lands = view
                    .hand
                    .iter()
                    .filter(|card| card.types.contains(TypeSet::LAND))
                    .count();
                let keep = matches!(action, PlayerAction::MulliganKeep);
                if view.hand.len() == 7 && keep && !(2..=5).contains(&lands) {
                    let units = u64::try_from(lands).unwrap_or(0);
                    self.note("mulligan: kept 0-1 or 6-7 lands", view, units, vec![], "");
                }
                if !keep && (2..=5).contains(&lands) {
                    let units = u64::try_from(lands).unwrap_or(0);
                    self.note("mulligan: took with 2-5 lands", view, units, vec![], "");
                }
            }
            (Pending::ChooseColor { .. }, PlayerAction::ChooseColor(color)) => {
                let color = baylee_color(*color);
                let spells: Vec<_> = view
                    .hand
                    .iter()
                    .filter(|card| !card.types.contains(TypeSet::LAND))
                    .collect();
                let wanted = spells.iter().any(|card| {
                    color.is_some_and(|color| card.colors.contains(color))
                });
                if !spells.is_empty() && !wanted {
                    self.note("colour: none of the hand's spells wants it", view, 1, vec![], "");
                }
            }
            _ => {}
        }
    }

    /// Whether the seat holds an instant that its own tapped lands lock out.
    fn their_turn(&mut self, view: &PlayerView, names: &Names) {
        let seat = self.seat;
        let lands = view
            .battlefield_of(seat)
            .filter(|object| object.types.contains(TypeSet::LAND))
            .count();
        let untapped = untapped_mana_production(view, seat);
        let locked: Vec<String> = view
            .hand
            .iter()
            .filter(|card| card.types.contains(TypeSet::INSTANT))
            .filter(|card| {
                card.mana_value > untapped
                    && usize::try_from(card.mana_value).is_ok_and(|value| value <= lands)
            })
            .map(|card| names.name(card.id))
            .collect();
        if !locked.is_empty() {
            let class = if self.wasted {
                "their turn: instant locked out, mana wasted on ours"
            } else {
                "their turn: instant locked out, mana was spent"
            };
            self.note(class, view, 1, locked, "");
        }
    }
}

fn baylee_color(color: ManaColor) -> Option<baylee_core::color::Color> {
    use baylee_core::color::Color;
    match color {
        ManaColor::White => Some(Color::White),
        ManaColor::Blue => Some(Color::Blue),
        ManaColor::Black => Some(Color::Black),
        ManaColor::Red => Some(Color::Red),
        ManaColor::Green => Some(Color::Green),
        ManaColor::Colorless => None,
    }
}

/// The classes of one game.
fn game(run: &Path, deal: u64) -> anyhow::Result<Survey> {
    let record = Game::load(run, deal)?;
    let seat = record.seat;
    let won = record.row["outcome"]["kind"].as_str() == Some("won")
        && record.row["outcome"]["seat"].as_u64() == Some(u64::from(seat.get()));
    let mut reader = Reader {
        seat,
        deal,
        lost: !won,
        wasted: false,
        survey: Survey {
            games: 1,
            lost_games: usize::from(!won),
            ..Survey::default()
        },
    };
    record.replay(false, |event| {
        match event {
            Event::Turn { views, .. } => {
                let view = &views[usize::from(seat.get())];
                reader.survey.turns += 1;
                if view.active == seat {
                    reader.survey.own_turns += 1;
                    reader.wasted = false;
                } else {
                    let names = Names::of(views);
                    reader.their_turn(view, &names);
                }
            }
            Event::Decision(decision) => reader.decide(&decision),
            _ => {}
        }
        Ok(())
    })?;
    Ok(reader.survey)
}

/// Counts the classes over the deals of `summary` that `which` selects, and
/// with `cases` writes every candidate there as a JSON line.
pub(super) fn count(
    run: &Path,
    summary: &Path,
    which: Which,
    threads: usize,
    cases: Option<&Path>,
) -> anyhow::Result<()> {
    let deals = losses::deals(summary, which)?;
    let next = AtomicUsize::new(0);
    let total = Mutex::new(Survey::default());
    let failures = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..threads.max(1) {
            scope.spawn(|| {
                while let Some(deal) = deals.get(next.fetch_add(1, Ordering::Relaxed)) {
                    match game(run, deal.number) {
                        Ok(survey) => total.lock().expect("survey lock").merge(survey),
                        Err(error) => failures
                            .lock()
                            .expect("failures lock")
                            .push(format!("deal {}: {error:#}", deal.number)),
                    }
                }
            });
        }
    });
    let failures = failures.into_inner().expect("failures lock");
    for failure in &failures {
        eprintln!("[gamelog4] {failure}");
    }
    println!("{which:?} deals of {}", summary.display());
    let mut total = total.into_inner().expect("survey lock");
    total.print();
    if let Some(path) = cases {
        total.write_cases(path)?;
    }
    anyhow::ensure!(
        failures.is_empty(),
        "{} games could not be replayed",
        failures.len()
    );
    Ok(())
}
