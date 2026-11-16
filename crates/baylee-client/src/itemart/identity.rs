//! Only compiled original identities may use our item art. Never match a name.
use std::sync::LazyLock;

use baylee_cards::tokens;
use baylee_client_core::images::{Face, ImageKey, ImageSource};

use super::{Definition, Family, Goods, Potion, Rarity};

pub(crate) fn of(key: Option<ImageKey>) -> Option<Goods> {
    let key = key?;
    if key.face != Face::Front {
        return None;
    }
    let ImageSource::Token(id) = key.source else {
        return None;
    };
    IDENTITIES
        .get(usize::from(id))
        .copied()
        .flatten()
        .map(|identity| identity.goods)
}

#[derive(Clone, Copy)]
struct Identity {
    goods: Goods,
    run: bool,
}

/// Only compiled run sources belong in the drawer. A matching name is never enough.
/// Callers inspect command-zone objects; retained artifact items stay on the table.
pub(crate) fn run_source(object: &baylee_view::PublicObject) -> Option<Goods> {
    if object.card.is_some() {
        return None;
    }
    let identity = IDENTITIES
        .get(usize::from(object.token?))
        .copied()
        .flatten()?;
    identity.run.then_some(identity.goods)
}

pub(crate) fn auxiliary(object: &baylee_view::PublicObject) -> bool {
    run_source(object).is_some()
}

static IDENTITIES: LazyLock<Box<[Option<Identity>]>> = LazyLock::new(|| {
    let mut identities = vec![None; tokens::ALL.len()].into_boxed_slice();
    let mut family = |family, tiers: [&baylee_cards_dsl::TokenDef; 4]| {
        for (token, rarity) in tiers.into_iter().zip(Rarity::ALL) {
            identities[usize::from(tokens::token_id(token))] = Some(Identity {
                goods: Goods::Relic(Definition { family, rarity }),
                run: false,
            });
        }
    };
    family(
        Family::Grove,
        [
            &tokens::TOWER_GROVE_COMMON,
            &tokens::TOWER_GROVE_UNCOMMON,
            &tokens::TOWER_GROVE_RARE,
            &tokens::TOWER_GROVE_MYTHIC,
        ],
    );
    for (kind, tiers) in [
        Family::SpringChalice,
        Family::Hearthstone,
        Family::BloodAmber,
        Family::LifeCoil,
    ]
    .into_iter()
    .zip(tokens::tower::LIFE_RELICS)
    {
        family(kind, tiers);
    }
    for (kind, tiers) in [
        Family::SeedCradle,
        Family::SwarmBanner,
        Family::HarvestWheel,
        Family::RootReservoir,
    ]
    .into_iter()
    .zip(tokens::tower::GROWTH_RELICS)
    {
        family(kind, tiers);
    }
    for (token, potion) in [
        (&tokens::TOWER_HEALING_POTION, Potion::Healing),
        (&tokens::TOWER_MANA_POTION, Potion::Mana),
    ] {
        identities[usize::from(tokens::token_id(token))] = Some(Identity {
            goods: Goods::Potion(potion),
            run: true,
        });
    }
    for (kind, tiers) in Family::ALL.into_iter().zip(tokens::tower::run::RUN_RELICS) {
        for (token, rarity) in tiers.into_iter().zip(Rarity::ALL) {
            identities[usize::from(tokens::token_id(token))] = Some(Identity {
                goods: Goods::Relic(Definition {
                    family: kind,
                    rarity,
                }),
                run: true,
            });
        }
    }
    identities
});

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::images::ArtSize;

    #[test]
    fn stable_original_identities_keep_their_rarity_and_exclude_prints() {
        assert_eq!(IDENTITIES.iter().flatten().count(), 110);
        let key = |token| Some(ImageKey::token(tokens::token_id(token), ArtSize::Normal));
        assert_eq!(
            of(key(&tokens::TOWER_ROOTS_MYTHIC)),
            Some(Goods::Relic(Definition {
                family: Family::RootReservoir,
                rarity: Rarity::MythicRare
            }))
        );
        assert_eq!(
            of(key(&tokens::TOWER_GROVE_RARE)),
            Some(Goods::Relic(Definition {
                family: Family::Grove,
                rarity: Rarity::Rare
            }))
        );
        assert_eq!(
            of(key(&tokens::TOWER_HEALING_POTION)),
            Some(Goods::Potion(Potion::Healing))
        );
        assert_eq!(of(Some(ImageKey::card_back(ArtSize::Normal))), None);
        assert_eq!(
            of(Some(ImageKey::new(
                baylee_core::ids::PrintRef(0),
                0,
                ArtSize::Normal
            ))),
            None
        );
        assert_eq!(of(Some(ImageKey::token(u16::MAX, ArtSize::Normal))), None);
        let mut back = ImageKey::token(
            tokens::token_id(&tokens::TOWER_ROOTS_MYTHIC),
            ArtSize::Normal,
        );
        back.face = Face::Back;
        assert_eq!(of(Some(back)), None);
    }
}
