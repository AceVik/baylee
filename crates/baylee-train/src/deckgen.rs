//! Decks built from the working pool, so the trained AI sees more than two.
//!
//! A generated deck is a training artifact, not a house deck: it lives in a
//! run's manifest, never in `data/decks/` or the database. It is built to a
//! shape ([`Shape`]: at least 60 cards with four copies, sometimes more
//! cards; Highlander; Commander) and an archetype ([`Archetype`]: how many
//! lands, which curve, how many creatures), from cards that work, and it is
//! checked by the same predicate the gateway will use
//! ([`baylee_cards::formats::check`]). A deterministic slice of the working
//! cards is held out of every deck ([`held_out`]), so a net's play on cards
//! it never saw can be measured.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use anyhow::{Context as _, bail};
use baylee_cards::decks::{DeckCard, LoadedDeck, basic_lands, leader_of};
use baylee_cards::formats::{self, Shape};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::types::{SupertypeSet, TypeSet};

use crate::working::Working;

/// The five colours in their bit order.
const COLORS: [Color; 5] = [
    Color::White,
    Color::Blue,
    Color::Black,
    Color::Red,
    Color::Green,
];

/// How a deck means to win, as land count, curve and creature share.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Archetype {
    /// Cheap creatures, few lands.
    Aggro,
    /// A bit of everything, the curve in the middle.
    Midrange,
    /// Few creatures, answers, more lands, a higher curve.
    Control,
    /// Mana first, then big things.
    Ramp,
}

impl Archetype {
    /// Every archetype.
    pub const ALL: [Self; 4] = [Self::Aggro, Self::Midrange, Self::Control, Self::Ramp];

    /// The share of a deck that is land.
    const fn land_share(self) -> f64 {
        match self {
            Self::Aggro => 0.38,
            Self::Midrange => 0.40,
            Self::Control => 0.43,
            Self::Ramp => 0.42,
        }
    }

    /// Weights of mana value 0, 1, …, 7+ among the spells.
    const fn curve(self) -> [f64; 8] {
        match self {
            Self::Aggro => [0.04, 0.30, 0.30, 0.20, 0.10, 0.04, 0.02, 0.0],
            Self::Midrange => [0.03, 0.14, 0.24, 0.24, 0.17, 0.10, 0.05, 0.03],
            Self::Control => [0.03, 0.12, 0.22, 0.22, 0.17, 0.12, 0.08, 0.04],
            Self::Ramp => [0.03, 0.16, 0.20, 0.14, 0.12, 0.12, 0.12, 0.11],
        }
    }

    /// The share of the spells that are creatures.
    const fn creature_share(self) -> f64 {
        match self {
            Self::Aggro => 0.70,
            Self::Midrange => 0.50,
            Self::Control => 0.25,
            Self::Ramp => 0.45,
        }
    }

    /// Its name in manifests.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Aggro => "aggro",
            Self::Midrange => "midrange",
            Self::Control => "control",
            Self::Ramp => "ramp",
        }
    }
}

/// A shape's name in manifests.
#[must_use]
pub const fn shape_name(shape: Shape) -> &'static str {
    match shape {
        Shape::Constructed => "constructed",
        Shape::Highlander => "highlander",
        Shape::Commander => "commander",
    }
}

/// `SplitMix64`: the generator's only source of chance, so a seed is a deck.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    /// A generator for `seed`.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// The next 64 bits.
    pub const fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number in `0..n` (`n > 0`).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    /// A number in `[0, 1)`.
    #[allow(clippy::cast_precision_loss)]
    pub fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1_u64 << 53) as f64
    }

    /// An index drawn by `weights` (not all zero).
    pub fn weighted(&mut self, weights: &[f64]) -> usize {
        let total: f64 = weights.iter().sum();
        let mut x = self.unit() * total;
        for (i, w) in weights.iter().enumerate() {
            if x < *w {
                return i;
            }
            x -= w;
        }
        weights.len() - 1
    }
}

/// What the generator knows about one card.
#[derive(Clone, Copy, Debug)]
#[allow(clippy::struct_excessive_bools)] // facts about a card, each a yes or no
pub struct Entry {
    /// The card.
    pub card: CardIndex,
    /// Its mana value.
    pub mana_value: u32,
    /// Its colour identity.
    pub identity: ColorSet,
    /// A land (front face).
    pub land: bool,
    /// A basic land.
    pub basic: bool,
    /// A creature (front face).
    pub creature: bool,
    /// It may lead a Commander deck.
    pub leader: bool,
}

