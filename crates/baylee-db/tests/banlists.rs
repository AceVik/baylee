//! What only a real server can answer about `baylee_db::banlists`: who owns
//! what, what a copy holds, when a version moves, and what a cascade takes.
//!
//! Without `DATABASE_URL` they fail rather than skip, as `social.rs` does.

use baylee_cardtext::{MAX_LIST_ENTRIES, MAX_LISTS_PER_ACCOUNT};
use baylee_db::banlists::{self, Changed, Created, Edit, Entry, Status};
use baylee_db::entity::account;
use sea_orm::{
    ActiveModelTrait,
    ActiveValue::{NotSet, Set},
    ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement,
};
use time::OffsetDateTime;
use uuid::Uuid;

/// A database to run against, and the schema to run in.
struct Sandbox {
    db: DatabaseConnection,
    admin: DatabaseConnection,
    schema: String,
}

impl Sandbox {
    /// Make a fresh schema and migrate it.
    async fn open(what: &str) -> Self {
        let url = std::env::var("DATABASE_URL")
            .ok()
            .filter(|u| !u.is_empty())
            .expect("DATABASE_URL is not set, and these tests are about PostgreSQL");
        let schema: String = format!("t_{what}_{}", Uuid::now_v7().simple())
            .chars()
            .take(63)
            .collect();
        let admin = Database::connect(&url).await.expect("connecting");
        admin
            .execute_unprepared(&format!("CREATE SCHEMA \"{schema}\""))
            .await
            .expect("creating a schema");
        let sep = if url.contains('?') { '&' } else { '?' };
        let scoped = format!("{url}{sep}options=-c%20search_path%3D{schema},public");
        let db = baylee_db::connect(&scoped, 2)
            .await
            .expect("migrating a fresh schema");
        Self { db, admin, schema }
    }

    /// Give the schema back.
    async fn close(self) {
        self.admin
            .execute_unprepared(&baylee_db::drop_schema_statement(&self.schema))
            .await
            .expect("dropping the schema");
    }

    /// Inserts an account and answers its id.
    async fn person(&self, name: &str) -> Uuid {
        account::ActiveModel {
            id: Set(Uuid::now_v7()),
            email: Set(Some(format!("{}@example.org", name.to_lowercase()))),
            username: Set(None),
            username_key: Set(None),
            display_name: Set(name.to_owned()),
            tag: NotSet,
            password_hash: Set(Some(
                "$argon2id$v=19$m=19456,t=2,p=1$c2FsdA$aGFzaA".to_owned(),
            )),
            created_at: Set(OffsetDateTime::now_utc()),
            confirmed_at: Set(None),
            lang: Set("de".to_owned()),
            guest: Set(false),
            invite_id: Set(None),
            terms_version: Set(None),
            terms_accepted_at: Set(None),
            must_change_password: Set(false),
            banned_at: Set(None),
            banned_until: Set(None),
            ban_reason: Set(None),
            banned_by: Set(None),
        }
        .insert(&self.db)
        .await
        .expect("inserting an account")
        .id
    }

    /// How many rows a statement counts.
    async fn count(&self, sql: &str) -> i64 {
        self.db
            .query_one_raw(Statement::from_string(DbBackend::Postgres, sql.to_owned()))
            .await
            .expect("counting")
            .expect("a row")
            .try_get("", "n")
            .expect("a count")
    }
}

fn entry(n: u128, status: Status) -> Entry {
    Entry {
        oracle_id: Uuid::from_u128(n),
        status,
    }
}

