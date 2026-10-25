//! Decks built from the working pool, so the trained AI sees every working card
//! in coherent, synergistic, curve-balanced decks with pip-accurate mana bases.
//!
//! A generated deck is a training and evaluation artifact: it is built to a
//! shape ([`Shape`]: Constructed, Highlander, Commander) and an archetype
//! ([`Archetype`]: land share, mana curve, creature share, and functional quotas
//! for ramp, interaction, and card advantage), from cards that work, and it is
//! checked by the same predicate the gateway uses
//! ([`baylee_cards::formats::check`]). A deterministic slice of the working
//! cards is held out of every training deck ([`held_out`]), so a net's play on
//! cards it never saw can be measured.
//!
//! Submodules provide reusable building blocks across the workspace:
//! - [`profile`]: semantic card profiles (`CardProfile`, role bitmasks, mana pip counts)
//! - [`synergy`]: pairwise enabler/payoff synergy graph and $O(1)$ [`DeckThemeAccumulator`](synergy::DeckThemeAccumulator)
//! - [`manabase`]: Frank Karsten hypergeometric mana-source solver
//! - [`strategy`]: multi-label deck archetype/tag classifier, functional card replacement, and multiset Jaccard similarity
//! - [`coverage`]: $k$-Multi-Set Cover corpus builder ([`build_coverage_corpus`]) ensuring every working card appears in at least $k$ decks

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use baylee_cards::decks::{DeckCard, LoadedDeck, leader_of};
use baylee_cards::formats::{self, Shape};
use baylee_core::color::{Color, ColorPair, ColorSet};
use baylee_core::generated::index::CRYPTIC_SPIRES;
use baylee_core::ids::CardIndex;
use baylee_core::types::{SupertypeSet, TypeSet};

use crate::working::Working;

pub mod coverage;
pub mod manabase;
pub mod profile;
pub mod strategy;
pub mod synergy;

pub use coverage::{
    CoverageConfig, CoverageReport, DeckBuildRequest, build_coverage_corpus,
    build_coverage_corpus_scoped, build_synergistic,
};
pub use profile::CardProfile;