/// The cards decks are built from.
#[derive(Clone, Debug, Default)]
pub struct Pool {
    /// Every usable card.
    pub entries: Vec<Entry>,
}

/// FNV-1a over a card index: the held-out slice is the same on every
/// machine and in every build that keeps the card.
const fn fnv(card: CardIndex) -> u64 {
    let bytes = card.get().to_le_bytes();
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut i = 0;
    while i < 4 {
        h ^= bytes[i] as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
        i += 1;
    }
    h
}

/// The working cards held out of every training deck: one in `every`,
/// basics never. What a net does with them is its play on unseen cards.
#[must_use]
pub fn held_out(working: &Working, every: u64) -> BTreeSet<CardIndex> {
    working
        .cards
        .iter()
        .filter(|c| !formats::is_basic_land(**c) && fnv(**c).is_multiple_of(every.max(1)))
        .copied()
        .collect()
}

impl Pool {
    /// The working cards, less `excluded`.
    #[must_use]
    pub fn new(working: &Working, excluded: &BTreeSet<CardIndex>) -> Self {
        let entries = working
            .cards
            .iter()
            .filter(|c| !excluded.contains(c))
            .filter_map(|c| baylee_cards::by_index(*c))
            .map(|def| {
                let face = &def.faces[0];
                Entry {
                    card: def.index,
                    mana_value: face.mana_cost.cmc(),
                    identity: def.color_identity,
                    land: face.types.contains(TypeSet::LAND),
                    basic: face.types.contains(TypeSet::LAND)
                        && face.supertypes.contains(SupertypeSet::BASIC),
                    creature: face.types.contains(TypeSet::CREATURE),
                    leader: leader_of(def.index).is_some_and(|l| l.eligible),
                }
            })
            .collect();
        Self { entries }
    }
}

/// A generated deck.
#[derive(Clone, Debug)]
pub struct Generated {
    /// Its name: shape, archetype, colours and seed.
    pub name: String,
    /// Its shape.
    pub shape: Shape,
    /// Its archetype.
    pub archetype: Archetype,
    /// Its colours.
    pub colors: ColorSet,
    /// The library (leaders not in it).
    pub main: Vec<CardIndex>,
    /// Its leaders, for Commander.
    pub leaders: Vec<CardIndex>,
}

impl Generated {
    /// The deck as the engine is dealt it.
    #[must_use]
    pub fn loaded(&self) -> LoadedDeck {
        LoadedDeck {
            name: self.name.clone(),
            main: self.main.iter().map(|c| DeckCard::plain(*c)).collect(),
            sideboard: Vec::new(),
            commanders: self.leaders.iter().map(|c| DeckCard::plain(*c)).collect(),
        }
    }

    /// The deck in the house decks' text format: `[deck:Name]`, one
    /// `count Name` row per card, `[commander]` and the leaders' names.
    ///
    /// TODO: `baylee_deckio::export` writes this format once it reaches
    /// main; call it then instead.
    #[must_use]
    pub fn text(&self) -> String {
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for c in &self.main {
            *counts
                .entry(baylee_cards::by_index(*c).map_or("?", |d| d.name()))
                .or_default() += 1;
        }
        let mut out = format!("[deck:{}]\n", self.name);
        for (name, n) in counts {
            let _ = writeln!(out, "{n} {name}");
        }
        if !self.leaders.is_empty() {
            out.push_str("[commander]\n");
            for l in &self.leaders {
                out.push_str(baylee_cards::by_index(*l).map_or("?", |d| d.name()));
                out.push('\n');
            }
        }
        out
    }

    /// Its cards, each once.
    #[must_use]
    pub fn distinct(&self) -> BTreeSet<CardIndex> {
        self.main.iter().chain(&self.leaders).copied().collect()
    }
}

fn colour_letters(colors: ColorSet) -> String {
    ["W", "U", "B", "R", "G"]
        .iter()
        .enumerate()
        .filter(|(i, _)| colors.bits() >> i & 1 == 1)
        .map(|(_, l)| *l)
        .collect()
}

