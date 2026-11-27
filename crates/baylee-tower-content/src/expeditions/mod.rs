//! Curated build-arounds, assembled once by the shared deterministic deck builder.
//! Publication freezes the resulting cards: a registry update never edits a run.
use super::{Card, Catalog, Collection, Content, Error, Formation, Stack, metadata, named};
use baylee_cards::deckgen::{Archetype, Pool, build_commander_run_with_core};
use baylee_core::ids::CardIndex;
use std::collections::{BTreeMap, BTreeSet};

mod frozen;
mod lands;
mod theme;

// Lower commander costs and several distinct plans per colour pair avoid a pool
// made solely of expensive finishers. Seeds are explicit and append-only; the
// third value is a measured calibration variant of the builder seed (0: none),
// chosen with the `tower_lab` example's `variants` (docs/tower-operations.md §"Calibration").
const LEADERS: [(&str, Archetype, u16); 50] = [
    ("Emmara, Soul of the Accord", Archetype::Midrange, 2),
    ("Neva, Stalked by Nightmares", Archetype::Midrange, 0),
    ("Talrand, Sky Summoner", Archetype::Midrange, 2),
    ("Slimefoot, the Stowaway", Archetype::Midrange, 0),
    ("Pia and Kiran Nalaar", Archetype::Aggro, 1),
    ("Radha, Heir to Keld", Archetype::Ramp, 3),
    ("Rishkar, Peema Renegade", Archetype::Ramp, 1),
    ("Sram, Senior Edificer", Archetype::Aggro, 0),
    ("Hapatra, Vizier of Poisons", Archetype::Aggro, 1),
    ("Tatyova, Benthic Druid", Archetype::Ramp, 0),
    ("Judith, the Scourge Diva", Archetype::Midrange, 2),
    ("Kykar, Wind's Fury", Archetype::Aggro, 3),
    ("Alela, Artful Provocateur", Archetype::Midrange, 0),
    ("Rin and Seri, Inseparable", Archetype::Control, 0),
    ("Loran of the Third Path", Archetype::Aggro, 0),
    ("Glissa, the Traitor", Archetype::Midrange, 0),
    ("Thrun, the Last Troll", Archetype::Control, 2),
    ("Derevi, Empyrial Tactician", Archetype::Aggro, 0),
    ("Titania, Protector of Argoth", Archetype::Ramp, 0),
    ("Eladamri, Lord of Leaves", Archetype::Ramp, 0),
    ("Kambal, Consul of Allocation", Archetype::Control, 0),
    ("Padeem, Consul of Innovation", Archetype::Aggro, 3),
    ("Yahenni, Undying Partisan", Archetype::Aggro, 0),
    ("Samut, Voice of Dissent", Archetype::Aggro, 0),
    ("Kess, Dissident Mage", Archetype::Midrange, 0),
    ("Vona, Butcher of Magan", Archetype::Midrange, 0),
    ("Selenia, Dark Angel", Archetype::Midrange, 0),
    ("Garna, the Bloodflame", Archetype::Midrange, 0),
    ("Soraya the Falconer", Archetype::Midrange, 0),
    ("Muldrotha, the Gravetide", Archetype::Ramp, 1),
    ("Pashalik Mons", Archetype::Aggro, 0),
    ("Ayara, First of Locthwain", Archetype::Control, 3),
    ("Emry, Lurker of the Loch", Archetype::Aggro, 2),
    ("Linden, the Steadfast Queen", Archetype::Aggro, 1),
    ("Arasta of the Endless Web", Archetype::Midrange, 2),
    ("Sab-Sunen, Luxa Embodied", Archetype::Midrange, 0),
    ("Chevill, Bane of Monsters", Archetype::Ramp, 0),
    ("Kaervek, the Spiteful", Archetype::Midrange, 0),
    ("Emiel the Blessed", Archetype::Midrange, 0),
    ("Ashaya, Soul of the Wild", Archetype::Control, 2),
    ("Ghen, Arcanum Weaver", Archetype::Ramp, 1),
    ("Birgi, God of Storytelling", Archetype::Aggro, 0),
    ("Maja, Bretagard Protector", Archetype::Ramp, 0),
    ("Galazeth Prismari", Archetype::Control, 0),
    ("Grist, the Hunger Tide", Archetype::Midrange, 0),
    ("Sythis, Harvest's Hand", Archetype::Midrange, 1),
    ("Saryth, the Viper's Fang", Archetype::Midrange, 0),
    ("Mahadi, Emporium Master", Archetype::Midrange, 0),
    ("Ertai Resurrected", Archetype::Aggro, 2),
    ("Tura Kennerüd, Skyknight", Archetype::Ramp, 3),
];

