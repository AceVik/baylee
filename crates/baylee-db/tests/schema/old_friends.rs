//! Migration 35 forgets the friend lists from before Baylee Central and keeps
//! what tables and blocks still need.
use super::{Sandbox, Uuid, an_account, before, count};
use baylee_db::migration::Migrator;
use sea_orm::{ActiveModelTrait, ConnectionTrait};
use sea_orm_migration::MigratorTrait as _;

const FORGET: &str = "m20261125_000035_drop_old_friends";

#[tokio::test]
async fn the_old_friend_lists_go_and_blocks_devices_and_table_chat_stay() {
    let sandbox = Sandbox::open_at("old_friends", Some(before(FORGET))).await;
    let db = &sandbox.db;
    let mut ids = Vec::new();
    for email in ["ada@example.org", "bo@example.org", "cy@example.org"] {
        ids.push(an_account(email).insert(db).await.expect("an account").id);
    }
    // `friendship` holds each pair once, the lower id first.
    ids.sort();
    let (a, b, c) = (ids[0], ids[1], ids[2]);
    let group = Uuid::now_v7();
    let in_group = Uuid::now_v7();
    let at_table = Uuid::now_v7();
    let game = Uuid::now_v7();
    db.execute_unprepared(&format!(
        "INSERT INTO friendship (a, b) VALUES ('{a}', '{b}');
         INSERT INTO friend_request (from_id, to_id) VALUES ('{a}', '{c}');
         INSERT INTO social_block (blocker, blocked) VALUES ('{c}', '{b}');
         INSERT INTO chat_group (id, name) VALUES ('{group}', 'Before Central');
         INSERT INTO group_member (group_id, account_id, owner)
             VALUES ('{group}', '{a}', true), ('{group}', '{b}', false);
         INSERT INTO group_invite (group_id, account_id, inviter_id)
             VALUES ('{group}', '{c}', '{a}');
         INSERT INTO social_device (id, account_id, ik, dk, dsig, label)
             VALUES ('00000000000000b0', '{b}', decode(repeat('01', 32), 'hex'),
                     decode(repeat('02', 32), 'hex'), decode(repeat('03', 64), 'hex'), 'Laptop');
         INSERT INTO chat_message (id, conv, sender_id, sender_dev, sealed)
             VALUES ('{in_group}', 'grp:{group}', '{a}', '00000000000000a0', decode('00', 'hex')),
                    ('{at_table}', 'tbl:{game}', '{a}', '00000000000000a0', decode('00', 'hex'));
         INSERT INTO chat_delivery (device_id, message_id, wrap, expires_at)
             VALUES ('00000000000000b0', '{in_group}', decode(repeat('04', 48), 'hex'),
                     now() + interval '14 days'),
                    ('00000000000000b0', '{at_table}', decode(repeat('04', 48), 'hex'),
                     now() + interval '14 days');"
    ))
    .await
    .expect("the rows from before Central");

    Migrator::up(db, None).await.expect("migration 35");

    for table in [
        "friendship",
        "friend_request",
        "chat_group",
        "group_member",
        "group_invite",
    ] {
        assert_eq!(
            count(db, &format!("SELECT 1 FROM {table}")).await,
            0,
            "{table} is empty"
        );
    }
    assert_eq!(
        count(
            db,
            &format!("SELECT 1 FROM chat_message WHERE id = '{in_group}'")
        )
        .await,
        0,
        "the group's waiting message is gone"
    );
    assert_eq!(
        count(
            db,
            &format!("SELECT 1 FROM chat_delivery WHERE message_id = '{in_group}'")
        )
        .await,
        0,
        "and its delivery with it"
    );
    assert_eq!(
        count(
            db,
            &format!("SELECT 1 FROM chat_delivery WHERE message_id = '{at_table}'")
        )
        .await,
        1,
        "the table's message still waits for its device"
    );
    assert_eq!(
        count(
            db,
            &format!("SELECT 1 FROM social_block WHERE blocker = '{c}' AND blocked = '{b}'")
        )
        .await,
        1,
        "a block still keeps table chat away"
    );
    assert_eq!(
        count(db, "SELECT 1 FROM social_device").await,
        1,
        "devices stay"
    );
    assert_eq!(count(db, "SELECT 1 FROM account").await, 3, "accounts stay");

    // Stepping back leaves the tables in place and brings nothing back.
    Migrator::down(db, Some(1)).await.expect("steps back");
    assert_eq!(count(db, "SELECT 1 FROM friendship").await, 0);
    Migrator::up(db, None).await.expect("and forward again");

    sandbox.close().await;
}
