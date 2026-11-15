use super::*;
use sha2::{Digest, Sha256};

#[test]
fn original_offer_stream_and_catalog_commitment_stay_byte_identical() {
    let (catalog, cards, economy) = fixture();
    let mut digest = Sha256::new();
    for seed in 0..32 {
        let mut previous = None;
        for stage in 1..=MAX_STAGES {
            let mut shop = Shop::open_for(
                1,
                &[seed; 32],
                stage,
                Stock {
                    catalog: &catalog,
                    cards: &cards,
                    economy: &economy,
                },
                previous.as_ref(),
            )
            .unwrap();
            digest.update(serde_json::to_vec(&shop).unwrap());
            // Stable pinned goods and every paid refresh are part of the fixture.
            shop = shop.pinned(shop.offers[3].id, true, &catalog).unwrap();
            let mut wallet = economy.clone();
            for _ in 0..REROLLS {
                (shop, wallet) = shop
                    .refreshed(
                        &[seed; 32],
                        Stock {
                            catalog: &catalog,
                            cards: &cards,
                            economy: &wallet,
                        },
                    )
                    .unwrap();
                digest.update(serde_json::to_vec(&shop).unwrap());
            }
            previous = Some(shop);
        }
    }
    // Captured from generation one's implementation before versioning was added.
    assert_eq!(
        <[u8; 32]>::from(digest.finalize()),
        [
            72, 189, 20, 63, 210, 152, 103, 245, 233, 69, 249, 179, 94, 90, 182, 182, 35, 225, 175,
            227, 145, 175, 80, 15, 230, 40, 243, 114, 119, 35, 124, 238
        ]
    );
    assert_eq!(
        crate::v2::package::catalog_digest_for(&catalog, 1).unwrap(),
        [
            105, 165, 112, 95, 28, 167, 180, 73, 190, 204, 15, 248, 252, 126, 46, 181, 235, 224,
            185, 21, 78, 156, 170, 149, 216, 230, 49, 53, 219, 178, 71, 191
        ]
    );
}

#[test]
fn legacy_shop_encoding_defaults_to_one_and_unknown_generations_are_refused() {
    let (catalog, cards, economy) = fixture();
    let stock = Stock {
        catalog: &catalog,
        cards: &cards,
        economy: &economy,
    };
    let shop = Shop::open_for(1, &[7; 32], 1, stock, None).unwrap();
    let bytes = serde_json::to_vec(&shop).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(value.get("generator").is_none());
    let decoded: Shop = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(decoded.generator, 1);
    assert_eq!(serde_json::to_vec(&decoded).unwrap(), bytes);
    for generator in [0, 3, u16::MAX] {
        assert!(Shop::open_for(generator, &[7; 32], 1, stock, None).is_err());
        let mut changed = value.clone();
        changed["generator"] = generator.into();
        let changed: Shop = serde_json::from_value(changed).unwrap();
        assert!(changed.validate(&catalog).is_err());
        assert!(changed.refreshed(&[7; 32], stock).is_err());
        assert!(Shop::open(&[7; 32], 2, stock, Some(&changed)).is_err());
    }
}