// Keep unusually explosive openings out of a short starter deck. These are
// construction exclusions, not altered Magic rules or a competitive banlist.
const EXCLUDED: &[&str] = &[
    "Cryptic Spires",
    "Black Lotus",
    "Mox Pearl",
    "Mox Sapphire",
    "Mox Jet",
    "Mox Ruby",
    "Mox Emerald",
    "Mox Diamond",
    "Chrome Mox",
    "Mana Crypt",
    "Mana Vault",
    "Sol Ring",
    "Ancient Tomb",
    "City of Traitors",
    "Jeweled Lotus",
    "Lion's Eye Diamond",
    "Lotus Petal",
    "Time Walk",
    "Ancestral Recall",
    "Timetwister",
    "Channel",
    "Tinker",
    "Demonic Consultation",
    "Thassa's Oracle",
];

/// The whole admitted pool, with the reviewed utility lands only.
pub(crate) fn prepared_pool() -> Pool {
    let excluded: BTreeSet<_> = EXCLUDED
        .iter()
        .filter_map(|name| baylee_cards::decks::by_name(name))
        .collect();
    let mut pool = Pool::all_implemented(&excluded);
    lands::prepare(&mut pool);
    pool
}

/// How one starter is built; a run rebuilds its deck the same way.
pub(crate) struct Recipe {
    /// The starter's commander.
    pub(crate) commander: CardIndex,
    /// The builder's archetype for it.
    pub(crate) archetype: Archetype,
    /// Its explicit, append-only seed.
    pub(crate) seed: u64,
    /// Its mandatory on-theme spells.
    pub(crate) core: Vec<CardIndex>,
}

impl Recipe {
    /// The 39 library cards the shared builder makes from `pool`: the whole
    /// admitted pool for a starter, a run's owned cards for a refit.
    pub(crate) fn build(&self, pool: &Pool) -> Result<Vec<CardIndex>, Error> {
        let mut generated = build_commander_run_with_core(
            pool,
            self.commander,
            self.archetype,
            self.seed,
            &self.core,
        )?;
        lands::retain_fetch_targets(
            &mut generated.main,
            baylee_cards::by_index(self.commander)
                .ok_or(Error::Metadata)?
                .color_identity,
        )?;
        Ok(generated.main)
    }
}

/// Every starter's recipe, in publication order.
pub(crate) fn recipes(pool: &Pool) -> Result<Vec<Recipe>, Error> {
    LEADERS
        .iter()
        .enumerate()
        .map(|(index, &(name, archetype, salt))| recipe(pool, index, name, archetype, salt))
        .collect()
}

fn recipe(
    pool: &Pool,
    index: usize,
    name: &str,
    archetype: Archetype,
    salt: u16,
) -> Result<Recipe, Error> {
    let commander = baylee_cards::decks::by_name(name).ok_or(Error::Metadata)?;
    metadata(commander).ok_or(Error::Metadata)?;
    let seed = 0x746f_7765_7200_0000
        | u64::try_from(index).map_err(|_| Error::Formation)?
        | u64::from(salt) << 16;
    let core = theme::core(pool, commander, name, seed)?;
    Ok(Recipe {
        commander,
        archetype,
        seed,
        core,
    })
}

