//! The house's opponents: what fifty scripted runs hold before each duel.
//!
//! Each starter plays one run through the real run machine: the cohort's own
//! shop stream, automatic fusion, verified-settlement income and the reward
//! cards of the opponent it met, claimed and played the way a careful player
//! would. Only the duel outcomes are scripted, along [`PATH`], the record of a
//! run that reaches the twelfth duel at all (7–4); every stage's build is the
//! run's frozen pre-duel build there, exactly what an echo of a player at
//! that stage is. The opponents whose cards a run is offered are the other
//! scripted runs at the same stage. A private seed picks the shop streams,
//! the opponents and the reward offers, so the published builds cannot be
//! derived from this repository.
//!
//! Nothing here claims balance. `baylee-tower-lab ladder` measures every
//! stage's fifty builds against each other and against the stage before, and
//! `docs/tower-operations.md` §"Calibration" records what it found.
use super::worth;
use crate::{
    Content, Error,
    expeditions::{Recipe, prepared_pool, recipes},
};
use baylee_cards::deckgen::Pool;
use baylee_core::{color::ColorSet, ids::CardIndex};
use baylee_protocol::execution::{Identity, Variant};
use baylee_tower::{
    cards::{CardRarity, Catalog, Formation, Stack},
    v2::{
        GENERATOR, MAX_STAGES,
        catalog::Definition,
        execution::{MAX_RECORD_BYTES, Prefix},
        generation,
        inventory::Slot,
        package::{Build, Cohort, Opponent, Origin, Package, Seeds},
        run::{Binding, Command, Outcome, Phase, Run, Settlement, WorkerCommand},
        shop::{Delivery, Goods},
    },
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

/// The scripted outcomes of duels one to eleven: seven victories and four
/// defeats, alternating early so that no stage's build is all luck or all loss.
pub const PATH: [Outcome; 11] = [
    Outcome::Victory,
    Outcome::Defeat,
    Outcome::Victory,
    Outcome::Defeat,
    Outcome::Victory,
    Outcome::Victory,
    Outcome::Defeat,
    Outcome::Victory,
    Outcome::Defeat,
    Outcome::Victory,
    Outcome::Victory,
];

/// At most this many paid refreshes per visit; a careful player stops there.
const REFRESHES: u8 = 2;

/// Every stage's house builds, one per starter, in publication order.
pub struct Ladder {
    /// `stages[stage - 1][archetype - 1]`.
    pub stages: Vec<Vec<Build>>,
}

impl Ladder {
    /// The build published for one-based `stage` and `archetype`.
    #[must_use]
    pub fn build(&self, stage: u8, archetype: u8) -> Option<&Build> {
        self.stages
            .get(usize::from(stage).checked_sub(1)?)?
            .get(usize::from(archetype).checked_sub(1)?)
    }
}

/// Play the fifty scripted runs of `content` under the private `seed`.
/// # Errors
/// A run the real machine refuses, which is a defect here, never a balance result.
pub fn ladder(content: &Content, seed: &[u8; 32]) -> Result<Ladder, Error> {
    let catalog = &content.catalog;
    let cohort = construction_cohort(catalog)?;
    let pool = prepared_pool();
    let recipes = recipes(&pool)?;
    if recipes.len() != content.starters.len() {
        return Err(Error::House("starters and recipes disagree"));
    }
    let mut runs = Vec::with_capacity(content.starters.len());
    for (index, starter) in content.starters.iter().enumerate() {
        let run_seed = stream(seed, b"run", index, 0);
        let run = Run::begin(catalog, starter.clone(), &run_seed).map_err(refused)?;
        runs.push(Scripted {
            run,
            seed: run_seed,
            recipe: &recipes[index],
        });
    }
    let mut stages = Vec::with_capacity(usize::from(MAX_STAGES));
    for stage in 1..=MAX_STAGES {
        for scripted in &mut runs {
            scripted.shop(catalog, &pool)?;
        }
        let builds = runs
            .iter()
            .map(|scripted| Build::freeze(&scripted.run, catalog).map_err(refused))
            .collect::<Result<Vec<_>, _>>()?;
        if stage < MAX_STAGES {
            let outcome = PATH[usize::from(stage) - 1];
            let count = runs.len();
            for (index, scripted) in runs.iter_mut().enumerate() {
                let pick = stream(seed, b"opponent", index, stage);
                let offset = usize::from(pick[0]) | usize::from(pick[1]) << 8;
                let opponent = (index + 1 + offset % (count - 1)) % count;
                scripted.duel(
                    catalog,
                    &cohort,
                    Opponent {
                        stage,
                        cohort: cohort.clone(),
                        origin: Origin::House {
                            archetype: u8::try_from(opponent + 1)
                                .map_err(|_| Error::House("more starters than archetypes"))?,
                        },
                        build: builds[opponent].clone(),
                    },
                    outcome,
                    &stream(seed, b"reward", index, stage),
                )?;
            }
        }
        stages.push(builds);
    }
    Ok(Ladder { stages })
}

/// A fixed executor for building the ladder only. Reward offers hash the
/// package, so a real executor's identity would make the builds depend on
/// which commit built the engine; this one never plays a duel.
fn construction_cohort(catalog: &Catalog) -> Result<Cohort, Error> {
    let identity = Identity::native(
        Variant::Tower,
        "house-ladder",
        "0000000000000000000000000000000000000000",
        u32::from(generation::CURRENT.ai_generation),
    );
    Cohort::for_generation(identity, catalog, GENERATOR).map_err(refused)
}

struct Scripted<'a> {
    run: Run,
    seed: [u8; 32],
    recipe: &'a Recipe,
}