/// The five colours in their bit order.
pub(crate) const COLORS: [Color; 5] = [
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
    #[must_use]
    pub const fn land_share(self) -> f64 {
        match self {
            Self::Aggro => 0.38,
            Self::Midrange => 0.40,
            Self::Control => 0.43,
            Self::Ramp => 0.42,
        }
    }

    /// Weights of mana value 0, 1, …, 7+ among the spells.
    #[must_use]
    pub const fn curve(self) -> [f64; 8] {
        match self {
            Self::Aggro => [0.04, 0.30, 0.30, 0.20, 0.10, 0.04, 0.02, 0.0],
            Self::Midrange => [0.03, 0.14, 0.24, 0.24, 0.17, 0.10, 0.05, 0.03],
            Self::Control => [0.03, 0.12, 0.22, 0.22, 0.17, 0.12, 0.08, 0.04],
            Self::Ramp => [0.03, 0.16, 0.20, 0.14, 0.12, 0.12, 0.12, 0.11],
        }
    }

    /// The share of the spells that are creatures.
    #[must_use]
    pub const fn creature_share(self) -> f64 {
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
#[derive(Clone, Debug)]
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
    /// Semantic deckbuilding profile (`CardDsl` roles, mana pips, subtypes, and effect vector).
    pub profile: CardProfile,
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
                let profile = CardProfile::from_def(def);
                Entry {
                    card: def.index,
                    mana_value: face.mana_cost.cmc(),
                    identity: def.color_identity,
                    land: face.types.contains(TypeSet::LAND),
                    basic: face.types.contains(TypeSet::LAND)
                        && face.supertypes.contains(SupertypeSet::BASIC),
                    creature: face.types.contains(TypeSet::CREATURE),
                    leader: leader_of(def.index).is_some_and(|l| l.eligible),
                    profile,
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
            main: self.main.iter().map(|c| self.copy(*c)).collect(),
            sideboard: Vec::new(),
            commanders: self.leaders.iter().map(|c| DeckCard::plain(*c)).collect(),
        }
    }

    // Keep the in-memory preset and exported deck's construction choices equal.
    fn copy(&self, index: CardIndex) -> DeckCard {
        let mut card = DeckCard::plain(index);
        if index == CRYPTIC_SPIRES {
            let mut colors = COLORS.into_iter().filter(|c| self.colors.contains(*c));
            let first = colors.next().unwrap_or(Color::White);
            let second = colors
                .next()
                .unwrap_or_else(|| COLORS.into_iter().find(|c| *c != first).unwrap());
            card.choices.circled_colors = Some(ColorPair::new(first, second));
        }
        card
    }

    /// The deck in the house decks' text format: `[deck:Name]`, one
    /// `count Name` row per card, `[commander]` and the leaders' names.
    #[must_use]
    pub fn text(&self) -> String {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for c in &self.main {
            let mut name = baylee_cards::by_index(*c)
                .map_or("?", |d| d.name())
                .to_owned();
            if let Some(pair) = self.copy(*c).choices.circled_colors {
                let _ = write!(
                    name,
                    " circles={}{}",
                    pair.first().symbol(),
                    pair.second().symbol()
                );
            }
            *counts.entry(name).or_default() += 1;
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

pub(crate) fn colour_letters(colors: ColorSet) -> String {
    ["W", "U", "B", "R", "G"]
        .iter()
        .enumerate()
        .filter(|(i, _)| colors.bits() >> i & 1 == 1)
        .map(|(_, l)| *l)
        .collect()
}

/// Builds one synergistic, curve-balanced deck of `shape` and `archetype` from
/// `pool`, with `seed` choosing everything left to chance.
///
/// # Errors
/// When the pool cannot fill the deck (too few cards in its colours), or
/// the result is not of its shape.
pub fn generate(
    pool: &Pool,
    shape: Shape,
    archetype: Archetype,
    seed: u64,
) -> anyhow::Result<Generated> {
    build_synergistic(pool, &DeckBuildRequest::new(shape, archetype, seed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deckgen::manabase::karsten_sources_for_spell;
    use crate::deckgen::profile::{ROLE_SAC_OUTLET, ROLE_TOKENS};
    use crate::deckgen::synergy::pair_synergy;

    fn pool() -> &'static (Working, Pool) {
        static CACHE: std::sync::OnceLock<(Working, Pool)> = std::sync::OnceLock::new();
        CACHE.get_or_init(|| {
            let working = Working::scan(&crate::working::repo_root()).unwrap();
            let held = held_out(&working, 20);
            let pool = Pool::new(&working, &held);
            (working, pool)
        })
    }

    /// Every shape and archetype builds decks of its shape, from working
    /// cards that are not held out, and the same seed builds the same deck.
    #[test]
    fn generated_decks_are_decks_of_working_cards() {
        let (working, pool) = pool();
        let held = held_out(working, 20);
        let mut built = 0;
        for shape in [Shape::Constructed, Shape::Highlander, Shape::Commander] {
            for archetype in Archetype::ALL {
                for seed in 0..6 {
                    let Ok(deck) = generate(pool, shape, archetype, seed) else {
                        continue;
                    };
                    built += 1;
                    assert!(formats::check(shape, &deck.main, &deck.leaders).is_empty());
                    for card in deck.distinct() {
                        assert!(working.check(card).is_ok(), "{card} does not work");
                        assert!(!held.contains(&card), "{card} is held out");
                    }
                    let parsed = crate::housedeck::HouseDeck::parse("generated", &deck.text())
                        .expect("every generated deck retains valid construction choices");
                    let mut parsed_main = parsed.deck.main;
                    parsed_main.sort_by_key(|card| card.index);
                    assert_eq!(parsed_main, deck.loaded().main);
                    let again = generate(pool, shape, archetype, seed).unwrap();
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
            .filter_map(|s| generate(pool, Shape::Constructed, Archetype::Midrange, s).ok())
            .map(|d| d.main.len())
            .collect();
        assert!(sizes.contains(&60));
        assert!(sizes.iter().any(|n| *n > 60), "{sizes:?}");
    }

    #[test]
    #[allow(clippy::cast_precision_loss)]
    fn the_held_out_slice_is_a_twentieth_and_stable() {
        let (working, _) = pool();
        let held = held_out(working, 20);
        let share = held.len() as f64 / working.cards.len() as f64;
        assert!((0.03..0.08).contains(&share), "{share}");
        assert_eq!(held, held_out(working, 20));
    }

    #[test]
    fn spires_choices_survive_loaded_and_text_decks() {
        for colors in [ColorSet::of(Color::Red), ColorSet::ALL] {
            let deck = Generated {
                name: "Spires".into(),
                shape: Shape::Constructed,
                archetype: Archetype::Midrange,
                colors,
                main: vec![CRYPTIC_SPIRES; 4],
                leaders: Vec::new(),
            };
            let loaded = deck.loaded();
            let parsed = crate::housedeck::HouseDeck::parse("spires", &deck.text()).unwrap();
            assert_eq!(parsed.deck.main, loaded.main);
            for card in loaded.main {
                baylee_cards::print_rules::validate_choices(card.index, card.choices).unwrap();
                assert_eq!(card.choices.colors().len(), 2);
                assert!(!card.choices.colors().intersection(colors).is_empty());
            }
        }
    }

    #[test]
    fn the_text_is_a_house_deck_file() {
        let (_, pool) = pool();
        let deck = generate(pool, Shape::Commander, Archetype::Midrange, 3).unwrap();
        let parsed = crate::housedeck::HouseDeck::parse("gen", &deck.text()).unwrap();
        let mut parsed_main = parsed.deck.main;
        parsed_main.sort_by_key(|card| card.index);
        assert_eq!(parsed_main, deck.loaded().main);
        assert_eq!(parsed.deck.commanders.len(), 1);
    }

    #[test]
    fn karsten_mana_math_scales_with_pips_and_deck_size() {
        let single_pip_5drop = karsten_sources_for_spell(60, 5, 1);
        let double_pip_3drop = karsten_sources_for_spell(60, 3, 2);
        let triple_pip_3drop = karsten_sources_for_spell(60, 3, 3);
        assert!(
            double_pip_3drop > single_pip_5drop,
            "1WW ({double_pip_3drop}) must demand more sources than 4W ({single_pip_5drop})"
        );
        assert!(
            triple_pip_3drop > double_pip_3drop,
            "WWW ({triple_pip_3drop}) must demand more sources than 1WW ({double_pip_3drop})"
        );
        let commander_double_pip = karsten_sources_for_spell(100, 3, 2);
        assert!(
            commander_double_pip > double_pip_3drop,
            "100-card deck ({commander_double_pip}) needs more sources than 60-card ({double_pip_3drop})"
        );
    }

    #[test]
    fn synergy_couples_enablers_and_payoffs() {
        let (_, pool) = pool();
        let token_card = pool
            .entries
            .iter()
            .find(|e| (e.profile.roles & ROLE_TOKENS) != 0)
            .expect("pool has a token generator");
        let sac_card = pool
            .entries
            .iter()
            .find(|e| (e.profile.roles & ROLE_SAC_OUTLET) != 0)
            .expect("pool has a sacrifice outlet");
        let vanilla_card = pool
            .entries
            .iter()
            .find(|e| !e.land && e.profile.roles == 0 && e.profile.subtypes.is_empty())
            .unwrap_or(&pool.entries[0]);

        let syn_coupled = pair_synergy(&token_card.profile, &sac_card.profile);
        let syn_vanilla = pair_synergy(&token_card.profile, &vanilla_card.profile);
        assert!(
            syn_coupled > syn_vanilla + 1.5,
            "Token+Sacrifice synergy ({syn_coupled}) should exceed vanilla ({syn_vanilla})"
        );
    }

    #[test]
    fn coverage_corpus_covers_every_subset_card_at_least_three_times() {
        let (_, full_pool) = pool();
        // Take a representative 220-card subset including basic lands, leaders, and spells
        let mut entries: Vec<Entry> = full_pool
            .entries
            .iter()
            .filter(|e| e.basic || e.leader)
            .take(30)
            .cloned()
            .collect();
        for e in &full_pool.entries {
            if entries.len() >= 220 {
                break;
            }
            if !entries.iter().any(|x| x.card == e.card) {
                entries.push(e.clone());
            }
        }
        let sub_pool = Pool { entries };
        let cfg = CoverageConfig {
            min_coverage: 3,
            max_jaccard: 0.85,
            base_seed: 42,
            max_new_decks: 120,
        };
        let (decks, report) = build_coverage_corpus(&sub_pool, &[], &cfg).unwrap();
        assert_eq!(report.under_covered_after, 0, "{report:?}");
        assert!(report.min_coverage_after >= 3, "{report:?}");
        assert!(!decks.is_empty());
        for d in &decks {
            assert!(formats::check(d.shape, &d.main, &d.leaders).is_empty());
        }
    }
}
