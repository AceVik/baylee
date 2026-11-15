//! Reproducible first-fusion probe, not a human difficulty or win-rate estimate.
//! Run: `cargo run --locked -p baylee-tower --example market_fusion`.
//! A narrow policy commits to its first Common family, buys only its Common
//! copies, and pays for at most two refreshes per visit. No future stock is read.
use baylee_core::ids::CardIndex;
use baylee_tower::{
    cards::{Card, Catalog, Collection, Stack},
    v2::{
        MAX_STAGES,
        catalog::{Definition, Rarity},
        economy::{Economy, Recipe},
        inventory::Slot,
        shop::{Delivery, Goods, Shop, Stock},
    },
};
use sha2::{Digest, Sha256};

struct Sample {
    stage: u8,
    spent: u16,
    refreshes: u16,
}

fn fixture() -> (Catalog, Collection) {
    let mut cards = vec![Card {
        card: CardIndex::new(0),
        price: 0,
        basic_land: true,
    }];
    cards.extend((1..=8).map(|id| Card {
        card: CardIndex::new(id),
        price: 4,
        basic_land: false,
    }));
    (
        Catalog { cards },
        Collection {
            owned: Vec::new(),
            deck: vec![Stack {
                card: CardIndex::new(0),
                copies: 40,
            }],
        },
    )
}

fn simulate(
    generator: u16,
    seed: &[u8; 32],
    catalog: &Catalog,
    cards: &Collection,
) -> Option<Sample> {
    let mut economy = Economy::default();
    let mut previous = None;
    let mut family = None;
    let mut refreshes = 0;
    for stage in 1..=MAX_STAGES {
        let mut shop = Shop::open_for(
            generator,
            seed,
            stage,
            Stock {
                catalog,
                cards,
                economy: &economy,
            },
            previous.as_ref(),
        )
        .unwrap();
        if family.is_none() {
            family = shop.offers.iter().find_map(|offer| match offer.goods {
                Goods::Relic(Definition {
                    family,
                    rarity: Rarity::Common,
                }) => Some(family),
                _ => None,
            });
        }
        let wanted = Definition {
            family: family.expect("first shop has Common relics"),
            rarity: Rarity::Common,
        };
        for visit in 0..=2 {
            let offers: Vec<_> = shop
                .offers
                .iter()
                .filter(|offer| !offer.sold && offer.goods == Goods::Relic(wanted))
                .map(|offer| offer.id)
                .collect();
            for offer in offers {
                if economy.gold < 4 {
                    break;
                }
                let slot = Slot::Reserve(u8::try_from(economy.inventory.relics.len()).unwrap());
                let purchase = shop
                    .buy(
                        offer,
                        &Delivery::Slot(slot),
                        Stock {
                            catalog,
                            cards,
                            economy: &economy,
                        },
                    )
                    .unwrap();
                shop = purchase.shop;
                economy = purchase.economy;
                if economy.inventory.relics.len() == 3 {
                    let recipe = Recipe {
                        relics: economy
                            .inventory
                            .relics
                            .iter()
                            .map(|item| item.id)
                            .collect(),
                        mythic: None,
                        destination: Slot::Active(0),
                    };
                    economy = economy.fused(&recipe).unwrap();
                    assert_eq!(
                        economy.inventory.relics[0].definition.rarity,
                        Rarity::Uncommon
                    );
                    return Some(Sample {
                        stage,
                        spent: 18 + (u16::from(stage) - 1) * 12 - economy.gold,
                        refreshes,
                    });
                }
            }
            if visit == 2 || shop.reroll_price().is_none_or(|price| economy.gold < price) {
                break;
            }
            (shop, economy) = shop
                .refreshed(
                    seed,
                    Stock {
                        catalog,
                        cards,
                        economy: &economy,
                    },
                )
                .unwrap();
            refreshes += 1;
        }
        previous = Some(shop);
        if stage < MAX_STAGES {
            economy = economy.settled([0; 2]).unwrap();
        }
    }
    None
}

fn main() {
    let (catalog, cards) = fixture();
    println!(
        "Fixed first-family Common-fusion policy; 512 seeds; normal starting gold/income; <=2 paid refreshes per visit."
    );
    println!(
        "No duel, card purchase, pivot, direct Uncommon purchase or human difficulty is modeled."
    );
    for generator in [1, 2] {
        let mut samples = Vec::new();
        for number in 0_u32..512 {
            let seed: [u8; 32] = Sha256::new()
                .chain_update(b"tower-market-fusion-v1")
                .chain_update(number.to_le_bytes())
                .finalize()
                .into();
            if let Some(sample) = simulate(generator, &seed, &catalog, &cards) {
                samples.push(sample);
            }
        }
        let count = f64::from(u32::try_from(samples.len()).expect("512 samples"));
        let mean = |sum: u32| f64::from(sum) / count;
        println!(
            "generation {generator}: fused {}/512; by visit 1/2/3 = {}/{}/{}; mean visit {:.2}; mean gold spent {:.2}; mean paid refreshes {:.2}",
            samples.len(),
            samples.iter().filter(|sample| sample.stage <= 1).count(),
            samples.iter().filter(|sample| sample.stage <= 2).count(),
            samples.iter().filter(|sample| sample.stage <= 3).count(),
            mean(samples.iter().map(|sample| u32::from(sample.stage)).sum()),
            mean(samples.iter().map(|sample| u32::from(sample.spent)).sum()),
            mean(
                samples
                    .iter()
                    .map(|sample| u32::from(sample.refreshes))
                    .sum()
            )
        );
    }
}
