//! Generation-six selection: every family is reachable without modulo bias.
//! Work is bounded by the published catalog and owner inventory, never opponents.
use super::{Definition, Goods, Potion, Rarity, Shop, Stock, score};
use crate::v2::catalog::Family;

pub(super) fn select(shop: &Shop, seed: &[u8; 32], slot: u8, stock: Stock<'_>) -> Goods {
    let random = score(seed, shop.stage, shop.rerolls, slot, 0);
    if slot >= 10 {
        return match random[0] {
            0..=11 if shop.stage >= 5 => Goods::Mythic,
            0..=127 => Goods::Potion(Potion::Healing),
            _ => Goods::Potion(Potion::Mana),
        };
    }
    // One guaranteed progression slot and a bounded 1/3 affinity elsewhere.
    // Exact rarities matter: a matching family with the wrong tier cannot fuse.
    if (slot == 0 || random[3] < 85)
        && let Some(definition) = affinity(shop, seed, slot, stock)
    {
        return Goods::Relic(definition);
    }
    let rarity = match random[1] {
        0..=11 if shop.stage >= 4 => Rarity::Rare,
        0..=71 if shop.stage >= 2 => Rarity::Uncommon,
        _ => Rarity::Common,
    };
    let family = Family::ALL
        .into_iter()
        .min_by_key(|family| {
            let identity = u32::from(
                Definition {
                    family: *family,
                    rarity: Rarity::Common,
                }
                .id(),
            );
            (
                score(seed, shop.stage, shop.rerolls, slot, identity),
                identity,
            )
        })
        .expect("nonempty frozen catalog");
    Goods::Relic(Definition { family, rarity })
}

fn affinity(shop: &Shop, seed: &[u8; 32], slot: u8, stock: Stock<'_>) -> Option<Definition> {
    // At most two useful tickets per definition, in canonical ID order.
    // Inventory order and equipment placement cannot reroll the same shop.
    let mut copies = [0_u8; Family::ALL.len() * 4];
    for item in &stock.economy.inventory.relics {
        if matches!(item.definition.rarity, Rarity::Common | Rarity::Uncommon)
            && Goods::Relic(item.definition).admitted(shop.stage)
        {
            let count = &mut copies[usize::from(item.definition.id() - 1)];
            *count = count.saturating_add(1).min(2);
        }
    }
    copies
        .into_iter()
        .enumerate()
        .filter(|(_, n)| *n != 0)
        .flat_map(|(zero, n)| {
            (0..n).map(move |ticket| {
                let id = u16::try_from(zero + 1).expect("bounded definition ID");
                let identity = 0x1_0000 + u32::from(id) * 2 + u32::from(ticket);
                (score(seed, shop.stage, shop.rerolls, slot, identity), id)
            })
        })
        .min()
        .and_then(|(_, id)| Definition::from_id(id))
}