/// One starter's collection, and the catalog entries it needs.
fn collection(
    commander: CardIndex,
    main: Vec<CardIndex>,
) -> Result<(Collection, Vec<Card>), Error> {
    let mut counts = BTreeMap::<CardIndex, u8>::new();
    let mut owned = Vec::new();
    let mut cards = Vec::new();
    for index in std::iter::once(commander).chain(main.iter().copied()) {
        let card = metadata(index).ok_or(Error::Metadata)?;
        cards.push(card);
        if !card.basic_land {
            owned.push(Stack {
                card: index,
                copies: 1,
            });
        }
    }
    for index in main {
        *counts.entry(index).or_default() += 1;
    }
    owned.sort_by_key(|row| row.card);
    owned.dedup_by_key(|row| row.card);
    Ok((
        Collection {
            formation: Some(Formation {
                commander,
                sideboard: Vec::new(),
            }),
            owned,
            deck: counts
                .into_iter()
                .map(|(card, copies)| Stack { card, copies })
                .collect(),
        },
        cards,
    ))
}

/// The fifty 40-card build-arounds with singleton spells, reviewed utility
/// lands and a colour-correct mana base, as the calibration measured them
/// (`frozen::STARTERS`): every card is checked against this registry, never
/// rebuilt from it. This is an offline publication operation; it never runs
/// per socket, per frame or during an engine turn.
/// # Errors
/// A card this registry no longer supports, or a commander out of `LEADERS`.
pub fn expeditions() -> Result<Content, Error> {
    let mut cards = BTreeMap::<CardIndex, Card>::new();
    for name in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
        let card = named(name)?;
        cards.insert(card.card, card);
    }
    let mut starters = Vec::with_capacity(LEADERS.len());
    for (&(commander, deck), &(name, _, _)) in frozen::STARTERS.iter().zip(&LEADERS) {
        let commander = CardIndex::new(commander);
        if baylee_cards::decks::by_name(name) != Some(commander) {
            return Err(Error::Card(name));
        }
        let main = deck
            .iter()
            .flat_map(|&(card, copies)| {
                std::iter::repeat_n(CardIndex::new(card), usize::from(copies))
            })
            .collect();
        let (starter, needed) = collection(commander, main)?;
        for card in needed {
            cards.insert(card.card, card);
        }
        starters.push(starter);
    }
    let catalog = Catalog {
        cards: cards.into_values().collect(),
    };
    catalog
        .validate_starter_pool(&starters)
        .map_err(|_| Error::Formation)?;
    Ok(Content { catalog, starters })
}

/// Calibration candidates: a starter rebuilt with another archetype or seed
/// variant, or another commander in its place. Offline only; what is chosen
/// is written into `LEADERS` and published from there.
pub struct Workshop {
    pool: Pool,
}

impl Default for Workshop {
    fn default() -> Self {
        Self::new()
    }
}

impl Workshop {
    /// The admitted pool, prepared once.
    #[must_use]
    pub fn new() -> Self {
        Self {
            pool: prepared_pool(),
        }
    }

    /// The `LEADERS` entry at zero-based `index`: its commander's name.
    #[must_use]
    pub fn leader(index: usize) -> Option<(&'static str, Archetype, u16)> {
        LEADERS.get(index).copied()
    }

    /// Starter `index` built under `name`, `archetype` and seed variant `salt`,
    /// with the catalog entries it needs beyond the basics.
    /// # Errors
    /// An unknown or ineligible commander, or a deck the builder cannot make.
    pub fn variant(
        &self,
        index: usize,
        name: &str,
        archetype: Archetype,
        salt: u16,
    ) -> Result<(Collection, Vec<Card>), Error> {
        let recipe = recipe(&self.pool, index, name, archetype, salt)?;
        collection(recipe.commander, recipe.build(&self.pool)?)
    }
}