impl Scripted<'_> {
    fn command(&mut self, catalog: &Catalog, command: &Command) -> Result<(), Error> {
        self.run = self
            .run
            .command(self.run.revision, command, catalog, &self.seed)
            .map_err(refused)?;
        Ok(())
    }

    /// One shop visit: refit the deck, buy what improves the loadout most per
    /// gold, refresh a little when nothing does, then equip the best six.
    fn shop(&mut self, catalog: &Catalog, pool: &Pool) -> Result<(), Error> {
        if self.run.phase != Phase::Shop {
            return Err(Error::House("a scripted run left the shop"));
        }
        self.sell_unplayable(catalog)?;
        self.refit(catalog, pool)?;
        let deck = worth::Deck::of(&self.run.cards.deck);
        loop {
            if let Some(offer) = self.best_relic(&deck) {
                self.command(
                    catalog,
                    &Command::Buy {
                        offer,
                        delivery: Delivery::Inventory,
                    },
                )?;
                continue;
            }
            let gold = self.run.economy.gold;
            match self.run.shop.reroll_price() {
                Some(price) if self.run.shop.rerolls < REFRESHES && gold >= price + 4 => {
                    self.command(catalog, &Command::Refresh {})?;
                }
                _ => break,
            }
        }
        self.buy_potions(catalog)?;
        self.equip(catalog, &deck)
    }

    /// The unsold, affordable relic whose purchase raises the best six's worth
    /// most per gold, counting a second copy as progress towards its fusion.
    fn best_relic(&self, deck: &worth::Deck) -> Option<u32> {
        let economy = &self.run.economy;
        let owned: Vec<Definition> = economy
            .inventory
            .relics
            .iter()
            .map(|relic| relic.definition)
            .collect();
        let before = worth::best_six(&owned, deck);
        let mut best: Option<(f64, u32)> = None;
        for offer in self.run.shop.offers.iter().filter(|offer| !offer.sold) {
            let Goods::Relic(definition) = offer.goods else {
                continue;
            };
            let Some(price) = definition.price().filter(|price| *price <= economy.gold) else {
                continue;
            };
            let Ok(after) = economy.bought_automatic(definition) else {
                continue;
            };
            let items: Vec<Definition> = after
                .inventory
                .relics
                .iter()
                .map(|relic| relic.definition)
                .collect();
            let copies = items.iter().filter(|item| **item == definition).count();
            let progress = match worth::fused(definition.rarity) {
                Some(rarity) if copies == 2 => {
                    0.35 * (worth::of(
                        Definition {
                            family: definition.family,
                            rarity,
                        },
                        deck,
                    ) - worth::of(definition, deck))
                    .max(0.0)
                }
                _ => 0.0,
            };
            let gain = worth::best_six(&items, deck) - before + progress;
            let efficiency = gain / f64::from(price.max(1));
            if gain > 0.25 && best.is_none_or(|(value, _)| efficiency > value) {
                best = Some((efficiency, offer.id));
            }
        }
        best.map(|(_, offer)| offer)
    }

    /// Spare gold buys table charges: healing first, it is the cheaper.
    fn buy_potions(&mut self, catalog: &Catalog) -> Result<(), Error> {
        loop {
            let economy = &self.run.economy;
            if economy.potions.iter().sum::<u8>() >= 2 {
                return Ok(());
            }
            let offer = self
                .run
                .shop
                .offers
                .iter()
                .filter(|offer| !offer.sold)
                .filter_map(|offer| match offer.goods {
                    Goods::Potion(potion) if potion.price() <= economy.gold => {
                        Some((potion.price(), offer.id))
                    }
                    _ => None,
                })
                .min();
            let Some((_, offer)) = offer else {
                return Ok(());
            };
            self.command(
                catalog,
                &Command::Buy {
                    offer,
                    delivery: Delivery::Inventory,
                },
            )?;
        }
    }

    /// Equip the six relics worth most to this deck, in that order.
    fn equip(&mut self, catalog: &Catalog, deck: &worth::Deck) -> Result<(), Error> {
        let mut ranked: Vec<(f64, u32)> = self
            .run
            .economy
            .inventory
            .relics
            .iter()
            .map(|relic| (worth::of(relic.definition, deck), relic.id))
            .collect();
        ranked.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        let wanted: Vec<u32> = ranked.iter().take(6).map(|(_, id)| *id).collect();
        let equipped: Vec<u32> = self
            .run
            .economy
            .inventory
            .relics
            .iter()
            .filter(|relic| matches!(relic.slot, Slot::Active(_)))
            .map(|relic| relic.id)
            .collect();
        for item in equipped {
            self.command(
                catalog,
                &Command::Move {
                    item,
                    slot: Slot::Inventory,
                },
            )?;
        }
        for (slot, item) in wanted.into_iter().enumerate() {
            self.command(
                catalog,
                &Command::Move {
                    item,
                    slot: Slot::Active(u8::try_from(slot).map_err(|_| Error::Formation)?),
                },
            )?;
        }
        Ok(())
    }

    /// Reward cards outside the commander's colours cannot be played: sell them.
    fn sell_unplayable(&mut self, catalog: &Catalog) -> Result<(), Error> {
        let identity = self.identity(catalog);
        let unplayable: Vec<CardIndex> = self
            .run
            .cards
            .owned
            .iter()
            .map(|stack| stack.card)
            .filter(|card| {
                Some(*card) != self.commander()
                    && catalog
                        .card(*card)
                        .and_then(|entry| entry.run)
                        .is_some_and(|rules| !rules.identity.difference(identity).is_empty())
            })
            .collect();
        for card in unplayable {
            self.command(catalog, &Command::SellCard { card })?;
        }
        Ok(())
    }

    /// Rebuild the library from every owned card with the starter's own builder.
    fn refit(&mut self, catalog: &Catalog, pool: &Pool) -> Result<(), Error> {
        let Some(commander) = self.commander() else {
            return Err(Error::Formation);
        };
        let owned: BTreeSet<CardIndex> = self
            .run
            .cards
            .owned
            .iter()
            .map(|stack| stack.card)
            .collect();
        let in_deck: BTreeSet<CardIndex> =
            self.run.cards.deck.iter().map(|stack| stack.card).collect();
        if owned
            .iter()
            .all(|card| *card == commander || in_deck.contains(card))
        {
            return Ok(());
        }
        let available = Pool {
            entries: pool
                .entries
                .iter()
                .filter(|entry| entry.basic || owned.contains(&entry.card))
                .cloned()
                .collect(),
        };
        // A pool the builder cannot fill keeps the deck the run already has.
        let Ok(main) = self.recipe.build(&available) else {
            return Ok(());
        };
        let mut counts = BTreeMap::<CardIndex, u8>::new();
        for card in main {
            *counts.entry(card).or_default() += 1;
        }
        let deck: Vec<Stack> = counts
            .into_iter()
            .map(|(card, copies)| Stack { card, copies })
            .collect();
        if deck == self.run.cards.deck {
            return Ok(());
        }
        self.command(
            catalog,
            &Command::Refit {
                deck,
                formation: Formation {
                    commander,
                    sideboard: Vec::new(),
                },
            },
        )
    }

    /// Reserve against `opponent`, settle with the scripted `outcome` and
    /// claim the reward: the verifier's transitions without a played duel.
    fn duel(
        &mut self,
        catalog: &Catalog,
        cohort: &Cohort,
        opponent: Opponent,
        outcome: Outcome,
        reward_seed: &[u8; 32],
    ) -> Result<(), Error> {
        let potions = self.run.economy.potions;
        let reservation = Package::reserve(
            &self.run,
            self.run.revision,
            catalog,
            cohort,
            opponent,
            Seeds {
                shuffle: 1,
                policy: 2,
            },
            MAX_RECORD_BYTES,
        )
        .map_err(refused)?;
        let package = reservation.package;
        let digest = package.digest(catalog).map_err(refused)?;
        self.run = reservation.run;
        self.worker(catalog, 1, WorkerCommand::Claim { duration: 60_000 })?;
        let fence = self.execution()?.fence;
        self.worker(
            catalog,
            2,
            WorkerCommand::Append {
                fence,
                expected: Prefix::default(),
                chunk: b"scripted house-ladder duel",
            },
        )?;
        let prefix = self.execution()?.prefix;
        self.worker(
            catalog,
            3,
            WorkerCommand::Seal {
                fence,
                expected: prefix,
            },
        )?;
        self.run = self
            .run
            .settled_against(
                self.run.revision,
                Settlement {
                    binding: Binding {
                        package: digest,
                        fence,
                        prefix,
                    },
                    outcome,
                    turns: 17,
                    // A player buys charges to use them.
                    used: potions,
                },
                catalog,
                reward_seed,
                &package,
            )
            .map_err(refused)?;
        let reward = self
            .run
            .reward
            .clone()
            .ok_or(Error::House("a settled run has no reward"))?;
        let cards = self.choose(catalog, &reward.cards, reward.take);
        self.command(catalog, &Command::Claim { cards })
    }

    fn worker(
        &mut self,
        catalog: &Catalog,
        now: u64,
        command: WorkerCommand<'_>,
    ) -> Result<(), Error> {
        self.run = self
            .run
            .worker(self.run.revision, now, command, catalog)
            .map_err(refused)?;
        Ok(())
    }

    fn execution(&self) -> Result<&baylee_tower::v2::execution::Execution, Error> {
        self.run
            .execution
            .as_ref()
            .ok_or(Error::House("a reserved run has no execution"))
    }

    /// Playable new cards first, the rarer the better; then a duplicate,
    /// which pays gold; off-colour cards last (they are sold).
    fn choose(&self, catalog: &Catalog, offered: &[CardIndex], take: u8) -> Vec<CardIndex> {
        let identity = self.identity(catalog);
        let owned: BTreeSet<CardIndex> = self
            .run
            .cards
            .owned
            .iter()
            .map(|stack| stack.card)
            .collect();
        let mut ranked: Vec<(u8, CardIndex)> = offered
            .iter()
            .map(|card| {
                let rules = catalog.card(*card).and_then(|entry| entry.run);
                let playable =
                    rules.is_some_and(|rules| rules.identity.difference(identity).is_empty());
                let rarity = match rules
                    .and_then(|rules| rules.presentation)
                    .map(|art| art.rarity)
                {
                    Some(CardRarity::Mythic) => 3,
                    Some(CardRarity::Rare) => 2,
                    Some(CardRarity::Uncommon) => 1,
                    _ => 0,
                };
                let rank = if owned.contains(card) {
                    4
                } else if playable {
                    10 + rarity
                } else {
                    rarity
                };
                (rank, *card)
            })
            .collect();
        ranked.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let mut cards: Vec<CardIndex> = ranked
            .into_iter()
            .take(usize::from(take))
            .map(|(_, card)| card)
            .collect();
        cards.sort_unstable();
        cards
    }

    fn commander(&self) -> Option<CardIndex> {
        self.run
            .cards
            .formation
            .as_ref()
            .map(|formation| formation.commander)
    }

    fn identity(&self, catalog: &Catalog) -> ColorSet {
        self.commander()
            .and_then(|commander| catalog.card(commander))
            .and_then(|card| card.run)
            .map_or(ColorSet::EMPTY, |rules| rules.identity)
    }
}