async fn made(db: &DatabaseConnection, owner: Uuid, name: &str) -> banlists::Banlist {
    match banlists::create(db, owner, name, "modern", None)
        .await
        .expect("creating")
    {
        Created::Made(list) => list,
        other => panic!("{other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_list_is_its_owners_and_an_official_one_is_everyones_to_read() {
    let sandbox = Sandbox::open("own").await;
    let db = &sandbox.db;
    let alice = sandbox.person("Alice").await;
    let bob = sandbox.person("Bob").await;
    banlists::sync_official(db, "commander", &[entry(1, Status::Banned)])
        .await
        .expect("syncing");
    let list = made(db, alice, "Kitchen table").await;
    assert!(!list.builtin);
    assert_eq!(list.owner, Some(alice));
    assert_eq!(list.version, 1);

    let seen = banlists::listed(db, alice).await.expect("listing");
    assert_eq!(seen.len(), 2, "the official one first, then her own");
    assert!(seen[0].builtin);
    assert_eq!(seen[1].id, list.id);

    let theirs = banlists::listed(db, bob).await.expect("listing");
    assert_eq!(theirs.len(), 1, "Bob sees no list of Alice's");
    assert!(theirs[0].builtin);

    assert!(
        banlists::readable(db, list.id, bob)
            .await
            .expect("reading")
            .is_none(),
        "and cannot read one by its id"
    );
    assert_eq!(
        banlists::edit(db, list.id, bob, &Edit::default())
            .await
            .expect("editing"),
        Changed::NoSuchList,
        "another's list reads as none at all"
    );
    assert!(
        !banlists::delete(db, list.id, bob).await.expect("deleting"),
        "nor deletes it"
    );
    assert!(
        banlists::delete(db, list.id, alice)
            .await
            .expect("deleting")
    );
    sandbox.close().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn an_official_list_is_read_only_and_copied_whole() {
    let sandbox = Sandbox::open("copy").await;
    let db = &sandbox.db;
    let alice = sandbox.person("Alice").await;
    let cards = [entry(1, Status::Banned), entry(2, Status::Restricted)];
    assert!(
        banlists::sync_official(db, "vintage", &cards)
            .await
            .expect("syncing")
    );
    let official = banlists::listed(db, alice).await.expect("listing")[0].clone();
    assert_eq!(official.cards, 2);
    assert_eq!(official.owner, None);

    assert_eq!(
        banlists::edit(
            db,
            official.id,
            alice,
            &Edit {
                name: Some("Mine now".to_owned()),
                ..Edit::default()
            }
        )
        .await
        .expect("editing"),
        Changed::Official
    );
    assert!(
        !banlists::delete(db, official.id, alice)
            .await
            .expect("deleting"),
        "and nobody's to delete"
    );

    let Created::Made(copy) =
        banlists::create(db, alice, "My Vintage", "vintage", Some(official.id))
            .await
            .expect("copying")
    else {
        panic!("a copy");
    };
    assert_eq!(copy.cards, 2);
    assert_eq!(copy.owner, Some(alice));
    assert!(!copy.builtin);
    let mut held = banlists::entries(db, copy.id).await.expect("reading");
    held.sort_unstable();
    assert_eq!(held, cards);

    // A copy stands on its own: the feed moving does not move it.
    banlists::sync_official(db, "vintage", &[entry(3, Status::Banned)])
        .await
        .expect("syncing");
    assert_eq!(
        banlists::entries(db, copy.id).await.expect("reading").len(),
        2
    );
    sandbox.close().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn another_accounts_list_cannot_be_copied_but_ones_own_can() {
    let sandbox = Sandbox::open("copy_own").await;
    let db = &sandbox.db;
    let alice = sandbox.person("Alice").await;
    let bob = sandbox.person("Bob").await;
    let list = made(db, alice, "Secret").await;
    assert_eq!(
        banlists::create(db, bob, "Stolen", "modern", Some(list.id))
            .await
            .expect("asking"),
        Created::NoSuchList
    );
    assert!(matches!(
        banlists::create(db, alice, "Second", "modern", Some(list.id))
            .await
            .expect("asking"),
        Created::Made(_)
    ));
    sandbox.close().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_version_moves_with_the_verdicts_and_not_with_the_name() {
    let sandbox = Sandbox::open("version").await;
    let db = &sandbox.db;
    let alice = sandbox.person("Alice").await;
    let list = made(db, alice, "Kitchen table").await;

    let Changed::Made(renamed) = banlists::edit(
        db,
        list.id,
        alice,
        &Edit {
            name: Some("Friday".to_owned()),
            ..Edit::default()
        },
    )
    .await
    .expect("editing") else {
        panic!("a rename");
    };
    assert_eq!(renamed.name, "Friday");
    assert_eq!(renamed.version, 1, "a name changes no verdict");

    let cards = vec![entry(1, Status::BannedAsCommander)];
    let change = Edit {
        entries: Some(cards.clone()),
        ..Edit::default()
    };
    let Changed::Made(edited) = banlists::edit(db, list.id, alice, &change)
        .await
        .expect("editing")
    else {
        panic!("an edit");
    };
    assert_eq!(edited.version, 2);
    assert_eq!(edited.cards, 1);
    let Changed::Made(again) = banlists::edit(db, list.id, alice, &change)
        .await
        .expect("editing")
    else {
        panic!("an edit");
    };
    assert_eq!(again.version, 2, "the same cards are no new version");
    assert_eq!(
        banlists::entries(db, list.id).await.expect("reading"),
        cards
    );

    let Changed::Made(moved) = banlists::edit(
        db,
        list.id,
        alice,
        &Edit {
            format: Some("legacy".to_owned()),
            ..Edit::default()
        },
    )
    .await
    .expect("editing") else {
        panic!("an edit");
    };
    assert_eq!(moved.version, 3, "another format is another verdict");
    sandbox.close().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_feed_read_twice_is_no_new_version() {
    let sandbox = Sandbox::open("sync").await;
    let db = &sandbox.db;
    let alice = sandbox.person("Alice").await;
    let cards = [entry(2, Status::Banned), entry(1, Status::Restricted)];
    assert!(
        banlists::sync_official(db, "legacy", &cards)
            .await
            .expect("syncing")
    );
    let shuffled = [cards[1], cards[0]];
    assert!(
        !banlists::sync_official(db, "legacy", &shuffled)
            .await
            .expect("syncing"),
        "the same cards in another order move nothing"
    );
    let list = banlists::listed(db, alice).await.expect("listing")[0].clone();
    assert_eq!(list.version, 2, "created at 1, its cards written once");
    assert!(
        banlists::sync_official(db, "legacy", &[])
            .await
            .expect("syncing"),
        "a ban lifted is a change"
    );
    let list = banlists::listed(db, alice).await.expect("listing")[0].clone();
    assert_eq!(list.version, 3);
    assert_eq!(list.cards, 0);
    assert_eq!(
        banlists::listed(db, alice).await.expect("listing").len(),
        1,
        "one official list a format"
    );
    sandbox.close().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_list_has_limits() {
    let sandbox = Sandbox::open("limits").await;
    let db = &sandbox.db;
    let alice = sandbox.person("Alice").await;
    for n in 0..MAX_LISTS_PER_ACCOUNT {
        made(db, alice, &format!("List {n}")).await;
    }
    assert_eq!(
        banlists::create(db, alice, "One more", "modern", None)
            .await
            .expect("asking"),
        Created::TooMany
    );
    let list = banlists::listed(db, alice).await.expect("listing")[0].clone();
    let oversize: Vec<Entry> = (0..=MAX_LIST_ENTRIES)
        .map(|n| entry(n as u128 + 1, Status::Banned))
        .collect();
    assert_eq!(
        banlists::edit(
            db,
            list.id,
            alice,
            &Edit {
                entries: Some(oversize),
                ..Edit::default()
            }
        )
        .await
        .expect("asking"),
        Changed::TooManyCards
    );
    let bob = sandbox.person("Bob").await;
    assert!(
        banlists::create(db, bob, "", "modern", None).await.is_err(),
        "the table holds the name's bounds"
    );
    sandbox.close().await;
}

/// Closing an account takes its lists and leaves the official ones standing.
#[tokio::test(flavor = "multi_thread")]
async fn an_accounts_lists_go_with_it() {
    let sandbox = Sandbox::open("cascade").await;
    let db = &sandbox.db;
    let alice = sandbox.person("Alice").await;
    banlists::sync_official(db, "modern", &[entry(9, Status::Banned)])
        .await
        .expect("syncing");
    let list = made(db, alice, "Kitchen table").await;
    banlists::edit(
        db,
        list.id,
        alice,
        &Edit {
            entries: Some(vec![entry(1, Status::Banned)]),
            ..Edit::default()
        },
    )
    .await
    .expect("editing");
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "DELETE FROM account WHERE id = $1",
        [alice.into()],
    ))
    .await
    .expect("deleting the account");
    assert_eq!(
        sandbox
            .count("SELECT count(*) AS n FROM banlists WHERE NOT builtin")
            .await,
        0
    );
    assert_eq!(
        sandbox
            .count("SELECT count(*) AS n FROM banlist_entries")
            .await,
        1,
        "the official list's card stays"
    );
    sandbox.close().await;
}
