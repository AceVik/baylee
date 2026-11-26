//! The running games a gateway keeps across its restart (`running_game`,
//! migration 36): kept only while their record is started and not complete,
//! gone with the record's last piece, and read back whole.
use super::{Account, EntityTrait, Sandbox, Set, Uuid, an_account};
use baylee_db::{records, running};
use sea_orm::ConnectionTrait;

#[tokio::test]
async fn a_running_game_is_kept_from_its_first_piece_until_its_last() {
    let sandbox = Sandbox::open("running_kept").await;
    let db = &sandbox.db;
    let game = Uuid::now_v7().to_string();

    assert!(
        !running::keep(db, &game, 1, r#"{"seats":[]}"#)
            .await
            .unwrap(),
        "a game whose record is not started is not kept: there is nothing to resume"
    );
    records::open(db, &game, &[records::Seat::default()])
        .await
        .unwrap();
    assert!(
        records::append(db, &game, 0, b"head".to_vec(), false)
            .await
            .unwrap()
    );
    assert!(
        running::keep(db, &game, 1, r#"{"seats":[]}"#)
            .await
            .unwrap()
    );
    assert!(
        running::keep(db, &game, 2, r#"{"seats":[1]}"#)
            .await
            .unwrap(),
        "a later write replaces the row"
    );
    assert_eq!(
        running::all(db).await.unwrap(),
        vec![running::Running {
            game_id: game.clone(),
            kept: 2,
            body: r#"{"seats": [1]}"#.to_owned(),
        }]
    );
    assert_eq!(
        records::first_piece(db, &game).await.unwrap().as_deref(),
        Some(&b"head"[..])
    );

    assert!(
        records::append(db, &game, 1, b"end".to_vec(), true)
            .await
            .unwrap()
    );
    assert!(
        running::all(db).await.unwrap().is_empty(),
        "the last piece takes the row with it"
    );
    assert!(
        !running::keep(db, &game, 2, r#"{"seats":[]}"#)
            .await
            .unwrap(),
        "a write that comes after the end does not bring the game back"
    );
    sandbox.close().await;
}

#[tokio::test]
async fn what_an_older_gateway_left_is_cleared_and_a_gap_is_refused() {
    let sandbox = Sandbox::open("running_left").await;
    let db = &sandbox.db;
    let game = Uuid::now_v7().to_string();
    records::open(db, &game, &[]).await.unwrap();
    assert!(
        records::append(db, &game, 0, b"a".to_vec(), false)
            .await
            .unwrap()
    );
    assert!(running::keep(db, &game, 1, "{}").await.unwrap());
    // A row beside a complete record, as no gateway of this build leaves
    // one: it is not read, and the sweep removes it.
    db.execute_unprepared(&format!(
        "UPDATE game_record SET complete = true WHERE game_id = '{game}'"
    ))
    .await
    .unwrap();
    assert!(running::all(db).await.unwrap().is_empty());
    assert_eq!(running::forget_finished(db).await.unwrap(), 1);
    assert_eq!(running::forget_finished(db).await.unwrap(), 0);

    let gap = Uuid::now_v7().to_string();
    records::open(db, &gap, &[]).await.unwrap();
    assert!(
        records::append(db, &gap, 0, b"a".to_vec(), false)
            .await
            .unwrap()
    );
    assert!(
        records::append(db, &gap, 2, b"c".to_vec(), false)
            .await
            .unwrap()
    );
    assert!(
        records::stored_pieces(db, &gap).await.is_err(),
        "a record missing a piece is not resumed from"
    );
    assert!(
        records::append(db, &gap, 1, b"b".to_vec(), false)
            .await
            .unwrap()
    );
    assert_eq!(
        records::stored_pieces(db, &gap).await.unwrap(),
        Some((b"abc".to_vec(), 3))
    );
    assert!(running::keep(db, &gap, 1, "{}").await.unwrap());
    running::forget(db, &gap).await.unwrap();
    assert!(running::all(db).await.unwrap().is_empty());
    sandbox.close().await;
}

#[tokio::test]
async fn a_deleted_account_is_named_gone() {
    let sandbox = Sandbox::open("running_gone").await;
    let db = &sandbox.db;
    let here = an_account("running-here@example.invalid");
    let Set(here_id) = here.id else {
        unreachable!()
    };
    Account::insert(here).exec(db).await.unwrap();
    let gone = Uuid::now_v7();
    assert_eq!(
        running::gone_accounts(db, &[here_id, gone]).await.unwrap(),
        vec![gone]
    );
    assert!(running::gone_accounts(db, &[]).await.unwrap().is_empty());
    sandbox.close().await;
}