/// A private, independent 32-byte stream per purpose, starter and stage.
fn stream(seed: &[u8; 32], purpose: &[u8], starter: usize, stage: u8) -> [u8; 32] {
    Sha256::new()
        .chain_update(b"baylee/tower/house-ladder/v1\0")
        .chain_update(seed)
        .chain_update(purpose)
        .chain_update((starter as u64).to_le_bytes())
        .chain_update([stage])
        .finalize()
        .into()
}

fn refused(refusal: baylee_tower::Refusal) -> Error {
    Error::Refused(format!("{refusal:?}"))
}

/// The ladder's cohort-independent checks: every cell, a legal build, the
/// starter's own commander.
/// # Errors
/// A missing cell or a build the catalog refuses.
pub fn check(ladder: &Ladder, content: &Content) -> Result<(), Error> {
    if ladder.stages.len() != usize::from(MAX_STAGES) {
        return Err(Error::House("a stage is missing"));
    }
    for builds in &ladder.stages {
        if builds.len() != content.starters.len() {
            return Err(Error::House("a starter is missing at a stage"));
        }
        for (build, starter) in builds.iter().zip(&content.starters) {
            build
                .validate_for(&content.catalog, GENERATOR)
                .map_err(refused)?;
            if build.formation.as_ref().map(|f| f.commander)
                != starter.formation.as_ref().map(|f| f.commander)
            {
                return Err(Error::House("a house build changed its commander"));
            }
        }
    }
    Ok(())
}
