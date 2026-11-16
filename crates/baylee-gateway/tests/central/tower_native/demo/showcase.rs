//! Synthetic visual fixture only: never linked into a gateway or operator binary.
use super::super::{Fixture, demo};
use baylee_tower::v2::{
    catalog::{Definition, Family, Potion, Rarity},
    inventory::{Material, Relic, Slot},
    run::Run,
    shop::Goods,
};
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use uuid::Uuid;

pub(super) async fn prepare(fixture: &Fixture, id: Uuid) {
    assert!(
        demo::enabled(),
        "showcase is confined to the synthetic native fixture"
    );
    let schema = &fixture.sandbox.schema;
    assert!(
        schema.starts_with("t_tower_")
            && schema
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_')
    );
    let db = &fixture.sandbox.db;
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            format!("SELECT state FROM \"{schema}\".runs WHERE id=$1 AND cohort=$2"),
            [id.into(), fixture.cohort.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    let bytes: Vec<u8> = row.try_get("", "state").unwrap();
    let mut state: Run = serde_json::from_slice(&bytes).unwrap();
    populate(&mut state);
    let publication = demo::publication(&fixture.identity);
    state
        .validate_for(&publication.catalog, publication.cohort.generator)
        .unwrap();
    let changed = db
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            format!("UPDATE \"{schema}\".runs SET state=$3,revision=$4 WHERE id=$1 AND cohort=$2"),
            [
                id.into(),
                fixture.cohort.into(),
                serde_json::to_vec(&state).unwrap().into(),
                i64::try_from(state.revision).unwrap().into(),
            ],
        ))
        .await
        .unwrap();
    assert_eq!(changed.rows_affected(), 1);
}

fn populate(run: &mut Run) {
    use Family as F;
    use Rarity as R;
    let inventory = &mut run.economy.inventory;
    assert!(run.history.is_empty() && inventory.relics.is_empty());
    for (family, rarity, slot) in [
        (F::Grove, R::Common, Slot::Active(0)),
        (F::Grove, R::Common, Slot::Inventory),
        (F::Grove, R::Uncommon, Slot::Inventory),
        (F::Grove, R::Uncommon, Slot::Inventory),
        (F::BloodAmber, R::MythicRare, Slot::Active(1)),
        (F::Heartwood, R::Rare, Slot::Active(2)),
        (F::Satchel, R::Uncommon, Slot::Active(3)),
        (F::StormSpindle, R::Rare, Slot::Active(4)),
        (F::GildedAcorn, R::MythicRare, Slot::Active(5)),
    ]
    .into_iter()
    .chain(
        F::ALL
            .into_iter()
            .filter(|f| *f != F::Grove)
            .flat_map(|f| [R::Common, R::Rare].map(move |r| (f, r, Slot::Inventory))),
    ) {
        inventory.relics.push(Relic {
            id: inventory.next_id,
            definition: Definition { family, rarity },
            slot,
            basis: 0,
        });
        inventory.next_id += 1;
    }
    inventory.materials.push(Material {
        id: inventory.next_id,
        basis: 0,
    });
    inventory.next_id += 1;
    let supplies = Potion::Healing.price() + Potion::Mana.price();
    run.economy.gold = baylee_tower::v2::STARTING_GOLD - supplies;
    run.ledger.spent = u64::from(supplies);
    run.economy.potions = [1, 1];
    let relic = |family, rarity| Goods::Relic(Definition { family, rarity });
    for (offer, goods) in run.shop.offers.iter_mut().zip([
        relic(F::Grove, R::Common),
        relic(F::BloodAmber, R::Common),
        relic(F::Grove, R::Common),
        relic(F::Satchel, R::Common),
        relic(F::Heartwood, R::Common),
        relic(F::GildedAcorn, R::Common),
        Goods::Potion(Potion::Healing),
        Goods::Potion(Potion::Mana),
    ]) {
        offer.goods = goods;
    }
    run.revision += 1;
}

#[test]
fn showcase_is_valid_and_contains_a_real_cascading_purchase() {
    let identity = baylee_protocol::execution::Identity::native(
        baylee_protocol::execution::Variant::Tower,
        "fixture",
        "fixture",
        22,
    );
    let publication = demo::publication(&identity);
    let mut run = Run::begin_for(
        &publication.catalog,
        publication.starters[0].clone(),
        &[3; 32],
        publication.cohort.generator,
    )
    .unwrap();
    populate(&mut run);
    run.validate_for(&publication.catalog, publication.cohort.generator)
        .unwrap();
    let grove = Definition {
        family: Family::Grove,
        rarity: Rarity::Common,
    };
    assert_eq!(
        run.economy
            .inventory
            .relics
            .iter()
            .filter(|r| r.definition == grove)
            .count(),
        2
    );
    assert_eq!(run.economy.inventory.active().len(), 6);
    assert!(run.economy.inventory.relics.len() > 36);
    let after = run
        .command(
            run.revision,
            &baylee_tower::v2::run::Command::Buy {
                offer: run.shop.offers[0].id,
                delivery: baylee_tower::v2::shop::Delivery::Inventory,
            },
            &publication.catalog,
            &[3; 32],
        )
        .unwrap();
    assert_eq!(after.economy.gold, run.economy.gold - 4);
    let equipped = after
        .economy
        .inventory
        .relics
        .iter()
        .find(|r| r.slot == Slot::Active(0))
        .unwrap();
    assert_eq!(
        equipped.definition,
        Definition {
            family: Family::Grove,
            rarity: Rarity::Rare
        }
    );
    assert_eq!(
        after.economy.inventory.relics.len(),
        run.economy.inventory.relics.len() - 3
    );
}