/// Builds one deck of `shape` and `archetype` from `pool`, `seed` choosing
/// everything left to chance.
///
/// # Errors
/// When the pool cannot fill the deck (too few cards in its colours), or
/// the result is not of its shape (a bug here, and said so).
#[allow(
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)] // one recipe; small counts
pub fn generate(
    pool: &Pool,
    shape: Shape,
    archetype: Archetype,
    seed: u64,
) -> anyhow::Result<Generated> {
    let mut rng = Rng::new(seed ^ 0x5eed_dec5);
    let basics = basic_lands();
    // Colours, and for Commander the leader who sets them.
    let mut leaders = Vec::new();
    let colors = if shape == Shape::Commander {
        let candidates: Vec<&Entry> = pool
            .entries
            .iter()
            .filter(|e| e.leader && !e.identity.is_empty())
            .collect();
        if candidates.is_empty() {
            bail!("no working card may lead a deck");
        }
        let leader = candidates[rng.below(candidates.len())];
        leaders.push(leader.card);
        leader.identity
    } else {
        let count = [1, 2, 2, 2, 3][rng.below(5)];
        let mut chosen = ColorSet::EMPTY;
        let all = [0_u8, 1, 2, 3, 4];
        while chosen.len() < count {
            let weights: Vec<f64> = all
                .iter()
                .map(|i| {
                    if chosen.bits() >> i & 1 == 1 {
                        0.0
                    } else {
                        pool.entries
                            .iter()
                            .filter(|e| !e.land && e.identity.bits() >> i & 1 == 1)
                            .count() as f64
                    }
                })
                .collect();
            let pick = all[rng.weighted(&weights)];
            chosen = chosen.union(ColorSet::of(COLORS[usize::from(pick)]));
        }
        chosen
    };

    let size = match shape {
        // At least 60, and a quarter of the decks a little more.
        Shape::Constructed => {
            if rng.unit() < 0.25 {
                61 + rng.below(15)
            } else {
                60
            }
        }
        Shape::Highlander | Shape::Commander => formats::SINGLETON_SIZE,
    };
    let library = size - leaders.len();
    let lands = ((size as f64) * archetype.land_share()).round() as usize;
    let spells = library - lands;
    let singleton = shape != Shape::Constructed;

    // Spells in the deck's colours (colourless ones too), by mana value.
    let fits = |e: &&Entry| e.identity.difference(colors).is_empty() && !leaders.contains(&e.card);
    let spell_pool: Vec<&Entry> = pool
        .entries
        .iter()
        .filter(|e| !e.land)
        .filter(fits)
        .collect();
    if spell_pool.len() < 12 {
        bail!(
            "only {} working spells in {}",
            spell_pool.len(),
            colour_letters(colors)
        );
    }
    let curve = archetype.curve();
    let mut main: Vec<CardIndex> = Vec::with_capacity(library);
    let mut taken: BTreeSet<CardIndex> = BTreeSet::new();
    let mut guard = 0;
    while main.len() < spells {
        guard += 1;
        if guard > 20 * library {
            bail!(
                "the pool in {} cannot fill {spells} spells",
                colour_letters(colors)
            );
        }
        let creature = rng.unit() < archetype.creature_share();
        let bucket = rng.weighted(&curve);
        let candidates: Vec<&&Entry> = spell_pool
            .iter()
            .filter(|e| e.creature == creature)
            .filter(|e| (e.mana_value as usize).min(7) == bucket)
            .filter(|e| !taken.contains(&e.card))
            .collect();
        if candidates.is_empty() {
            continue;
        }
        let card = candidates[rng.below(candidates.len())].card;
        taken.insert(card);
        let copies = if singleton {
            1
        } else {
            [4, 4, 3, 2][rng.below(4)]
        };
        for _ in 0..copies.min(spells - main.len()) {
            main.push(card);
        }
    }

    // Lands: some nonbasics in the colours, the rest basics by colour weight.
    let nonbasics: Vec<&Entry> = pool
        .entries
        .iter()
        .filter(|e| e.land && !e.basic)
        .filter(fits)
        .collect();
    let mut land_cards: Vec<CardIndex> = Vec::with_capacity(lands);
    let nonbasic_target = (lands as f64 * if colors.len() > 1 { 0.4 } else { 0.2 }) as usize;
    let mut tries = 0;
    while land_cards.len() < nonbasic_target && !nonbasics.is_empty() && tries < 200 {
        tries += 1;
        let e = nonbasics[rng.below(nonbasics.len())];
        if taken.contains(&e.card) {
            continue;
        }
        taken.insert(e.card);
        let copies = if singleton { 1 } else { 1 + rng.below(4) };
        for _ in 0..copies.min(nonbasic_target - land_cards.len()) {
            land_cards.push(e.card);
        }
    }
    let weight: Vec<f64> = (0..5_u8)
        .map(|i| {
            if colors.bits() >> i & 1 == 1 {
                1.0 + main
                    .iter()
                    .filter_map(|c| baylee_cards::by_index(*c))
                    .filter(|d| d.color_identity.bits() >> i & 1 == 1)
                    .count() as f64
            } else {
                0.0
            }
        })
        .collect();
    while land_cards.len() < lands {
        let colour = if weight.iter().sum::<f64>() > 0.0 {
            rng.weighted(&weight)
        } else {
            rng.below(5)
        };
        let basic = basics[colour].context("the five basic lands are in the pool")?;
        land_cards.push(basic);
    }
    main.extend(land_cards);
    main.sort();

    let generated = Generated {
        name: format!(
            "{} {} {} #{seed}",
            shape_name(shape),
            archetype.name(),
            colour_letters(colors)
        ),
        shape,
        archetype,
        colors,
        main,
        leaders,
    };
    let problems = formats::check(shape, &generated.main, &generated.leaders);
    if !problems.is_empty() {
        bail!(
            "a generated {} deck is not one: {problems:?}",
            shape_name(shape)
        );
    }
    Ok(generated)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pool() -> (Working, Pool) {
        let working = Working::scan(&crate::working::repo_root()).unwrap();
        let held = held_out(&working, 20);
        let pool = Pool::new(&working, &held);
        (working, pool)
    }

    /// Every shape and archetype builds decks of its shape, from working
    /// cards that are not held out, and the same seed builds the same deck.
    #[test]
    fn generated_decks_are_decks_of_working_cards() {
        let (working, pool) = pool();
        let held = held_out(&working, 20);
        let mut built = 0;
        for shape in [Shape::Constructed, Shape::Highlander, Shape::Commander] {
            for archetype in Archetype::ALL {
                for seed in 0..6 {
                    let Ok(deck) = generate(&pool, shape, archetype, seed) else {
                        continue;
                    };
                    built += 1;
                    assert!(formats::check(shape, &deck.main, &deck.leaders).is_empty());
                    for card in deck.distinct() {
                        assert!(working.check(card).is_ok(), "{card} does not work");
                        assert!(!held.contains(&card), "{card} is held out");
                    }
                    let again = generate(&pool, shape, archetype, seed).unwrap();
                    assert_eq!(deck.main, again.main);
                }
            }
        }
        assert!(built >= 60, "only {built} of 72 decks built");
    }

    #[test]
    fn some_constructed_decks_hold_more_than_sixty() {
        let (_, pool) = pool();
        let sizes: BTreeSet<usize> = (0..40)
            .filter_map(|s| generate(&pool, Shape::Constructed, Archetype::Midrange, s).ok())
            .map(|d| d.main.len())
            .collect();
        assert!(sizes.contains(&60));
        assert!(sizes.iter().any(|n| *n > 60), "{sizes:?}");
    }

    #[test]
    #[allow(clippy::cast_precision_loss)]
    fn the_held_out_slice_is_a_twentieth_and_stable() {
        let (working, _) = pool();
        let held = held_out(&working, 20);
        let share = held.len() as f64 / working.cards.len() as f64;
        assert!((0.03..0.08).contains(&share), "{share}");
        assert_eq!(held, held_out(&working, 20));
    }

    #[test]
    fn the_text_is_a_house_deck_file() {
        let (_, pool) = pool();
        let deck = generate(&pool, Shape::Commander, Archetype::Midrange, 3).unwrap();
        let parsed = crate::housedeck::HouseDeck::parse("gen", &deck.text()).unwrap();
        assert_eq!(parsed.deck.main.len(), deck.main.len());
        assert_eq!(parsed.deck.commanders.len(), 1);
    }
}
