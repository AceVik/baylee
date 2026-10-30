//! What only a real server can answer.
//!
//! Three things in this crate are claims about PostgreSQL rather than about
//! Rust, and no amount of unit testing reaches any of them: that the
//! migration is SQL the server accepts, that the two `lower(...)` indexes
//! actually refuse a second account whose address differs only in case, and
//! that deleting an account takes its decks, sessions and settings with it.
//!
//! Each test runs in a **schema of its own**, created and dropped around it,
//! so a suite of them can run against one database at once and so nothing
//! here can touch the catalog's tables in `public` — which on a developer's
//! machine hold half a gigabyte of ingested card text that took three minutes
//! to write.
//!
//! Without `DATABASE_URL` they fail rather than skip. A skipped test that
//! reports success is worse than a missing one: the gateway keeps its
//! accounts in PostgreSQL now, so a green suite with no server would be
//! saying that a schema works when nothing had asked it to do anything.

#![forbid(unsafe_code)]

use baylee_db::entity::prelude::*;
use baylee_db::entity::{
    account, client_settings, confirmation, deck, deck_version, session_token, upload,
};
use baylee_db::invites;
use baylee_db::migration::Migrator;
use baylee_db::precons::{self, Precon};
use sea_orm::{
    ActiveValue::{NotSet, Set},
    ColumnTrait, ConnectionTrait, Database, DatabaseConnection, DbBackend, EntityTrait,
    PaginatorTrait, QueryFilter, Statement,
};
use sea_orm_migration::MigratorTrait as _;
use time::OffsetDateTime;
use uuid::Uuid;

/// A database to run against, and the schema to run in.
struct Sandbox {
    db: DatabaseConnection,
    admin: DatabaseConnection,
    schema: String,
    scoped: String,
}

impl Sandbox {
    /// Make a fresh schema and migrate it.
    async fn open(what: &str) -> Self {
        Self::open_at(what, None).await
    }

    /// Make a fresh schema and apply the first `applied` migrations, or all
    /// of them. A migration that changes rows is tested from the schema
    /// before it, holding the rows it has to change.
    async fn open_at(what: &str, applied: Option<u32>) -> Self {
        let url = std::env::var("DATABASE_URL")
            .ok()
            .filter(|u| !u.is_empty())
            .unwrap_or_else(|| {
                panic!(
                    "DATABASE_URL is not set, and these tests are about PostgreSQL.\n  \
                     docker compose up -d\n  \
                     export DATABASE_URL=postgres://baylee:baylee@127.0.0.1:5432/baylee"
                )
            });

        // The name carries the test's own, so a schema left behind by a
        // panic says which test abandoned it.
        let schema: String = format!("t_{what}_{}", Uuid::now_v7().simple())
            .chars()
            .take(63)
            .collect();

        let admin = Database::connect(&url).await.expect("connecting");
        admin
            .execute_unprepared(&format!("CREATE SCHEMA \"{schema}\""))
            .await
            .expect("creating a schema");

        // `public` stays on the path behind it: the catalog's tables live
        // there and an unqualified query for one would otherwise stop
        // finding it.
        let sep = if url.contains('?') { '&' } else { '?' };
        let scoped = format!("{url}{sep}options=-c%20search_path%3D{schema},public");
        let db = match applied {
            None => baylee_db::connect(&scoped, 2)
                .await
                .expect("migrating a fresh schema"),
            Some(steps) => {
                let db = Database::connect(&scoped).await.expect("connecting");
                Migrator::up(&db, Some(steps))
                    .await
                    .expect("migrating part of the way");
                db
            }
        };

        Self {
            db,
            admin,
            schema,
            scoped,
        }
    }

    /// Give the schema back.
    async fn close(self) {
        self.admin
            .execute_unprepared(&format!("DROP SCHEMA \"{}\" CASCADE", self.schema))
            .await
            .expect("dropping the schema");
    }
}

/// One account, ready to insert.
fn an_account(email: &str) -> account::ActiveModel {
    named(email, email.split('@').next().unwrap_or(email))
}

/// One account under a name of its own, for the tests about what a name is
/// allowed to be.
fn named(email: &str, display_name: &str) -> account::ActiveModel {
    account::ActiveModel {
        id: Set(Uuid::now_v7()),
        email: Set(Some(email.to_owned())),
        username: Set(None),
        username_key: Set(None),
        display_name: Set(display_name.to_owned()),
        // The database hands this out. Setting it here would be the one
        // way to collide with a later registration, because an explicit
        // value does not move the sequence.
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
}

/// The migration is SQL the server takes, and running it twice is not an
/// error — which is what a gateway restarting against its own database does
/// every time.
#[tokio::test]
async fn the_schema_applies_and_reapplies() {
    let sandbox = Sandbox::open("migrate").await;

    let again = baylee_db::connect(&sandbox.scoped, 2)
        .await
        .expect("a second start against the same schema is not a migration");
    drop(again);

    assert_eq!(
        Account::find().count(&sandbox.db).await.unwrap(),
        0,
        "a fresh schema has no accounts in it"
    );

    sandbox.close().await;
}

#[tokio::test]
async fn the_legacy_export_takes_unmoved_password_accounts_and_no_secrets() {
    use baylee_db::central;
    let sandbox = Sandbox::open("central_legacy_export").await;
    let (alice, bob, _) = central_seed(&sandbox.db).await;
    let ban = |until: Option<i64>, id: Uuid| {
        Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE account SET banned_at = clock_timestamp(), ban_reason = 'synthetic',
             banned_by = 'test', banned_until = to_timestamp($1::bigint) WHERE id = $2",
            [until.into(), id.into()],
        )
    };
    sandbox.db.execute_raw(ban(None, bob)).await.unwrap();
    // A lapsed ban is no ban; a guest, a projection and a moved account are
    // not legacy accounts.
    let lapsed = an_account("central-c@example.invalid");
    let Set(lapsed_id) = lapsed.id else {
        unreachable!()
    };
    Account::insert(lapsed).exec(&sandbox.db).await.unwrap();
    sandbox
        .db
        .execute_raw(ban(Some(1), lapsed_id))
        .await
        .unwrap();
    let mut guest = an_account("central-guest@example.invalid");
    guest.email = Set(None);
    guest.password_hash = Set(None);
    guest.guest = Set(true);
    Account::insert(guest).exec(&sandbox.db).await.unwrap();
    central::projection(&sandbox.db, *Uuid::now_v7().as_bytes(), true)
        .await
        .unwrap();
    let moved = an_account("central-moved@example.invalid");
    let Set(moved_id) = moved.id else {
        unreachable!()
    };
    Account::insert(moved).exec(&sandbox.db).await.unwrap();
    central::adopt(
        &sandbox.db,
        &moved_id.to_string(),
        *Uuid::now_v7().as_bytes(),
    )
    .await
    .unwrap();
    let accounts = central::legacy_accounts(&sandbox.db).await.unwrap();
    let mut expected = vec![alice.to_string(), bob.to_string(), lapsed_id.to_string()];
    expected.sort();
    assert_eq!(
        accounts.iter().map(|a| a.id.clone()).collect::<Vec<_>>(),
        expected,
        "the unmoved password accounts, by id"
    );
    let of = |id: Uuid| accounts.iter().find(|a| a.id == id.to_string()).unwrap();
    let text = of(alice).snapshot.as_deref().unwrap();
    assert!(text.contains("ALICE_PRIVATE_DECK"));
    assert!(!text.contains("BOB_PRIVATE_DECK"));
    assert!(!text.contains("password_hash") && !text.contains("token_hash"));
    let snapshot: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(snapshot["decks"][0]["history"].as_array().unwrap().len(), 1);
    assert_eq!(of(alice).ban, None);
    assert_eq!(of(bob).ban, Some(central::Ban { until: None }));
    assert_eq!(of(lapsed_id).ban, None);
    assert!(of(alice).password_hash.starts_with("$argon2id$"));
    // A source larger than the first snapshot format is named, not cut.
    ClientSettings::insert(client_settings::ActiveModel {
        account_id: Set(alice),
        doc: Set(serde_json::json!({"padding": "x".repeat(200_000)})),
        updated_at: Set(OffsetDateTime::now_utc()),
    })
    .exec(&sandbox.db)
    .await
    .unwrap();
    let accounts = central::legacy_accounts(&sandbox.db).await.unwrap();
    let alice_now = accounts.iter().find(|a| a.id == alice.to_string()).unwrap();
    assert!(alice_now.snapshot.is_none());
    sandbox.close().await;
}

#[tokio::test]
async fn a_moved_account_is_linked_at_its_first_admission_and_only_once() {
    use baylee_db::central;
    let sandbox = Sandbox::open("central_adopt").await;
    let (alice_id, bob_id, deck_id) = central_seed(&sandbox.db).await;
    let alice = alice_id.to_string();
    let central_id = *Uuid::now_v7().as_bytes();
    assert!(central::adopt(&sandbox.db, &alice, [0; 16]).await.is_err());
    assert!(
        central::adopt(&sandbox.db, &Uuid::now_v7().to_string(), central_id)
            .await
            .is_err(),
        "no such account"
    );
    assert!(!central::linked(&sandbox.db, &alice).await.unwrap());
    assert!(
        central::adopt(&sandbox.db, &alice, central_id)
            .await
            .unwrap()
    );
    assert!(central::linked(&sandbox.db, &alice).await.unwrap());
    assert_eq!(
        SessionToken::find().count(&sandbox.db).await.unwrap(),
        0,
        "its password sessions end with the link"
    );
    // The same link again is nothing new and ends nothing.
    SessionToken::insert(session_token::ActiveModel {
        token_hash: Set(vec![43; 32]),
        account_id: Set(alice_id),
        expires_at: Set(OffsetDateTime::now_utc() + time::Duration::days(1)),
    })
    .exec(&sandbox.db)
    .await
    .unwrap();
    assert!(
        !central::adopt(&sandbox.db, &alice, central_id)
            .await
            .unwrap()
    );
    assert_eq!(SessionToken::find().count(&sandbox.db).await.unwrap(), 1);
    assert!(
        central::adopt(&sandbox.db, &alice, *Uuid::now_v7().as_bytes())
            .await
            .is_err(),
        "a link is immutable"
    );
    assert!(
        central::adopt(&sandbox.db, &bob_id.to_string(), central_id)
            .await
            .is_err(),
        "an identity is linked to one account"
    );
    let mut guest = an_account("central-guest@example.invalid");
    guest.email = Set(None);
    guest.password_hash = Set(None);
    guest.guest = Set(true);
    let Set(guest_id) = guest.id else {
        unreachable!()
    };
    Account::insert(guest).exec(&sandbox.db).await.unwrap();
    assert!(
        central::adopt(
            &sandbox.db,
            &guest_id.to_string(),
            *Uuid::now_v7().as_bytes()
        )
        .await
        .is_err()
    );
    // The game data stays; the projection and the session issue under it.
    assert!(
        Deck::find_by_id(deck_id)
            .one(&sandbox.db)
            .await
            .unwrap()
            .is_some()
    );
    central::issue_session(
        &sandbox.db,
        &alice,
        vec![44; 32],
        [45; 32],
        OffsetDateTime::now_utc().unix_timestamp() + 60,
        "admit",
    )
    .await
    .unwrap();
    central_password_retired(&sandbox.db, alice_id, bob_id).await;
    sandbox.close().await;
}

#[tokio::test]
async fn a_projection_takes_its_name_and_tag_from_central() {
    use baylee_db::central;
    let sandbox = Sandbox::open("central_projection_profile").await;
    let row = |id: String| {
        let db = sandbox.db.clone();
        async move {
            let row = Account::find_by_id(Uuid::parse_str(&id).unwrap())
                .one(&db)
                .await
                .unwrap()
                .unwrap();
            (row.display_name, row.tag)
        }
    };
    let floo = central::projection(&sandbox.db, *Uuid::now_v7().as_bytes(), true)
        .await
        .unwrap();
    let other = central::projection(&sandbox.db, *Uuid::now_v7().as_bytes(), true)
        .await
        .unwrap();
    let (name, drawn) = row(floo.clone()).await;
    assert_eq!(name, "Player");
    // Central's name and tag replace the local ones, and taking them again
    // writes nothing.
    assert!(
        central::take_profile(&sandbox.db, &floo, Some("floo"), Some(0x1337))
            .await
            .unwrap()
    );
    assert_eq!(row(floo.clone()).await, ("floo".into(), 0x1337));
    assert!(
        !central::take_profile(&sandbox.db, &floo, Some("floo"), Some(0x1337))
            .await
            .unwrap()
    );
    // What Central does not say keeps the local value.
    assert!(
        central::take_profile(&sandbox.db, &floo, None, None)
            .await
            .is_ok()
    );
    assert_eq!(row(floo.clone()).await, ("floo".into(), 0x1337));
    // A tag another account holds here is not taken; the name still is.
    let (_, held) = row(other.clone()).await;
    assert!(
        central::take_profile(&sandbox.db, &floo, Some("floo2"), Some(held))
            .await
            .unwrap()
    );
    assert_eq!(row(floo.clone()).await, ("floo2".into(), 0x1337));
    assert_ne!(drawn, 0x1337);
    sandbox.close().await;
}

#[tokio::test]
async fn central_gateway_projection_is_immutable_and_sessions_require_a_bound_live_lease() {
    use baylee_db::central::{self, SessionBinding};
    let sandbox = Sandbox::open("central_projection").await;
    let central = *Uuid::now_v7().as_bytes();
    assert!(
        central::projection(&sandbox.db, [0; 16], true)
            .await
            .is_err()
    );
    assert!(
        central::projection(&sandbox.db, central, false)
            .await
            .is_err()
    );
    let account = central::projection(&sandbox.db, central, true)
        .await
        .unwrap();
    assert_eq!(
        central::projection(&sandbox.db, central, false)
            .await
            .unwrap(),
        account
    );
    let other = central::projection(&sandbox.db, *Uuid::now_v7().as_bytes(), true)
        .await
        .unwrap();
    assert_ne!(account, other);
    let row = Account::find_by_id(Uuid::parse_str(&account).unwrap())
        .one(&sandbox.db)
        .await
        .unwrap()
        .unwrap();
    assert!(row.username.is_none() && row.email.is_none() && row.password_hash.is_none());
    let expiry = OffsetDateTime::now_utc().unix_timestamp() + 60;
    central::issue_session(&sandbox.db, &account, vec![8; 32], [9; 32], expiry, "admit")
        .await
        .unwrap();
    match central::session_binding(&sandbox.db, vec![8; 32])
        .await
        .unwrap()
    {
        SessionBinding::Central(lease) => {
            assert_eq!(lease.central, central);
            assert_eq!(lease.lease, [9; 32]);
            assert_eq!(lease.expires_at, expiry);
        }
        _ => panic!("a Central session needs its matching lease"),
    }
    assert!(
        central::issue_session(&sandbox.db, &account, vec![10; 32], [11; 32], 1, "admit")
            .await
            .is_err()
    );
    assert_eq!(
        SessionToken::find().count(&sandbox.db).await.unwrap(),
        1,
        "expired issuance leaves no ordinary bearer behind"
    );
    sandbox
        .db
        .execute_unprepared("DELETE FROM central_gateway_session")
        .await
        .unwrap();
    assert!(matches!(
        central::session_binding(&sandbox.db, vec![8; 32])
            .await
            .unwrap(),
        SessionBinding::Rejected
    ));
    assert!(sandbox.db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "UPDATE central_account_link SET central_id = '00000000-0000-0000-0000-000000000000' WHERE account_id = $1",
        [Uuid::parse_str(&account).unwrap().into()])).await.is_err(), "SQL NULL from uuid_extract_version must not pass the CHECK");
    sandbox.close().await;
}

#[tokio::test]
async fn central_password_retirement_upgrades_existing_links_without_touching_legacy_accounts() {
    let before = u32::try_from(
        Migrator::migrations()
            .iter()
            .position(|migration| migration.name() == "m20261019_000021_retire_legacy_passwords")
            .expect("password retirement migration registered"),
    )
    .expect("a handful");
    let sandbox = Sandbox::open_at("central_retire_existing", Some(before)).await;
    let db = &sandbox.db;
    let (alice, bob, deck) = central_seed(db).await;
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO central_account_link (account_id, central_id, snapshot_digest, linked_at)
         VALUES ($1, uuidv7(), $2, clock_timestamp())",
        [alice.into(), vec![7_u8; 32].into()],
    ))
    .await
    .unwrap();
    assert!(
        Account::find_by_id(alice)
            .one(db)
            .await
            .unwrap()
            .unwrap()
            .password_hash
            .is_some()
    );
    Migrator::up(db, None).await.unwrap();
    central_password_retired(db, alice, bob).await;
    assert_eq!(
        Deck::find_by_id(deck)
            .one(db)
            .await
            .unwrap()
            .unwrap()
            .account_id,
        Some(alice)
    );
    Migrator::up(db, None).await.unwrap();
    central_password_retired(db, alice, bob).await;
    sandbox.close().await;
}

async fn central_password_retired(db: &DatabaseConnection, alice: Uuid, bob: Uuid) {
    assert!(
        Account::find_by_id(alice)
            .one(db)
            .await
            .unwrap()
            .unwrap()
            .password_hash
            .is_none()
    );
    assert!(
        Account::find_by_id(bob)
            .one(db)
            .await
            .unwrap()
            .unwrap()
            .password_hash
            .is_some()
    );
    assert!(
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE account SET password_hash = 'synthetic-disabled-password' WHERE id = $1",
            [alice.into()]
        ))
        .await
        .is_err(),
        "an old gateway reset must not restore password authentication"
    );
    // Every later migration rolls back; the one that bound the migration
    // credentials, and with it the retirement, does not.
    let bound = Migrator::migrations()
        .iter()
        .position(|migration| migration.name() == "m20261019_000023_bind_migration_credentials")
        .unwrap();
    let later = Migrator::migrations().len() - 1 - bound;
    if later > 0 {
        Migrator::down(db, Some(u32::try_from(later).unwrap()))
            .await
            .unwrap();
    }
    assert!(
        Migrator::down(db, Some(1)).await.is_err(),
        "retired credentials cannot be rolled back"
    );
    assert!(
        baylee_db::central::linked(db, &alice.to_string())
            .await
            .unwrap()
    );
}

async fn central_seed(db: &DatabaseConnection) -> (Uuid, Uuid, Uuid) {
    let alice = an_account("central-a@example.invalid");
    let Set(alice_id) = alice.id else {
        unreachable!()
    };
    Account::insert(alice).exec(db).await.unwrap();
    let bob = an_account("central-b@example.invalid");
    let Set(bob_id) = bob.id else { unreachable!() };
    Account::insert(bob).exec(db).await.unwrap();
    let deck = a_deck(Some(alice_id), "ALICE_PRIVATE_DECK");
    let deck_id = Deck::insert(deck).exec(db).await.unwrap().last_insert_id;
    Deck::insert(a_deck(Some(bob_id), "BOB_PRIVATE_DECK"))
        .exec(db)
        .await
        .unwrap();
    DeckVersion::insert(a_version(deck_id, 1, &["1 Forest"]))
        .exec(db)
        .await
        .unwrap();
    SessionToken::insert(session_token::ActiveModel {
        token_hash: Set(vec![42; 32]),
        account_id: Set(alice_id),
        expires_at: Set(OffsetDateTime::now_utc() + time::Duration::days(1)),
    })
    .exec(db)
    .await
    .unwrap();
    (alice_id, bob_id, deck_id)
}

/// Whether `schema` holds a table called `table`, asked with both names
/// spelled out so the search path has no say in the answer.
async fn has_table(db: &DatabaseConnection, schema: &str, table: &str) -> bool {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT count(*) AS n FROM information_schema.tables \
             WHERE table_schema = $1 AND table_name = $2",
            [schema.into(), table.into()],
        ))
        .await
        .expect("asking for a table")
        .expect("a count always comes back");
    row.try_get::<i64>("", "n").expect("a count") == 1
}

/// The standing answers' table goes from the schema being migrated and from
/// nowhere else, and stepping back past that migration builds it again.
///
/// A bare `DROP TABLE IF EXISTS` walks the search path: in a schema without
/// such a table it drops the next one it finds, which in a test sandbox is
/// the developer's `public`. So a table of that name is planted in a schema
/// behind this one, and the sandbox loses its own by hand first — the one
/// state in which a bare name would reach past it.
#[tokio::test]
async fn the_standing_answers_go_only_from_the_schema_being_migrated() {
    use baylee_db::migration::Migrator;
    use sea_orm_migration::MigratorTrait as _;

    let sandbox = Sandbox::open("answers").await;
    let here = sandbox.schema.clone();
    let behind = format!("{here}_behind");
    assert!(
        !has_table(&sandbox.admin, &here, "standing_answer").await,
        "a fresh schema still has the standing answers"
    );

    // Its own connection, with the second schema on the path and no
    // migration run on connect: `baylee_db::connect` would migrate.
    let path = sandbox.scoped.replace(
        &format!("{here},public"),
        &format!("{here},{behind},public"),
    );
    assert_ne!(path, sandbox.scoped, "the second schema went onto the path");
    let db = Database::connect(&path).await.expect("connecting behind");

    // Back past the drop and every migration after it, counted from the list
    // rather than assumed to be the last one: it was, until #269.
    let migrations = Migrator::migrations();
    let drop_at = migrations
        .iter()
        .position(|m| m.name() == "m20260924_000005_drop_standing_answer")
        .expect("the drop is in the list");
    let back = u32::try_from(migrations.len() - drop_at).expect("a handful");
    Migrator::down(&db, Some(back))
        .await
        .expect("stepping back past the drop");
    assert!(
        has_table(&sandbox.admin, &here, "standing_answer").await,
        "stepping back did not build the table again"
    );

    sandbox
        .admin
        .execute_unprepared(&format!(
            "CREATE SCHEMA \"{behind}\"; \
             CREATE TABLE \"{behind}\".standing_answer (card bigint); \
             DROP TABLE \"{here}\".standing_answer"
        ))
        .await
        .expect("planting the table behind and losing this one");

    Migrator::up(&db, None)
        .await
        .expect("migrating forward again");
    assert!(!has_table(&sandbox.admin, &here, "standing_answer").await);
    assert!(
        has_table(&sandbox.admin, &behind, "standing_answer").await,
        "the drop reached past the schema it was migrating"
    );

    drop(db);
    sandbox
        .admin
        .execute_unprepared(&format!("DROP SCHEMA \"{behind}\" CASCADE"))
        .await
        .expect("dropping the schema behind");
    sandbox.close().await;
}

/// Two addresses that differ only in case are one address. The index is on
/// `lower(email)`, so this is the only thing that proves the expression index
/// was created rather than a plain one that would have let the row through.
#[tokio::test]
async fn an_address_is_taken_whatever_its_case() {
    let sandbox = Sandbox::open("case").await;

    Account::insert(an_account("Player@Example.com"))
        .exec(&sandbox.db)
        .await
        .expect("the first account registers");

    let second = Account::insert(an_account("player@EXAMPLE.COM"))
        .exec(&sandbox.db)
        .await;
    assert!(
        second.is_err(),
        "the same address in a different case registered a second time"
    );

    sandbox.close().await;
}

/// The key is the database's, and it is a `UUIDv7`.
///
/// `Uuid::now_v7()` in the gateway would be ordered by *that gateway's*
/// clock. Several gateways behind one database each write at their own
/// right-hand edge and between them scatter the b-tree exactly the way a
/// `UUIDv4` would — which is the whole reason the key is a v7 at all. One
/// database has one clock.
#[tokio::test]
async fn postgres_mints_the_keys() {
    let sandbox = Sandbox::open("mint").await;

    let first = Account::insert(an_account("minted@example.com"))
        .exec_with_returning(&sandbox.db)
        .await
        .expect("registering without an id");
    let second = Account::insert(an_account("later@example.com"))
        .exec_with_returning(&sandbox.db)
        .await
        .expect("registering again");

    assert_eq!(first.id.get_version_num(), 7, "{}", first.id);
    assert_eq!(second.id.get_version_num(), 7, "{}", second.id);
    assert!(
        first.id < second.id,
        "two keys minted in order must sort in order: {} then {}",
        first.id,
        second.id
    );

    // A deck's key comes the same way, and hangs off the account that was
    // just minted one.
    let deck = Deck::insert(deck::ActiveModel {
        id: NotSet,
        account_id: Set(Some(first.id)),
        kind: Set(deck::KIND_ACCOUNT.to_owned()),
        format: Set("freeform".to_owned()),
        description: Set(None),
        copied_from: Set(None),
        copied_version: Set(None),
        version: Set(1),
        name: Set("Mono Red".to_owned()),
        cards: Set(vec!["4 Lightning Bolt".to_owned()]),
        sideboard: Set(Vec::new()),
        commanders: Set(Vec::new()),
        sleeve: Set(None),
        playmat: Set(None),
        updated_at: Set(OffsetDateTime::now_utc()),
        source: Set(None),
        offered: Set(true),
    })
    .exec_with_returning(&sandbox.db)
    .await
    .expect("a deck saves without an id");
    assert_eq!(deck.id.get_version_num(), 7, "{}", deck.id);

    // And a default is not a prohibition: the importer carries the ids an
    // older store already handed out.
    let carried = Uuid::now_v7();
    let mut brought = an_account("imported@example.com");
    brought.id = Set(carried);
    let kept = Account::insert(brought)
        .exec_with_returning(&sandbox.db)
        .await
        .expect("an imported account keeps its id");
    assert_eq!(kept.id, carried);

    sandbox.close().await;
}

/// The nine house decks are in every database, and they belong to nobody.
///
/// A migration that seeds is a migration whose data is part of the schema:
/// there is no file to forget to load and no first-run step to skip, which
/// is the whole reason they are here rather than in a `.json` beside the
/// binary.
#[tokio::test]
async fn every_database_comes_with_the_house_decks() {
    let sandbox = Sandbox::open("house").await;

    let house = Deck::find()
        .filter(deck::Column::Kind.eq(deck::KIND_HOUSE))
        .all(&sandbox.db)
        .await
        .expect("reading the house decks");
    assert_eq!(house.len(), 9, "nine decks ship with the schema");

    for seeded in &house {
        assert!(
            seeded.account_id.is_none(),
            "{} belongs to somebody",
            seeded.name
        );
        // A hundred cards either way, which is the thing worth asserting:
        // Commander is 99 plus the one in the command zone (CR 903.5a) and
        // Highlander is 100 with nobody in front of it, so the count alone
        // says nothing until the format is read beside it.
        // A leader is stored bare — `decks::by_name` is an exact-spelling
        // lookup — and its own row carries the printing, so it is counted
        // here only when no row names it. `decks::from_lines` moves a named
        // row out of the library rather than copying it, so a deck whose
        // commander is among its rows is still a hundred cards and not 101.
        let seats_a_leader = |leader: &String| {
            let bare = format!("1 {leader}");
            let printed = format!("{bare} (");
            seeded
                .cards
                .iter()
                .any(|row| row == &bare || row.starts_with(&printed))
        };
        for leader in &seeded.commanders {
            assert!(
                !leader.starts_with(|c: char| c.is_ascii_digit()),
                "{}: leader {leader} is written as a deck row",
                seeded.name
            );
        }
        let leaders = seeded
            .commanders
            .iter()
            .filter(|leader| !seats_a_leader(leader))
            .count();
        // Rows are not cards: a deck stores `"N Card Name"` lines, so four
        // Forests are one row. Counting rows here reads a hundred-card deck
        // as ninety-seven and would have to be loosened into saying nothing.
        let cards: usize = seeded
            .cards
            .iter()
            .map(|row| {
                row.split_once(' ')
                    .and_then(|(n, _)| n.parse::<usize>().ok())
                    .unwrap_or_else(|| panic!("{}: {row} has no count", seeded.name))
            })
            .sum();
        assert_eq!(
            cards + leaders,
            100,
            "{} plays {cards} cards and {leaders} commander(s) outside them",
            seeded.name
        );
        match seeded.format.as_str() {
            "commander" => assert_eq!(
                seeded.commanders.len(),
                1,
                "{} leads with nobody",
                seeded.name
            ),
            "highlander" => assert!(
                seeded.commanders.is_empty(),
                "{} has a commander",
                seeded.name
            ),
            other => panic!("{} plays {other}", seeded.name),
        }
        // The four engine decks are named after the commander they play;
        // the four real ones are named by their owners, so what holds across
        // all eight is only that a commander is a card and not an empty
        // string. Asserting the stronger rule here would be asserting a
        // coincidence of the first four.
        for leader in &seeded.commanders {
            assert!(
                !leader.trim().is_empty(),
                "{} leads with a blank",
                seeded.name
            );
        }
        assert!(
            seeded.description.is_some(),
            "{} says what it is for",
            seeded.name
        );
        assert_eq!(seeded.version, 1, "nobody has changed it yet");
    }

    // The counter-half: a player asking for *their* decks sees none of them,
    // which is what `account_id IS NULL` is for.
    let mine = Deck::find()
        .filter(deck::Column::AccountId.eq(Uuid::now_v7()))
        .count(&sandbox.db)
        .await
        .expect("counting one account's decks");
    assert_eq!(mine, 0, "a house deck is nobody's deck");

    sandbox.close().await;
}

/// A deck's owner and its kind are one statement, and the database holds
/// both halves of it.
///
/// Either way round is a real accident: a house deck with an owner hands one
/// player a deck everybody is meant to share, and an account deck with none
/// is a deck that survives `DELETE FROM account`.
#[tokio::test]
async fn a_deck_that_belongs_to_nobody_must_say_so() {
    let sandbox = Sandbox::open("kindcheck").await;
    let account = an_account("owner@example.com");
    let Set(id) = account.id else { unreachable!() };
    Account::insert(account).exec(&sandbox.db).await.unwrap();

    let mut orphan = a_deck(None, "nobody's");
    orphan.kind = Set(deck::KIND_ACCOUNT.to_owned());
    assert!(
        Deck::insert(orphan).exec(&sandbox.db).await.is_err(),
        "an account deck with no account was accepted"
    );

    let mut claimed = a_deck(Some(id), "everybody's");
    claimed.kind = Set(deck::KIND_HOUSE.to_owned());
    assert!(
        Deck::insert(claimed).exec(&sandbox.db).await.is_err(),
        "a house deck with an owner was accepted"
    );

    let mut invented = a_deck(Some(id), "mine");
    invented.kind = Set("legendary".to_owned());
    assert!(
        Deck::insert(invented).exec(&sandbox.db).await.is_err(),
        "a kind nobody has heard of was accepted"
    );

    // And the two shapes that are right still go in.
    Deck::insert(a_deck(Some(id), "mine"))
        .exec(&sandbox.db)
        .await
        .expect("an account deck with an owner");
    let mut published = a_deck(None, "everybody's");
    published.kind = Set(deck::KIND_PRECONSTRUCTED.to_owned());
    Deck::insert(published)
        .exec(&sandbox.db)
        .await
        .expect("a preconstructed deck with no owner");

    sandbox.close().await;
}

/// The history is the deck's past, and only one row may ever claim to be a
/// given version of it.
///
/// That is the primary key doing the work, and it is the difference between
/// two saves racing and a forked history nobody can read back.
#[tokio::test]
async fn a_version_number_belongs_to_one_state() {
    let sandbox = Sandbox::open("history").await;
    let account = an_account("historian@example.com");
    let Set(owner) = account.id else {
        unreachable!()
    };
    Account::insert(account).exec(&sandbox.db).await.unwrap();

    let deck = Deck::insert(a_deck(Some(owner), "in progress"))
        .exec_with_returning(&sandbox.db)
        .await
        .expect("a deck saves");

    DeckVersion::insert(a_version(deck.id, 1, &["4 Lightning Bolt"]))
        .exec(&sandbox.db)
        .await
        .expect("the state it used to hold");
    assert!(
        DeckVersion::insert(a_version(deck.id, 1, &["4 Shock"]))
            .exec(&sandbox.db)
            .await
            .is_err(),
        "a second version 1 was accepted, and the history has forked"
    );
    DeckVersion::insert(a_version(deck.id, 2, &["4 Shock"]))
        .exec(&sandbox.db)
        .await
        .expect("the next version is a different number");

    // A deleted deck takes its past with it: history that outlived its deck
    // would be rows nothing can ever name again.
    Deck::delete_by_id(deck.id)
        .exec(&sandbox.db)
        .await
        .expect("deleting the deck");
    let left = DeckVersion::find()
        .count(&sandbox.db)
        .await
        .expect("counting what is left");
    assert_eq!(left, 0, "the deck's history outlived the deck");

    sandbox.close().await;
}

/// A deck, ready to insert.
fn a_deck(owner: Option<Uuid>, name: &str) -> deck::ActiveModel {
    deck::ActiveModel {
        id: NotSet,
        account_id: Set(owner),
        kind: Set(deck::KIND_ACCOUNT.to_owned()),
        name: Set(name.to_owned()),
        format: Set("freeform".to_owned()),
        description: Set(None),
        copied_from: Set(None),
        copied_version: Set(None),
        version: Set(1),
        cards: Set(vec!["4 Lightning Bolt".to_owned()]),
        sideboard: Set(Vec::new()),
        commanders: Set(Vec::new()),
        sleeve: Set(None),
        playmat: Set(None),
        updated_at: Set(OffsetDateTime::now_utc()),
        source: Set(None),
        offered: Set(true),
    }
}

/// One superseded state, ready to insert.
fn a_version(deck_id: Uuid, version: i32, cards: &[&str]) -> deck_version::ActiveModel {
    deck_version::ActiveModel {
        deck_id: Set(deck_id),
        version: Set(version),
        cards: Set(cards.iter().map(|c| (*c).to_owned()).collect()),
        sideboard: Set(Vec::new()),
        commanders: Set(Vec::new()),
        summary: Set(None),
        superseded_at: Set(OffsetDateTime::now_utc()),
    }
}

/// A display name is not a claim. Two players may both be Alice, and what
/// tells them apart is the number the database hands out.
///
/// This is the test that would have failed before `account.tag` existed —
/// a `unique (lower(display_name))` index refused the second Alice — and it
/// is the whole reason the column is here.
#[tokio::test]
async fn two_players_may_both_be_called_alice() {
    let sandbox = Sandbox::open("alice").await;

    let first = Account::insert(named("one@example.com", "Alice"))
        .exec_with_returning(&sandbox.db)
        .await
        .expect("the first Alice registers");
    let second = Account::insert(named("two@example.com", "alice"))
        .exec_with_returning(&sandbox.db)
        .await
        .expect("the second Alice registers, in a different case");

    assert_eq!(first.display_name, "Alice");
    assert_eq!(second.display_name, "alice");
    assert_ne!(
        first.tag, second.tag,
        "two accounts were handed the same tag"
    );

    sandbox.close().await;
}

/// The tag is an identity column, so nothing in the gateway picks one and
/// two registrations cannot race for the same number.
#[tokio::test]
async fn a_tag_is_the_databases_to_hand_out() {
    let sandbox = Sandbox::open("tag").await;

    let made = Account::insert(an_account("first@example.com"))
        .exec_with_returning(&sandbox.db)
        .await
        .expect("registering");
    assert_eq!(made.tag, 1, "the first account in a fresh schema is #0001");

    let next = Account::insert(an_account("second@example.com"))
        .exec_with_returning(&sandbox.db)
        .await
        .expect("registering");
    assert_eq!(next.tag, 2);

    // And the column refuses a second account the same number, which is
    // what makes a tag a way to find somebody.
    let clash = sandbox
        .db
        .execute_unprepared(
            "INSERT INTO account (id, email, display_name, tag, password_hash, created_at, lang) \
             VALUES (gen_random_uuid(), 'third@example.com', 'Third', 2, 'x', now(), 'de')",
        )
        .await;
    assert!(clash.is_err(), "two accounts were given tag 2");

    sandbox.close().await;
}

/// Closing an account is one `DELETE`. Everything that hangs off it goes,
/// and the JSON store's six-map walk — where a forgotten map left a deck
/// owned by nobody — stops being possible.
#[tokio::test]
async fn deleting_an_account_takes_everything_it_owned() {
    let sandbox = Sandbox::open("cascade").await;

    let account = an_account("leaver@example.com");
    let Set(id) = account.id else { unreachable!() };
    Account::insert(account).exec(&sandbox.db).await.unwrap();

    Deck::insert(deck::ActiveModel {
        id: Set(Uuid::now_v7()),
        account_id: Set(Some(id)),
        kind: Set(deck::KIND_ACCOUNT.to_owned()),
        format: Set("freeform".to_owned()),
        description: Set(None),
        copied_from: Set(None),
        copied_version: Set(None),
        version: Set(1),
        name: Set("Mono Green".to_owned()),
        cards: Set(vec!["4 Llanowar Elves".to_owned(), "20 Forest".to_owned()]),
        sideboard: Set(Vec::new()),
        commanders: Set(Vec::new()),
        sleeve: Set(None),
        playmat: Set(None),
        updated_at: Set(OffsetDateTime::now_utc()),
        source: Set(None),
        offered: Set(true),
    })
    .exec(&sandbox.db)
    .await
    .expect("a deck saves");

    SessionToken::insert(session_token::ActiveModel {
        token_hash: Set(vec![0xff; 32]),
        account_id: Set(id),
        expires_at: Set(OffsetDateTime::now_utc()),
    })
    .exec(&sandbox.db)
    .await
    .expect("a session stores");

    ClientSettings::insert(client_settings::ActiveModel {
        account_id: Set(id),
        doc: Set(serde_json::json!({"keymap": {"pass": "Space"}})),
        updated_at: Set(OffsetDateTime::now_utc()),
    })
    .exec(&sandbox.db)
    .await
    .expect("preferences store");

    Upload::insert(upload::ActiveModel {
        image_id: Set("ab".repeat(32)),
        account_id: Set(id),
        kind: Set("sleeve".to_owned()),
        created_at: Set(OffsetDateTime::now_utc()),
    })
    .exec(&sandbox.db)
    .await
    .expect("an upload's owner stores");

    // The decklist survives the round trip as an ordered array, which is the
    // one thing `text[]` has to do that a join table with a position column
    // would have done more elaborately.
    // Whose deck, explicitly: every database ships with the house
    // decks, so "the deck" is no longer a question with one answer.
    let saved = Deck::find()
        .filter(deck::Column::AccountId.eq(id))
        .one(&sandbox.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(saved.cards, ["4 Llanowar Elves", "20 Forest"]);

    Account::delete_by_id(id).exec(&sandbox.db).await.unwrap();

    assert_eq!(
        Deck::find()
            .filter(deck::Column::AccountId.eq(id))
            .count(&sandbox.db)
            .await
            .unwrap(),
        0,
        "a deck outlived the account that owned it"
    );
    assert_eq!(
        SessionToken::find().count(&sandbox.db).await.unwrap(),
        0,
        "a session outlived the account it signed in"
    );
    assert_eq!(
        ClientSettings::find().count(&sandbox.db).await.unwrap(),
        0,
        "a preferences document outlived its account"
    );
    assert_eq!(
        Upload::find().count(&sandbox.db).await.unwrap(),
        0,
        "an account's claim on a picture outlived it"
    );

    sandbox.close().await;
}

/// An account's deletion answers every picture it claimed, read in the one
/// statement whose cascade removes the claims (#292): the caller removes
/// the files nobody claims any more, and could not find them afterwards.
/// Another account's claim on the same picture stays.
#[tokio::test]
async fn a_deletion_says_which_pictures_its_account_claimed() {
    let sandbox = Sandbox::open("delete_pictures").await;
    let mut players = Vec::new();
    for email in ["leaver@example.com", "stayer@example.com"] {
        let mut account = an_account(email);
        // A column from a later migration than this schema has.
        account.invite_id = NotSet;
        let Set(id) = account.id else { unreachable!() };
        Account::insert(account).exec(&sandbox.db).await.unwrap();
        players.push(id);
    }
    let (leaver, stayer) = (players[0], players[1]);
    let (shared, own) = ("aa".repeat(32), "bb".repeat(32));
    for (image, account, kind) in [
        (&shared, leaver, "sleeve"),
        (&own, leaver, "playmat"),
        (&shared, stayer, "sleeve"),
    ] {
        Upload::insert(upload::ActiveModel {
            image_id: Set(image.clone()),
            account_id: Set(account),
            kind: Set(kind.to_owned()),
            created_at: Set(OffsetDateTime::now_utc()),
        })
        .exec(&sandbox.db)
        .await
        .unwrap();
    }

    let gone = baylee_db::accounts::delete(&sandbox.db, leaver)
        .await
        .expect("the deletion");
    assert_eq!(gone.accounts, [leaver]);
    let mut claimed = vec![shared.clone(), own.clone()];
    claimed.sort();
    assert_eq!(gone.pictures, claimed, "both, though the cascade took them");

    let left: Vec<(String, Uuid)> = Upload::find()
        .all(&sandbox.db)
        .await
        .unwrap()
        .into_iter()
        .map(|u| (u.image_id, u.account_id))
        .collect();
    assert_eq!(left, [(shared, stayer)], "the other claim stays");
    assert_eq!(
        baylee_db::accounts::delete(&sandbox.db, leaver)
            .await
            .unwrap(),
        baylee_db::accounts::Gone::default(),
        "nobody left to delete"
    );

    sandbox.close().await;
}

/// What the gateway's sweep keeps (#301): a picture with an owner, and a
/// picture a deck shows as its sleeve or its playmat whether or not anybody
/// owns it. Only a picture with neither is left out, and so swept.
#[tokio::test]
async fn a_picture_a_deck_shows_is_claimed_without_an_owner() {
    let sandbox = Sandbox::open("claimed_pictures").await;
    let account = an_account("keeper@example.com");
    let Set(player) = account.id else {
        unreachable!()
    };
    Account::insert(account).exec(&sandbox.db).await.unwrap();
    let (owned, sleeve, mat) = ("aa".repeat(32), "bb".repeat(32), "cc".repeat(32));
    Upload::insert(upload::ActiveModel {
        image_id: Set(owned.clone()),
        account_id: Set(player),
        kind: Set("sleeve".to_owned()),
        created_at: Set(OffsetDateTime::now_utc()),
    })
    .exec(&sandbox.db)
    .await
    .unwrap();
    // Neither of these has an owner row.
    let mut shown = a_deck(Some(player), "shown");
    shown.sleeve = Set(Some(sleeve.clone()));
    Deck::insert(shown).exec(&sandbox.db).await.unwrap();
    let mut laid = a_deck(None, "laid");
    laid.kind = Set(deck::KIND_HOUSE.to_owned());
    laid.playmat = Set(Some(mat.clone()));
    Deck::insert(laid).exec(&sandbox.db).await.unwrap();

    let claimed = baylee_db::pictures::claimed(&sandbox.db)
        .await
        .expect("the claims");
    assert!(claimed.contains(&sleeve), "a deck's sleeve is claimed");
    assert!(claimed.contains(&mat), "a deck's playmat is claimed");
    assert!(claimed.contains(&owned), "an owned picture is claimed");
    assert_eq!(claimed.len(), 3, "and nothing else: {claimed:?}");

    sandbox.close().await;
}

/// The pictures uploaded before anybody recorded who uploaded them (#292)
/// belong to every account with a deck that shows them: the uploader, or a
/// player who copied the uploader's deck. A deck of nobody's gives nobody a
/// picture, and a picture on two of one player's decks is one claim.
#[tokio::test]
async fn a_picture_on_a_deck_belongs_to_the_decks_player() {
    // The schema before the migration that records owners.
    let sandbox = Sandbox::open_at("upload_owners", Some(7)).await;
    let (sleeve, mat, published) = ("aa".repeat(32), "bb".repeat(32), "cc".repeat(32));
    let mut players = Vec::new();
    for email in ["painter@example.com", "copier@example.com"] {
        let mut account = an_account(email);
        // Columns from later migrations than this schema has.
        account.invite_id = NotSet;
        account.terms_version = NotSet;
        account.terms_accepted_at = NotSet;
        account.must_change_password = NotSet;
        account.banned_at = NotSet;
        account.banned_until = NotSet;
        account.ban_reason = NotSet;
        account.banned_by = NotSet;
        let Set(id) = account.id else { unreachable!() };
        Account::insert(account).exec(&sandbox.db).await.unwrap();
        players.push(id);
    }
    let (painter, copier) = (players[0], players[1]);
    let decks = [
        (Some(painter), Some(&sleeve), Some(&mat)),
        (Some(painter), Some(&sleeve), None),
        (Some(copier), Some(&sleeve), None),
        (None, Some(&published), Some(&mat)),
    ];
    for (owner, on_sleeve, on_mat) in decks {
        let mut deck = a_deck(owner, "shown");
        if owner.is_none() {
            deck.kind = Set(deck::KIND_PRECONSTRUCTED.to_owned());
        }
        deck.sleeve = Set(on_sleeve.cloned());
        deck.playmat = Set(on_mat.cloned());
        // Columns from a later migration than this schema has.
        deck.source = NotSet;
        deck.offered = NotSet;
        Deck::insert(deck).exec(&sandbox.db).await.unwrap();
    }
    Migrator::up(&sandbox.db, None)
        .await
        .expect("the migration records them");

    let mut owners: Vec<(String, Uuid, String)> = Upload::find()
        .all(&sandbox.db)
        .await
        .unwrap()
        .into_iter()
        .map(|u| (u.image_id, u.account_id, u.kind))
        .collect();
    owners.sort();
    let mut expected = vec![
        (sleeve.clone(), painter, "sleeve".to_owned()),
        (sleeve.clone(), copier, "sleeve".to_owned()),
        (mat.clone(), painter, "playmat".to_owned()),
    ];
    expected.sort();
    assert_eq!(owners, expected);

    sandbox.close().await;
}

/// A whole store file, with something in every one of the six maps, so the
/// import's foreign keys are exercised rather than only its accounts. The
/// standing answers in `automation` are read and dropped: their table is gone
/// (#233), and the file must import anyway.
const A_WHOLE_STORE: &str = r#"{
  "accounts": {
    "0192f0c0-0000-7000-8000-000000000001": {
      "id": "0192f0c0-0000-7000-8000-000000000001",
      "email": "Keeper@Example.COM",
      "display_name": "Keeper",
      "password_hash": "$argon2id$v=19$m=19456,t=2,p=1$c2FsdA$aGFzaA",
      "created_at": 1700000000,
      "confirmed_at": 1700000050,
      "lang": "de"
    }
  },
  "tokens": {
    "aaaa": {
      "token_hash": "aaaa",
      "account_id": "0192f0c0-0000-7000-8000-000000000001",
      "expires_at": 1900000000
    }
  },
  "confirmations": {
    "bbbb": {
      "token_hash": "bbbb",
      "account_id": "0192f0c0-0000-7000-8000-000000000001",
      "expires_at": 1900000000
    }
  },
  "decks": {
    "0192f0c0-0000-7000-8000-00000000000a": {
      "id": "0192f0c0-0000-7000-8000-00000000000a",
      "account_id": "0192f0c0-0000-7000-8000-000000000001",
      "name": "Mono Green",
      "cards": ["4 Llanowar Elves", "20 Forest"],
      "sideboard": ["2 Naturalize"],
      "commander": null,
      "sleeve": "5eeb5eeb5eeb5eeb5eeb5eeb5eeb5eeb5eeb5eeb5eeb5eeb5eeb5eeb5eeb5eeb",
      "updated_at": 1700000100
    }
  },
  "automation": {
    "0192f0c0-0000-7000-8000-000000000001": [
      { "card": 7, "ability": 4294967295, "yes": true }
    ]
  },
  "settings": {
    "0192f0c0-0000-7000-8000-000000000001": { "lang": "de" }
  }
}"#;

/// The one code path that touches somebody's real accounts, run against a
/// real server.
///
/// Every unit test in `import.rs` stops at the `Plan` — it is a pure function
/// and that is the point — so nothing before this had ever asked PostgreSQL
/// to accept the rows it builds. Three things are only true here: the
/// children land after their parents, the tally describes what is actually in
/// the tables, and a second import of the same file does nothing rather than
/// duplicating or failing.
#[tokio::test]
async fn a_store_file_becomes_the_tables_it_describes() {
    let sandbox = Sandbox::open("import").await;

    let legacy = baylee_db::import::read_legacy(A_WHOLE_STORE).expect("the fixture is a store");
    let tally = baylee_db::import::import_legacy(&sandbox.db, &legacy)
        .await
        .expect("importing into an empty schema")
        .expect("an empty schema is imported into");

    assert_eq!(tally.accounts, 1);
    assert_eq!(tally.decks, 1);
    assert_eq!(tally.tokens, 1);
    assert_eq!(tally.confirmations, 1);
    assert_eq!(tally.settings, 1);

    let saved = Account::find().one(&sandbox.db).await.unwrap().unwrap();
    assert_eq!(
        saved.email.as_deref(),
        Some("Keeper@Example.COM"),
        "the address is kept as it was typed"
    );
    // It signed in with that address, and signs in with a name from it now
    // (#269), as every account the migration found does.
    assert_eq!(saved.username.as_deref(), Some("Keeper"));
    assert_eq!(saved.username_key.as_deref(), Some("keeper"));
    assert!(saved.confirmed_at.is_some());

    let deck = Deck::find()
        .filter(deck::Column::AccountId.eq(saved.id))
        .one(&sandbox.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(deck.account_id, Some(saved.id), "the deck found its owner");
    assert_eq!(deck.sideboard, ["2 Naturalize"]);
    // The old store recorded no uploader, so the deck says whose it is.
    let owners: Vec<(String, Uuid, String)> = Upload::find()
        .all(&sandbox.db)
        .await
        .unwrap()
        .into_iter()
        .map(|u| (u.image_id, u.account_id, u.kind))
        .collect();
    assert_eq!(
        owners,
        [(deck.sleeve.clone().unwrap(), saved.id, "sleeve".to_owned())],
        "the sleeve on an imported deck belongs to its player"
    );

    let again = baylee_db::import::import_legacy(&sandbox.db, &legacy)
        .await
        .expect("a second import is not an error");
    assert!(
        again.is_none(),
        "a database with accounts in it was imported into a second time"
    );
    assert_eq!(
        Account::find().count(&sandbox.db).await.unwrap(),
        1,
        "the second import wrote rows anyway"
    );

    sandbox.close().await;
}

/// The accounts that signed in with an address before usernames (#269) are
/// each given a name from it: the part before the `@`, made to keep the
/// rule, the tag added where that part is too short or already taken, the
/// older account keeping the plain name. Then the name is unique, whatever
/// its case, and an account may have no address at all.
#[tokio::test]
async fn every_account_that_signed_in_with_an_address_is_given_a_name_from_it() {
    // The schema before the migration that names them.
    let sandbox = Sandbox::open_at("usernames", Some(5)).await;
    for (email, display_name) in [
        ("alice@example.com", "Alice"),
        ("Alice@other.example", "Alice"),
        ("bob+baylee@example.com", "Bob"),
        ("x@example.com", "Xavier"),
    ] {
        sandbox
            .db
            .execute_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "INSERT INTO account (email, display_name, password_hash, created_at, lang) \
                 VALUES ($1, $2, 'hash', now(), 'de')",
                [email.into(), display_name.into()],
            ))
            .await
            .expect("an account as the old schema held it");
    }
    Migrator::up(&sandbox.db, None)
        .await
        .expect("the migration names them");

    let names = |db: DatabaseConnection| async move {
        Account::find()
            .all(&db)
            .await
            .unwrap()
            .into_iter()
            .map(|a| {
                (
                    a.tag,
                    (
                        a.username.unwrap_or_default(),
                        a.username_key.unwrap_or_default(),
                    ),
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    let named = names(sandbox.db.clone()).await;
    let shown: Vec<&str> = named.values().map(|(shown, _)| shown.as_str()).collect();
    assert_eq!(
        shown,
        ["alice", "Alice-0002", "bob-baylee", "x-0004"],
        "by tag: the plain name, a taken one, a `+`, a short one"
    );
    for (shown, key) in named.values() {
        let rule = baylee_protocol::names::username(shown).expect("a name its owner can type");
        assert_eq!(&rule.key, key);
    }

    // Unique whatever the case.
    let mut again = an_account("carol@example.com");
    again.username = Set(Some("ALICE".into()));
    again.username_key = Set(Some("alice".into()));
    assert!(
        Account::insert(again).exec(&sandbox.db).await.is_err(),
        "a second account signed in as alice"
    );

    // Back past it and forward again: the names come back the same. Counted
    // from the list, so the migrations after it are stepped back too.
    let past_it = u32::try_from(Migrator::migrations().len() - 5).expect("a handful");
    Migrator::down(&sandbox.db, Some(past_it))
        .await
        .expect("the migration steps back");
    Migrator::up(&sandbox.db, None)
        .await
        .expect("and forward again");
    assert_eq!(names(sandbox.db.clone()).await, named);

    // And an account needs no address: two without one are no clash.
    for name in ["Dora", "Emil"] {
        let mut addressless = an_account("unused@example.com");
        addressless.email = Set(None);
        addressless.username = Set(Some(name.into()));
        addressless.username_key = Set(Some(name.to_lowercase()));
        Account::insert(addressless)
            .exec(&sandbox.db)
            .await
            .expect("an account without an address");
    }

    sandbox.close().await;
}

/// A guest, ready to insert (#269): a name to be seen by, and nothing to
/// sign in with.
fn a_guest() -> account::ActiveModel {
    let mut guest = named("unused@example.com", "Guest");
    guest.email = Set(None);
    guest.password_hash = Set(None);
    guest.guest = Set(true);
    guest
}

/// A session of `account`'s that runs out at `expires`.
fn a_session(account: Uuid, expires: OffsetDateTime) -> session_token::ActiveModel {
    session_token::ActiveModel {
        token_hash: Set(Uuid::now_v7().as_bytes().to_vec()),
        account_id: Set(account),
        expires_at: Set(expires),
    }
}

/// Who holds what, in both directions (#269): a guest holds nothing to sign
/// in with, and an account that is not a guest holds a password and a name
/// or, until #280, an address.
#[tokio::test]
async fn a_guest_holds_nothing_to_sign_in_with_and_an_account_holds_something() {
    let sandbox = Sandbox::open("guest_check").await;
    let db = &sandbox.db;

    Account::insert(a_guest())
        .exec(db)
        .await
        .expect("a guest with nothing to sign in with");

    let mut with_password = a_guest();
    with_password.password_hash = Set(Some("hash".into()));
    let mut with_name = a_guest();
    with_name.username = Set(Some("Casper".into()));
    with_name.username_key = Set(Some("casper".into()));
    let mut with_address = a_guest();
    with_address.email = Set(Some("casper@example.com".into()));
    for (what, row) in [
        ("a password", with_password),
        ("a username", with_name),
        ("an address", with_address),
    ] {
        assert!(
            Account::insert(row).exec(db).await.is_err(),
            "a guest was written holding {what}"
        );
    }

    let mut no_password = an_account("nopass@example.com");
    no_password.password_hash = Set(None);
    let mut nothing_to_name = an_account("nobody@example.com");
    nothing_to_name.email = Set(None);
    for (what, row) in [
        ("no password", no_password),
        ("neither a username nor an address", nothing_to_name),
    ] {
        assert!(
            Account::insert(row).exec(db).await.is_err(),
            "an account was written with {what}"
        );
    }

    let mut by_name = an_account("unused@example.com");
    by_name.email = Set(None);
    by_name.username = Set(Some("Named".into()));
    by_name.username_key = Set(Some("named".into()));
    Account::insert(by_name)
        .exec(db)
        .await
        .expect("an account named by its username alone");
    Account::insert(an_account("address@example.com"))
        .exec(db)
        .await
        .expect("an account named by its address alone, as the importer writes it");

    sandbox.close().await;
}

/// A guest goes once no live session leads to it, and its decks with it; a
/// guest still playing stays, and an account is never touched (#269).
#[tokio::test]
async fn a_guest_goes_with_its_last_session_and_takes_its_decks() {
    let sandbox = Sandbox::open("guest_purge").await;
    let db = &sandbox.db;
    let now = OffsetDateTime::now_utc();
    let hour = time::Duration::hours(1);

    let guest = |db: DatabaseConnection| async move {
        Account::insert(a_guest())
            .exec_with_returning(&db)
            .await
            .expect("a guest")
            .id
    };
    let playing = guest(db.clone()).await;
    SessionToken::insert(a_session(playing, now + hour))
        .exec(db)
        .await
        .unwrap();
    let lapsed = guest(db.clone()).await;
    SessionToken::insert(a_session(lapsed, now - hour))
        .exec(db)
        .await
        .unwrap();
    let signed_out = guest(db.clone()).await;
    let keeper = Account::insert(an_account("keeper@example.com"))
        .exec_with_returning(db)
        .await
        .unwrap()
        .id;

    let guest_deck = Deck::insert(a_deck(Some(lapsed), "Guest deck"))
        .exec_with_returning(db)
        .await
        .unwrap()
        .id;
    let mut copy = a_deck(Some(keeper), "Copied from a guest");
    copy.copied_from = Set(Some(guest_deck));
    copy.copied_version = Set(Some(1));
    let copy = Deck::insert(copy).exec_with_returning(db).await.unwrap().id;

    assert_eq!(
        baylee_db::guests::purge(db, now, None)
            .await
            .unwrap()
            .accounts
            .len(),
        2,
        "the lapsed guest and the one signed out"
    );
    let left: Vec<Uuid> = Account::find()
        .all(db)
        .await
        .unwrap()
        .into_iter()
        .map(|a| a.id)
        .collect();
    assert!(left.contains(&playing), "a guest still playing stays");
    assert!(left.contains(&keeper), "an account without a session stays");
    assert!(!left.contains(&lapsed) && !left.contains(&signed_out));
    assert!(
        Deck::find_by_id(guest_deck)
            .one(db)
            .await
            .unwrap()
            .is_none(),
        "the guest's deck went with it"
    );
    let copy = Deck::find_by_id(copy)
        .one(db)
        .await
        .unwrap()
        .expect("another player's copy stays");
    assert_eq!(copy.copied_from, None, "pointing at nothing");

    // One at a time: still playing, it stays; the same guest once its session
    // has run out goes; and an account never does.
    assert_eq!(
        baylee_db::guests::purge(db, now, Some(playing))
            .await
            .unwrap(),
        baylee_db::accounts::Gone::default()
    );
    assert_eq!(
        baylee_db::guests::purge(db, now + 2 * hour, Some(keeper))
            .await
            .unwrap(),
        baylee_db::accounts::Gone::default()
    );
    assert_eq!(
        baylee_db::guests::purge(db, now + 2 * hour, Some(playing))
            .await
            .unwrap()
            .accounts,
        [playing]
    );
    assert_eq!(Account::find().count(db).await.unwrap(), 1);

    sandbox.close().await;
}

/// The migration that makes guests takes a database of accounts as it finds
/// them, and steps back only over a database without a guest: an account
/// without a password cannot be given one back.
#[tokio::test]
async fn guests_come_in_over_accounts_and_step_back_only_without_one() {
    let sandbox = Sandbox::open_at("guest_migration", Some(6)).await;
    let db = &sandbox.db;
    db.execute_unprepared(
        "INSERT INTO account (email, username, username_key, display_name, password_hash, \
                              created_at, lang) \
         VALUES ('old@example.com', 'old', 'old', 'Old', 'hash', now(), 'de')",
    )
    .await
    .expect("an account as the sixth migration left it");
    Migrator::up(db, None).await.expect("guests come in");
    let old = Account::find().one(db).await.unwrap().expect("the account");
    assert!(!old.guest, "an account from before is not a guest");

    let guest = Account::insert(a_guest())
        .exec_with_returning(db)
        .await
        .unwrap()
        .id;
    let back = u32::try_from(Migrator::migrations().len() - 6).expect("a handful");
    assert!(
        Migrator::down(db, Some(back)).await.is_err(),
        "stepped back over a guest"
    );
    Account::delete_by_id(guest).exec(db).await.unwrap();
    Migrator::down(db, Some(back))
        .await
        .expect("steps back without one");
    Migrator::up(db, None).await.expect("and forward again");

    sandbox.close().await;
}

/// #293: an expired confirmation link goes in one sweep, whichever account
/// it was for, and a live one stays. A link due at the very moment of the
/// sweep has expired, as `GET /auth/confirm` holds it.
#[tokio::test]
async fn an_expired_confirmation_link_goes_in_the_sweep_and_a_live_one_stays() {
    let sandbox = Sandbox::open("confirmation_sweep").await;
    let db = &sandbox.db;
    let now = OffsetDateTime::now_utc();
    let hour = time::Duration::hours(1);
    for (email, expires) in [
        ("lapsed@example.com", now - hour),
        ("due@example.com", now),
        ("waiting@example.com", now + hour),
    ] {
        let account = Account::insert(an_account(email))
            .exec_with_returning(db)
            .await
            .unwrap()
            .id;
        Confirmation::insert(confirmation::ActiveModel {
            token_hash: Set(email.as_bytes().to_vec()),
            account_id: Set(account),
            expires_at: Set(expires),
        })
        .exec(db)
        .await
        .unwrap();
    }

    assert_eq!(baylee_db::confirmations::sweep(db, now).await.unwrap(), 2);
    let left: Vec<Vec<u8>> = Confirmation::find()
        .all(db)
        .await
        .unwrap()
        .into_iter()
        .map(|link| link.token_hash)
        .collect();
    assert_eq!(
        left,
        [b"waiting@example.com".to_vec()],
        "the live one stays"
    );
    assert_eq!(
        baylee_db::confirmations::sweep(db, now).await.unwrap(),
        0,
        "and a second sweep finds nothing"
    );
    assert_eq!(
        Account::find().count(db).await.unwrap(),
        3,
        "no account went"
    );

    sandbox.close().await;
}

/// A game's record (#315) comes in pieces and is read back whole, in order,
/// by a player who sat at the game and by nobody else.
#[tokio::test]
async fn a_game_record_is_read_back_whole_by_a_player_who_sat_there() {
    use baylee_db::records;

    let sandbox = Sandbox::open("records").await;
    let db = &sandbox.db;
    let (me, stranger) = (an_account("me@example.com"), an_account("them@example.com"));
    let (me_id, stranger_id) = (me.id.clone().unwrap(), stranger.id.clone().unwrap());
    Account::insert(me).exec(db).await.expect("an account");
    Account::insert(stranger)
        .exec(db)
        .await
        .expect("an account");

    // Seat 1 is my seat bridge's (a language model at my table): no account
    // played it, and I answer for it.
    let mine = records::Seat {
        account: None,
        delegated_by: Some(me_id),
    };
    records::open(db, "g1", &[records::Seat::played_by(me_id), mine])
        .await
        .expect("opens");
    // Out of order, and one piece twice: the store keeps each once, in
    // `seq` order.
    assert!(
        records::append(db, "g1", 1, b"world".to_vec(), false)
            .await
            .unwrap()
    );
    assert!(
        records::append(db, "g1", 0, b"hello ".to_vec(), false)
            .await
            .unwrap()
    );
    assert!(
        !records::append(db, "g1", 0, b"again".to_vec(), false)
            .await
            .unwrap()
    );
    assert!(
        !records::append(db, "nowhere", 0, b"x".to_vec(), true)
            .await
            .unwrap(),
        "a piece of a record never opened is dropped"
    );
    let open = records::for_seated(db, "g1", me_id)
        .await
        .unwrap()
        .expect("mine");
    assert_eq!(open.data, b"hello world");
    assert!(!open.complete);
    assert!(
        records::append(db, "g1", 2, b"!".to_vec(), true)
            .await
            .unwrap()
    );
    let done = records::for_seated(db, "g1", me_id)
        .await
        .unwrap()
        .expect("mine");
    assert_eq!(done.data, b"hello world!");
    assert!(done.complete);

    assert_eq!(
        records::for_seated(db, "g1", stranger_id).await.unwrap(),
        None
    );
    assert_eq!(records::for_seated(db, "g2", me_id).await.unwrap(), None);

    // Opening again rewrites nobody's seat.
    records::open(
        db,
        "g1",
        &[
            records::Seat::played_by(stranger_id),
            records::Seat::default(),
        ],
    )
    .await
    .unwrap();
    assert_eq!(
        records::for_seated(db, "g1", stranger_id).await.unwrap(),
        None
    );

    // The account goes; the record stays, linked to nobody.
    baylee_db::accounts::delete(db, me_id)
        .await
        .expect("deletes");
    let row = db
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM game_record) AS records, \
                    (SELECT count(*) FROM game_record_seat \
                      WHERE account_id IS NOT NULL OR delegated_by IS NOT NULL) AS linked, \
                    (SELECT bytes FROM game_record WHERE game_id = 'g1') AS bytes",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i64>("", "records").unwrap(), 1);
    assert_eq!(row.try_get::<i64>("", "linked").unwrap(), 0);
    assert_eq!(row.try_get::<i64>("", "bytes").unwrap(), 12);

    sandbox.close().await;
}

/// A seat a host's seat bridge played names the host as the one who answers
/// for it, never as the one who played it, and goes with the host's account.
#[tokio::test]
async fn a_bridges_seat_names_its_host_and_goes_with_it() {
    use baylee_db::records;

    let sandbox = Sandbox::open("record_delegate").await;
    let db = &sandbox.db;
    let host = an_account("host@example.com");
    let host_id = host.id.clone().unwrap();
    Account::insert(host).exec(db).await.expect("an account");
    let bridge = records::Seat {
        account: None,
        delegated_by: Some(host_id),
    };
    records::open(db, "g1", &[records::Seat::played_by(host_id), bridge])
        .await
        .expect("opens");
    let delegated = |db| async move {
        let row = sea_orm::ConnectionTrait::query_one_raw(
            db,
            Statement::from_string(
                DbBackend::Postgres,
                "SELECT count(*) AS n FROM game_record_seat \
                 WHERE seat = 1 AND account_id IS NULL AND delegated_by IS NOT NULL",
            ),
        )
        .await
        .unwrap()
        .unwrap();
        row.try_get::<i64>("", "n").unwrap()
    };
    assert_eq!(delegated(db).await, 1);
    // A seat has one occupant: an account, or a host's bridge.
    let both = db
        .execute_raw(Statement::from_string(
            DbBackend::Postgres,
            format!(
                "INSERT INTO game_record_seat (game_id, seat, account_id, delegated_by) \
                 VALUES ('g1', 7, '{host_id}', '{host_id}')"
            ),
        ))
        .await;
    assert!(both.is_err(), "played and delegated at once");
    baylee_db::accounts::delete(db, host_id)
        .await
        .expect("deletes");
    assert_eq!(delegated(db).await, 0, "the link went with the account");

    sandbox.close().await;
}

/// The edges of a record: started and never sent a piece, sent pieces and
/// never its last, a piece after the last, a `seq` the column cannot hold.
#[tokio::test]
async fn a_game_record_at_its_edges() {
    use baylee_db::records;

    let sandbox = Sandbox::open("record_edges").await;
    let db = &sandbox.db;
    let me = an_account("edge@example.com");
    let me_id = me.id.clone().unwrap();
    Account::insert(me).exec(db).await.expect("an account");

    // Started, nothing sent: an empty record, not a missing one.
    records::open(db, "empty", &[records::Seat::played_by(me_id)])
        .await
        .unwrap();
    assert_eq!(
        records::for_seated(db, "empty", me_id).await.unwrap(),
        Some(records::Record {
            data: Vec::new(),
            complete: false
        })
    );

    // Its engine went before the last piece: what came, incomplete.
    records::open(
        db,
        "cut",
        &[records::Seat::default(), records::Seat::played_by(me_id)],
    )
    .await
    .unwrap();
    for (seq, piece) in [(0, b"ab"), (1, b"cd")] {
        assert!(
            records::append(db, "cut", seq, piece.to_vec(), false)
                .await
                .unwrap()
        );
    }
    let cut = records::for_seated(db, "cut", me_id)
        .await
        .unwrap()
        .expect("mine");
    assert_eq!(cut.data, b"abcd");
    assert!(!cut.complete);

    // An empty last piece completes it and adds nothing; a piece after it
    // is kept in order, and the record stays complete.
    assert!(
        records::append(db, "cut", 2, Vec::new(), true)
            .await
            .unwrap()
    );
    assert!(
        records::append(db, "cut", 3, b"!".to_vec(), false)
            .await
            .unwrap()
    );
    let done = records::for_seated(db, "cut", me_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(done.data, b"abcd!");
    assert!(done.complete, "a later piece does not undo the last");

    // Refused before the database is asked, whose `seq >= 0` check would
    // otherwise be the only thing between a wrapped seq and the table.
    assert!(
        matches!(
            records::append(db, "cut", u32::MAX, b"x".to_vec(), false).await,
            Err(sea_orm::DbErr::Custom(_))
        ),
        "a seq past the column is refused, not wrapped"
    );
    assert!(
        records::append(db, "cut", i32::MAX.unsigned_abs(), b"?".to_vec(), false)
            .await
            .unwrap(),
        "the largest seq the column holds"
    );
    assert_eq!(
        records::for_seated(db, "cut", me_id)
            .await
            .unwrap()
            .unwrap()
            .data,
        b"abcd!?"
    );

    sandbox.close().await;
}

/// A record flushed for every report and every half minute (#323) comes in
/// many small pieces: each is kept, in `seq` order, and counted, however
/// small, and the record reads back as their concatenation.
#[tokio::test]
async fn a_game_record_of_many_small_pieces_reads_back_in_order() {
    use baylee_db::records;

    let sandbox = Sandbox::open("record_small").await;
    let db = &sandbox.db;
    let me = an_account("small@example.com");
    let me_id = me.id.clone().unwrap();
    Account::insert(me).exec(db).await.expect("an account");

    records::open(db, "small", &[records::Seat::played_by(me_id)])
        .await
        .unwrap();
    let pieces: Vec<Vec<u8>> = (0..300_u32)
        .map(|n| format!("{n}\n").into_bytes())
        .collect();
    for (seq, piece) in pieces.iter().enumerate() {
        let seq = u32::try_from(seq).unwrap();
        assert!(
            records::append(db, "small", seq, piece.clone(), false)
                .await
                .unwrap(),
            "piece {seq}"
        );
    }
    let read = records::for_seated(db, "small", me_id)
        .await
        .unwrap()
        .expect("mine");
    assert_eq!(read.data, pieces.concat(), "in seq order, not text order");
    assert!(!read.complete);
    let bytes = db
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT bytes FROM game_record WHERE game_id = 'small'",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<i64>("", "bytes")
        .unwrap();
    assert_eq!(usize::try_from(bytes).unwrap(), pieces.concat().len());

    sandbox.close().await;
}

// ------------------------------------------------------ closed-beta keys

/// A key's row as the gateway's command writes it (#317): `hash` stands in
/// for the SHA-256 of a key, which this crate never sees made.
fn an_invite(hash: &[u8], uses: i32, expires_at: Option<OffsetDateTime>) -> invites::NewInvite {
    invites::NewInvite {
        key_hash: hash.to_vec(),
        note: Some("for Max".into()),
        uses,
        expires_at,
    }
}

/// Registers `username` the way the gateway does in invite mode: the key's
/// use and the account in one transaction, the use first. `Ok(None)` is a
/// key that admitted nobody, and nothing was written.
async fn register_with(
    db: &DatabaseConnection,
    hash: &[u8],
    username: &str,
    now: OffsetDateTime,
) -> Result<Option<Uuid>, sea_orm::DbErr> {
    use sea_orm::TransactionTrait as _;
    let txn = db.begin().await?;
    let Some(invite) = invites::redeem(&txn, hash, now).await? else {
        return Ok(None);
    };
    let mut row = an_account("unused@example.com");
    row.email = Set(None);
    row.username = Set(Some(username.to_owned()));
    row.username_key = Set(Some(username.to_lowercase()));
    row.invite_id = Set(Some(invite));
    let made = Account::insert(row).exec_with_returning(&txn).await?;
    txn.commit().await?;
    Ok(Some(made.id))
}

/// Made, listed with everything but the key, and revoked once (#317).
#[tokio::test]
async fn a_key_is_made_listed_and_revoked() {
    let sandbox = Sandbox::open("invite_list").await;
    let db = &sandbox.db;
    let now = OffsetDateTime::now_utc();
    let later = now + time::Duration::days(30);

    let first = invites::create(db, &an_invite(b"one", 1, None))
        .await
        .unwrap();
    let second = invites::create(
        db,
        &invites::NewInvite {
            note: None,
            ..an_invite(b"two", 5, Some(later))
        },
    )
    .await
    .unwrap();
    let listed = invites::list(db).await.unwrap();
    assert_eq!(
        listed.iter().map(|i| i.id).collect::<Vec<_>>(),
        [first, second],
        "oldest first"
    );
    assert_eq!(listed[0].note.as_deref(), Some("for Max"));
    assert_eq!((listed[0].uses_left, listed[0].expires_at), (1, None));
    assert_eq!(listed[1].note, None);
    assert_eq!(listed[1].uses_left, 5);
    assert_eq!(
        listed[1].expires_at.map(OffsetDateTime::unix_timestamp),
        Some(later.unix_timestamp())
    );
    assert!(
        listed
            .iter()
            .all(|i| i.revoked_at.is_none() && i.admitted == 0)
    );

    assert!(invites::revoke(db, first, now).await.unwrap());
    assert!(
        !invites::revoke(db, first, later).await.unwrap(),
        "the first revocation stands"
    );
    assert!(
        !invites::revoke(db, Uuid::now_v7(), now).await.unwrap(),
        "no such key"
    );
    let listed = invites::list(db).await.unwrap();
    assert_eq!(
        listed[0].revoked_at.map(OffsetDateTime::unix_timestamp),
        Some(now.unix_timestamp())
    );
    assert_eq!(listed[1].revoked_at, None, "only the one named");

    sandbox.close().await;
}

/// Each account a key admits takes one use, and a key with none left, one
/// past its expiry, or one revoked admits nobody and writes nothing (#317).
#[tokio::test]
async fn a_key_admits_as_many_as_it_has_uses_and_no_one_once_closed() {
    let sandbox = Sandbox::open("invite_redeem").await;
    let db = &sandbox.db;
    let now = OffsetDateTime::now_utc();

    let two = invites::create(db, &an_invite(b"two uses", 2, None))
        .await
        .unwrap();
    let alice = register_with(db, b"two uses", "Alice", now)
        .await
        .unwrap()
        .expect("the first use");
    register_with(db, b"two uses", "Bob", now)
        .await
        .unwrap()
        .expect("the second use");
    assert_eq!(
        register_with(db, b"two uses", "Carol", now).await.unwrap(),
        None,
        "no use left"
    );
    let listed = invites::list(db).await.unwrap();
    assert_eq!((listed[0].uses_left, listed[0].admitted), (0, 2));
    let admitted_by: Option<Uuid> = Account::find_by_id(alice)
        .one(db)
        .await
        .unwrap()
        .unwrap()
        .invite_id;
    assert_eq!(admitted_by, Some(two), "the account names its key");

    invites::create(
        db,
        &an_invite(b"expired", 1, Some(now - time::Duration::seconds(1))),
    )
    .await
    .unwrap();
    assert_eq!(
        register_with(db, b"expired", "Dave", now).await.unwrap(),
        None,
        "past its expiry"
    );
    invites::create(
        db,
        &an_invite(b"expiring", 1, Some(now + time::Duration::seconds(60))),
    )
    .await
    .unwrap();
    assert!(
        register_with(db, b"expiring", "Erin", now)
            .await
            .unwrap()
            .is_some(),
        "before its expiry"
    );

    let revoked = invites::create(db, &an_invite(b"revoked", 3, None))
        .await
        .unwrap();
    invites::revoke(db, revoked, now).await.unwrap();
    assert_eq!(
        register_with(db, b"revoked", "Frank", now).await.unwrap(),
        None,
        "revoked"
    );
    assert_eq!(
        register_with(db, b"no such key", "Gina", now)
            .await
            .unwrap(),
        None,
        "a key never made"
    );
    let uses_left: Vec<i32> = invites::list(db)
        .await
        .unwrap()
        .iter()
        .map(|i| i.uses_left)
        .collect();
    assert_eq!(uses_left, [0, 1, 0, 3], "a refusal takes no use");
    assert_eq!(
        Account::find().count(db).await.unwrap(),
        3,
        "Alice, Bob and Erin, and nobody refused"
    );

    sandbox.close().await;
}

/// **Two registrations racing for a key's one use: one account.** Each is
/// its own transaction on its own connection, and both are in flight before
/// either commits.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_registrations_racing_for_a_one_use_key_make_one_account() {
    let sandbox = Sandbox::open("invite_race").await;
    let now = OffsetDateTime::now_utc();
    for round in 0..8 {
        let hash = format!("race {round}").into_bytes();
        invites::create(&sandbox.db, &an_invite(&hash, 1, None))
            .await
            .unwrap();
        let (a, b) = (sandbox.db.clone(), sandbox.db.clone());
        let (ha, hb) = (hash.clone(), hash.clone());
        let (one, other) = tokio::join!(
            tokio::spawn(async move { register_with(&a, &ha, &format!("left{round}"), now).await }),
            tokio::spawn(
                async move { register_with(&b, &hb, &format!("right{round}"), now).await }
            ),
        );
        let made = [one.unwrap().unwrap(), other.unwrap().unwrap()];
        assert_eq!(
            made.iter().filter(|m| m.is_some()).count(),
            1,
            "round {round}: {made:?}"
        );
    }
    assert_eq!(Account::find().count(&sandbox.db).await.unwrap(), 8);
    assert!(
        invites::list(&sandbox.db)
            .await
            .unwrap()
            .iter()
            .all(|i| i.uses_left == 0 && i.admitted == 1)
    );

    sandbox.close().await;
}

/// A registration that fails after the key's use (a taken name) gives the
/// use back with its rollback.
#[tokio::test]
async fn a_registration_refused_after_its_key_gives_the_use_back() {
    let sandbox = Sandbox::open("invite_rollback").await;
    let db = &sandbox.db;
    let now = OffsetDateTime::now_utc();
    invites::create(db, &an_invite(b"key", 2, None))
        .await
        .unwrap();
    register_with(db, b"key", "Alice", now)
        .await
        .unwrap()
        .unwrap();
    assert!(
        register_with(db, b"key", "alice", now).await.is_err(),
        "the name is taken"
    );
    assert_eq!(invites::list(db).await.unwrap()[0].uses_left, 1);

    sandbox.close().await;
}

/// An account's deletion leaves its key's row; a key's deletion leaves its
/// accounts and forgets which key admitted them. A note is bounded by the
/// table as well as by the command (#317).
#[tokio::test]
async fn a_key_and_its_accounts_outlive_each_other() {
    let sandbox = Sandbox::open("invite_links").await;
    let db = &sandbox.db;
    let now = OffsetDateTime::now_utc();
    let key = invites::create(db, &an_invite(b"key", 2, None))
        .await
        .unwrap();
    let alice = register_with(db, b"key", "Alice", now)
        .await
        .unwrap()
        .unwrap();
    let bob = register_with(db, b"key", "Bob", now)
        .await
        .unwrap()
        .unwrap();

    baylee_db::accounts::delete(db, alice).await.unwrap();
    let listed = invites::list(db).await.unwrap();
    assert_eq!(listed.len(), 1, "the key stays");
    assert_eq!(
        (listed[0].uses_left, listed[0].admitted),
        (0, 1),
        "its use stays spent, and it counts the account that is left"
    );

    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "DELETE FROM invite WHERE id = $1",
        [key.into()],
    ))
    .await
    .unwrap();
    let bob = Account::find_by_id(bob)
        .one(db)
        .await
        .unwrap()
        .expect("the account stays");
    assert_eq!(bob.invite_id, None, "and forgets its key");

    let long = invites::NewInvite {
        note: Some("x".repeat(101)),
        ..an_invite(b"long", 1, None)
    };
    assert!(invites::create(db, &long).await.is_err(), "101 characters");
    let fits = invites::NewInvite {
        note: Some("ü".repeat(100)),
        ..an_invite(b"fits", 1, None)
    };
    invites::create(db, &fits)
        .await
        .expect("100 characters, whatever their bytes");
    assert!(
        invites::create(db, &an_invite(b"negative", -1, None))
            .await
            .is_err(),
        "a negative count"
    );

    sandbox.close().await;
}

// ------------------------------------------------------------- precons

/// A precon list as `decks-import` writes one, cut down to what a sync
/// reads.
const SUN_EMPIRE: &str = "# baylee deck export v1\n\
    # name: Sun Empire\n\
    # format: freeform\n\
    # source: MTGJSON 1.0.0, https://mtgjson.com, SunEmpire_E02.json\n\
    # type: Theme Deck\n\
    # set: E02\n\
    # released: 2017-11-24\n\
    1 Sol Ring (C18) 222\n\
    4 Plains\n\
    SB: 1 Lightning Bolt\n";

/// The same list as a later build of the source has it: one card fewer.
const SUN_EMPIRE_LATER: &str = "# baylee deck export v1\n\
    # name: Sun Empire\n\
    # format: freeform\n\
    # source: MTGJSON 2.0.0, https://mtgjson.com, SunEmpire_E02.json\n\
    # type: Theme Deck\n\
    # set: E02\n\
    # released: 2017-11-24\n\
    1 Sol Ring (C18) 222\n\
    3 Plains\n\
    SB: 1 Lightning Bolt\n";

/// A list with a commander.
const ESTRID: &str = "# baylee deck export v1\n\
    # name: Adaptive Enchantment\n\
    # format: commander\n\
    # source: MTGJSON 1.0.0, https://mtgjson.com, AdaptiveEnchantment_C18.json\n\
    # type: Commander Deck\n\
    # set: C18\n\
    # released: 2018-08-10\n\
    CMD: 1 Estrid, the Masked (C18) 40\n\
    99 Forest\n";

const SUN: Precon = Precon {
    key: "E02/sun-empire",
    text: SUN_EMPIRE,
};
const SUN_LATER: Precon = Precon {
    key: "E02/sun-empire",
    text: SUN_EMPIRE_LATER,
};
const EST: Precon = Precon {
    key: "C18/adaptive-enchantment",
    text: ESTRID,
};

/// Every deck a sync owns, in key order.
async fn synced(db: &DatabaseConnection) -> Vec<deck::Model> {
    let mut rows = Deck::find()
        .filter(deck::Column::Source.is_not_null())
        .all(db)
        .await
        .expect("reading the synced decks");
    rows.sort_by(|a, b| a.source.cmp(&b.source));
    rows
}

/// The deck a sync wrote from one list.
async fn synced_from(db: &DatabaseConnection, key: &str) -> deck::Model {
    Deck::find()
        .filter(deck::Column::Source.eq(key))
        .one(db)
        .await
        .expect("reading a synced deck")
        .unwrap_or_else(|| panic!("no deck from {key}"))
}

/// A sync writes each list once as a deck that belongs to nobody, and the
/// same lists again write nothing — with the stamp, not even a read of the
/// decks, and without it the same rows to the byte.
#[tokio::test]
async fn a_precon_sync_writes_its_decks_once() {
    let sandbox = Sandbox::open("precon_once").await;
    let db = &sandbox.db;
    let house_before = precons::shared_decks(db).await.unwrap();

    let first = precons::sync(db, &[EST, SUN])
        .await
        .expect("the first sync");
    assert_eq!((first.offered, first.added, first.unchanged), (2, 2, false));
    let written = synced(db).await;
    assert_eq!(written.len(), 2);
    for row in &written {
        assert_eq!(row.kind, deck::KIND_PRECONSTRUCTED);
        assert_eq!(row.account_id, None, "{} belongs to somebody", row.name);
        assert_eq!(row.version, 1);
        assert!(row.offered);
    }
    let estrid = &written[0];
    assert_eq!(estrid.name, "Adaptive Enchantment");
    assert_eq!(estrid.format, "commander");
    assert_eq!(
        estrid.description.as_deref(),
        Some("Commander Deck · C18 · 2018-08-10")
    );
    // Stored as a player's import of the same file would be: the leader a
    // row with its printing, and named again bare.
    assert_eq!(estrid.cards, ["1 Estrid, the Masked (C18) 40", "99 Forest"]);
    assert_eq!(estrid.commanders, ["Estrid, the Masked"]);
    assert_eq!(written[1].sideboard, ["1 Lightning Bolt"]);

    // Listed after the house's own decks, which a sync never touches.
    let listed = precons::shared_decks(db).await.unwrap();
    assert_eq!(listed.len(), house_before.len() + 2);
    assert_eq!(&listed[..house_before.len()], &house_before[..]);

    let again = precons::sync(db, &[EST, SUN])
        .await
        .expect("the second sync");
    assert!(again.unchanged, "the same lists are one read: {again:?}");
    assert_eq!(synced(db).await, written, "and no write");

    // Without the stamp the sync reads every deck, and still writes none.
    db.execute_unprepared("DELETE FROM deck_sync")
        .await
        .unwrap();
    let blind = precons::sync(db, &[EST, SUN])
        .await
        .expect("a sync without a stamp");
    assert_eq!(
        (blind.unchanged, blind.added, blind.changed, blind.withdrawn),
        (false, 0, 0, 0)
    );
    assert_eq!(synced(db).await, written, "the same rows to the byte");

    sandbox.close().await;
}

/// A precon the build no longer plays leaves the offer and nothing else: the
/// deck stays, so a player's copy still names what it came from, and when
/// the build plays it again it is the same deck.
#[tokio::test]
async fn a_precon_the_build_stops_playing_is_withdrawn_and_its_copy_plays_on() {
    let sandbox = Sandbox::open("precon_withdrawn").await;
    let db = &sandbox.db;
    precons::sync(db, &[EST, SUN])
        .await
        .expect("the first sync");
    let sun = synced_from(db, SUN.key).await;

    let player = an_account("copier@example.com");
    let Set(player_id) = player.id else {
        unreachable!()
    };
    Account::insert(player).exec(db).await.unwrap();
    let mut copy = a_deck(Some(player_id), "Sun Empire");
    copy.copied_from = Set(Some(sun.id));
    copy.copied_version = Set(Some(sun.version));
    let copy = Deck::insert(copy).exec_with_returning(db).await.unwrap();

    let without = precons::sync(db, &[EST]).await.expect("a sync without it");
    assert_eq!(
        (without.withdrawn, without.added, without.changed),
        (1, 0, 0)
    );
    let withdrawn = synced_from(db, SUN.key).await;
    assert!(!withdrawn.offered);
    assert_eq!(
        (withdrawn.id, withdrawn.version, &withdrawn.cards),
        (sun.id, sun.version, &sun.cards),
        "withdrawn is all that happened to it"
    );
    let listed = precons::shared_decks(db).await.unwrap();
    assert!(
        listed.iter().all(|deck| deck.id != sun.id),
        "a withdrawn deck is not offered"
    );
    assert!(
        listed
            .iter()
            .any(|deck| deck.source.as_deref() == Some(EST.key))
    );
    assert_eq!(
        listed
            .iter()
            .filter(|deck| deck.kind == deck::KIND_HOUSE)
            .count(),
        9,
        "the house's own decks are no sync's"
    );
    let kept = Deck::find_by_id(copy.id)
        .one(db)
        .await
        .unwrap()
        .expect("the copy");
    assert_eq!(
        kept.copied_from,
        Some(sun.id),
        "the copy still names its deck"
    );
    assert_eq!(kept.cards, copy.cards);

    let back = precons::sync(db, &[EST, SUN])
        .await
        .expect("a sync with it again");
    assert_eq!((back.returned, back.added, back.withdrawn), (1, 0, 0));
    let returned = synced_from(db, SUN.key).await;
    assert!(returned.offered);
    assert_eq!((returned.id, returned.version), (sun.id, sun.version));

    sandbox.close().await;
}

/// New cards in a list are a new version of its deck, and the state they
/// replace is kept as a player's save keeps it: a copy names the version it
/// came from, and that version stays readable.
#[tokio::test]
async fn a_precon_whose_cards_change_keeps_the_state_it_left() {
    let sandbox = Sandbox::open("precon_version").await;
    let db = &sandbox.db;
    precons::sync(db, &[SUN]).await.expect("the first sync");
    let before = synced_from(db, SUN.key).await;

    let later = precons::sync(db, &[SUN_LATER])
        .await
        .expect("the later list");
    assert_eq!((later.changed, later.added, later.withdrawn), (1, 0, 0));
    let after = synced_from(db, SUN.key).await;
    assert_eq!(after.id, before.id, "the same deck");
    assert_eq!(after.version, 2);
    assert_eq!(after.cards, ["1 Sol Ring (C18) 222", "3 Plains"]);
    let past = DeckVersion::find_by_id((before.id, 1))
        .one(db)
        .await
        .unwrap()
        .expect("version 1 is kept");
    assert_eq!(past.cards, before.cards);
    assert_eq!(past.summary.as_deref(), Some("MTGJSON 2.0.0"));

    sandbox.close().await;
}

/// Two gateways starting at once against one database: one writes, the
/// other waits for it and finds nothing left to do, and each list is still
/// one deck.
#[tokio::test]
async fn two_syncs_at_once_write_each_deck_once() {
    let sandbox = Sandbox::open("precon_race").await;
    let db = &sandbox.db;
    let (one, two) = tokio::join!(
        precons::sync(db, &[EST, SUN]),
        precons::sync(db, &[EST, SUN])
    );
    let (one, two) = (one.expect("one sync"), two.expect("the other"));
    assert_eq!(one.added + two.added, 2, "{one:?} {two:?}");
    assert_ne!(one.unchanged, two.unchanged, "{one:?} {two:?}");
    assert_eq!(synced(db).await.len(), 2);

    sandbox.close().await;
}

/// Only a deck that belongs to nobody is a sync's to own or to withdraw; the
/// database refuses either on a player's deck.
#[tokio::test]
async fn only_a_shared_deck_carries_a_source_or_leaves_the_offer() {
    let sandbox = Sandbox::open("precon_checks").await;
    let db = &sandbox.db;
    let player = an_account("owner@example.com");
    let Set(player_id) = player.id else {
        unreachable!()
    };
    Account::insert(player).exec(db).await.unwrap();

    let mut sourced = a_deck(Some(player_id), "mine");
    sourced.source = Set(Some("E02/sun-empire".to_owned()));
    assert!(Deck::insert(sourced).exec(db).await.is_err());
    let mut hidden = a_deck(Some(player_id), "mine");
    hidden.offered = Set(false);
    assert!(Deck::insert(hidden).exec(db).await.is_err());
    Deck::insert(a_deck(Some(player_id), "mine"))
        .exec(db)
        .await
        .expect("a player's deck as it always was");

    sandbox.close().await;
}

/// The columns in this schema that point at an account without an index
/// whose first column is that column, as `table.column`.
async fn unindexed_account_links(db: &DatabaseConnection) -> Vec<String> {
    db.query_all_raw(Statement::from_string(
        DbBackend::Postgres,
        "SELECT c.conrelid::regclass::text || '.' || a.attname AS link \
         FROM pg_constraint c \
         JOIN pg_attribute a ON a.attrelid = c.conrelid AND a.attnum = c.conkey[1] \
         WHERE c.contype = 'f' \
           AND c.connamespace = current_schema()::regnamespace \
           AND c.confrelid = format('%I.account', current_schema())::regclass \
           AND cardinality(c.conkey) = 1 \
           AND NOT EXISTS (SELECT 1 FROM pg_index i \
                           WHERE i.indrelid = c.conrelid AND i.indkey[0] = c.conkey[1]) \
         ORDER BY 1",
    ))
    .await
    .expect("asking for the links")
    .into_iter()
    .map(|row| row.try_get::<String>("", "link").expect("a link"))
    .collect()
}

/// Every table that follows an account away (`ON DELETE CASCADE`) or
/// forgets it (`SET NULL`) is indexed on the account: deleting one, and the
/// guest purge deleting thousands, asks each of them for the account's rows.
/// The sessions were the one that was not, and a purge of 2 000 guests
/// among 50 000 scanned every session 2 000 times (9.1 s, 76 ms indexed).
/// Stepping the index's migration back shows the gap again.
#[tokio::test]
async fn every_link_to_an_account_is_indexed() {
    let sandbox = Sandbox::open("account-links").await;
    assert_eq!(
        unindexed_account_links(&sandbox.db).await,
        Vec::<String>::new()
    );

    // Back to before the fourteenth migration, counted from the list, so the
    // migrations after it are stepped back too.
    let back = u32::try_from(Migrator::migrations().len() - 13).expect("a handful");
    Migrator::down(&sandbox.db, Some(back))
        .await
        .expect("the index steps back");
    assert_eq!(
        unindexed_account_links(&sandbox.db).await,
        vec!["session_token.account_id".to_string()],
        "the reader finds the gap the index closes"
    );
    Migrator::up(&sandbox.db, None)
        .await
        .expect("and forward again");
    assert_eq!(
        unindexed_account_links(&sandbox.db).await,
        Vec::<String>::new()
    );

    sandbox.close().await;
}

/// Replays (beta.8): the migration adds exactly the columns the list reads,
/// with the shape the statements rely on.
#[tokio::test]
async fn the_replay_columns_have_the_shape_the_list_reads() {
    let sandbox = Sandbox::open("replay_columns").await;
    let rows = sandbox
        .db
        .query_all_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT table_name || '.' || column_name AS name, data_type, is_nullable \
             FROM information_schema.columns \
             WHERE table_schema = current_schema() \
               AND table_name IN ('game_record', 'game_record_seat') \
               AND column_name IN ('build', 'checked', 'allow_spectators', 'winners', 'turn', \
                                   'end_reason', 'seated', 'colours', 'commander') \
             ORDER BY 1",
        ))
        .await
        .unwrap();
    let found: Vec<(String, String, String)> = rows
        .iter()
        .map(|row| {
            (
                row.try_get("", "name").unwrap(),
                row.try_get("", "data_type").unwrap(),
                row.try_get("", "is_nullable").unwrap(),
            )
        })
        .collect();
    let expected: Vec<(String, String, String)> = [
        ("game_record.allow_spectators", "boolean", "YES"),
        ("game_record.build", "text", "YES"),
        ("game_record.checked", "boolean", "NO"),
        ("game_record.end_reason", "text", "YES"),
        ("game_record.turn", "integer", "YES"),
        ("game_record.winners", "ARRAY", "YES"),
        ("game_record_seat.colours", "text", "YES"),
        ("game_record_seat.commander", "integer", "YES"),
        ("game_record_seat.seated", "text", "YES"),
    ]
    .map(|(a, b, c)| (a.to_owned(), b.to_owned(), c.to_owned()))
    .to_vec();
    assert_eq!(found, expected, "the nine columns of migration 18");

    // A record that says nothing of them reads as a record from before.
    baylee_db::records::open(&sandbox.db, "plain", &[baylee_db::records::Seat::default()])
        .await
        .unwrap();
    let row = sandbox
        .db
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT r.build IS NULL AS no_build, r.checked, r.allow_spectators IS NULL AS no_spec, \
                    r.winners IS NULL AS no_winners, s.seated IS NULL AS no_seated, \
                    s.colours IS NULL AS no_colours, s.commander IS NULL AS no_commander \
             FROM game_record r JOIN game_record_seat s USING (game_id)",
        ))
        .await
        .unwrap()
        .unwrap();
    assert!(!row.try_get::<bool>("", "checked").unwrap());
    for column in [
        "no_build",
        "no_spec",
        "no_winners",
        "no_seated",
        "no_colours",
        "no_commander",
    ] {
        assert!(row.try_get::<bool>("", column).unwrap(), "{column}");
    }

    sandbox.close().await;
}

/// What a game says when it starts and when it ends comes back in the list,
/// newest first, with the seats' handles as they are now.
#[tokio::test]
#[allow(clippy::too_many_lines)] // one scenario, read top to bottom
async fn a_players_games_are_listed_newest_first_with_what_the_engine_said() {
    use baylee_db::records::{self, ListedSeat, Opening, Seat, SeatFacts, Seated};

    let sandbox = Sandbox::open("replay_list").await;
    let db = &sandbox.db;
    let (alice, bob, carol) = (
        an_account("alice@example.com"),
        an_account("bob@example.com"),
        an_account("carol@example.com"),
    );
    let (a, b, c) = (
        alice.id.clone().unwrap(),
        bob.id.clone().unwrap(),
        carol.id.clone().unwrap(),
    );
    for account in [alice, bob, carol] {
        Account::insert(account).exec(db).await.expect("an account");
    }
    let tag_of = |id: Uuid| {
        let db = db.clone();
        async move {
            db.query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT tag FROM account WHERE id = $1",
                [id.into()],
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get::<i32>("", "tag")
            .unwrap()
        }
    };
    let (tag_a, tag_b) = (tag_of(a).await, tag_of(b).await);

    let person = |colours: Option<&str>, commander: Option<u32>| SeatFacts {
        seated: Seated::Person,
        colours: colours.map(str::to_owned),
        commander,
    };
    let build = "0.8.0+build.212 (3f9a1c7e21)";
    records::open_with(
        db,
        "old",
        &Opening {
            build: Some(build.to_owned()),
            checked: true,
            allow_spectators: Some(true),
        },
        &[
            (Seat::played_by(a), person(Some("UB"), None)),
            (Seat::played_by(b), person(Some("G"), Some(4242))),
            (
                Seat::default(),
                SeatFacts {
                    seated: Seated::House,
                    colours: Some(String::new()),
                    commander: None,
                },
            ),
        ],
    )
    .await
    .unwrap();
    // A game of a seat bridge: Alice answers for seat 1, plays seat 0.
    records::open_with(
        db,
        "new",
        &Opening::default(),
        &[
            (Seat::played_by(a), SeatFacts::default()),
            (
                Seat {
                    account: None,
                    delegated_by: Some(a),
                },
                SeatFacts {
                    seated: Seated::Model,
                    ..SeatFacts::default()
                },
            ),
        ],
    )
    .await
    .unwrap();
    // Alice only answered for the bridge here: not a game she sat at.
    records::open_with(
        db,
        "bridge-only",
        &Opening::default(),
        &[
            (
                Seat {
                    account: None,
                    delegated_by: Some(a),
                },
                SeatFacts {
                    seated: Seated::Model,
                    ..SeatFacts::default()
                },
            ),
            (Seat::played_by(c), SeatFacts::default()),
        ],
    )
    .await
    .unwrap();
    // Two games begun in the same instant: told apart by their ids.
    for game in ["tie-b", "tie-a"] {
        records::open_with(
            db,
            game,
            &Opening::default(),
            &[(Seat::played_by(c), SeatFacts::default())],
        )
        .await
        .unwrap();
    }
    for (game, at) in [
        ("old", "2026-10-01T10:00:00Z"),
        ("new", "2026-10-02T11:30:05Z"),
        ("bridge-only", "2026-10-03T00:00:00Z"),
        ("tie-a", "2026-10-04T00:00:00Z"),
        ("tie-b", "2026-10-04T00:00:00Z"),
    ] {
        db.execute_unprepared(&format!(
            "UPDATE game_record SET started_at = '{at}' WHERE game_id = '{game}'"
        ))
        .await
        .unwrap();
    }
    // "old" is played to its end, "new" is not.
    assert!(
        records::append(db, "old", 0, b"hello ".to_vec(), false)
            .await
            .unwrap()
    );
    assert!(
        records::append(db, "old", 1, b"world!".to_vec(), true)
            .await
            .unwrap()
    );
    records::finish(db, "old", &[1], Some(7), "Conceded")
        .await
        .unwrap();

    // Alice: the two she sat at, the newer first; the bridge game is not
    // hers, and what she is told of each is what was stored.
    let (games, total) = records::seated_games(db, a, false, 0, 10).await.unwrap();
    assert_eq!(total, 2);
    assert_eq!(
        games.iter().map(|g| g.game_id.as_str()).collect::<Vec<_>>(),
        ["new", "old"]
    );
    let old = &games[1];
    assert_eq!(old.started_at, "2026-10-01T10:00:00Z");
    assert!(
        old.ended_at
            .as_deref()
            .is_some_and(|t| t.len() == 20 && t.ends_with('Z'))
    );
    assert!(old.complete);
    assert_eq!(old.build.as_deref(), Some(build));
    assert!(old.checked);
    assert_eq!(old.allow_spectators, Some(true));
    assert_eq!(old.winners, Some(vec![1]));
    assert_eq!(old.turn, Some(7));
    assert_eq!(old.end_reason.as_deref(), Some("Conceded"));
    assert_eq!(old.bytes, 12);
    // Started on 2026-10-01, before the migration that stores the cutoff
    // ran in this schema: nobody said, so no seat is shared.
    assert_eq!(
        old.seats,
        vec![
            ListedSeat {
                seat: 0,
                account: Some(a),
                delegated_by: None,
                seated: Seated::Person,
                name: Some(("alice".to_owned(), tag_a)),
                colours: Some("UB".to_owned()),
                commander: None,
                shared: false,
            },
            ListedSeat {
                seat: 1,
                account: Some(b),
                delegated_by: None,
                seated: Seated::Person,
                name: Some(("bob".to_owned(), tag_b)),
                colours: Some("G".to_owned()),
                commander: Some(4242),
                shared: false,
            },
            ListedSeat {
                seat: 2,
                account: None,
                delegated_by: None,
                seated: Seated::House,
                name: None,
                colours: Some(String::new()),
                commander: None,
                shared: false,
            },
        ]
    );
    let new = &games[0];
    assert_eq!(new.started_at, "2026-10-02T11:30:05Z");
    assert!(!new.complete);
    assert_eq!(new.ended_at, None);
    assert_eq!(new.build, None);
    assert!(!new.checked);
    assert_eq!(new.allow_spectators, None);
    assert_eq!(new.winners, None);
    assert_eq!(new.turn, None);
    assert_eq!(new.end_reason, None);
    assert_eq!(new.bytes, 0);
    assert_eq!(new.seats.len(), 2);
    assert_eq!(new.seats[0].seated, Seated::Unknown, "none was said");
    assert_eq!(new.seats[0].colours, None);
    assert_eq!(new.seats[1].seated, Seated::Model);
    assert_eq!(new.seats[1].delegated_by, Some(a));
    assert_eq!(new.seats[1].name, None);

    // Pages: offset and limit, the total of all of them.
    let (page, total) = records::seated_games(db, a, false, 1, 1).await.unwrap();
    assert_eq!((page.len(), total), (1, 2));
    assert_eq!(page[0].game_id, "old");
    let (page, total) = records::seated_games(db, a, false, 2, 10).await.unwrap();
    assert_eq!((page.len(), total), (0, 2));

    // Bob has the one; Carol three, the two begun together by id.
    let (games, total) = records::seated_games(db, b, false, 0, 10).await.unwrap();
    assert_eq!((games.len(), total), (1, 1));
    let (games, total) = records::seated_games(db, c, false, 0, 10).await.unwrap();
    assert_eq!(total, 3);
    assert_eq!(
        games.iter().map(|g| g.game_id.as_str()).collect::<Vec<_>>(),
        ["tie-a", "tie-b", "bridge-only"]
    );
    // And somebody who sat nowhere has none.
    let (games, total) = records::seated_games(db, Uuid::now_v7(), false, 0, 10)
        .await
        .unwrap();
    assert_eq!((games.len(), total), (0, 0));

    // The seat an account played, never a bridge's, and the lowest of two.
    assert_eq!(records::seat_of(db, "old", a).await.unwrap(), Some(0));
    assert_eq!(records::seat_of(db, "old", b).await.unwrap(), Some(1));
    assert_eq!(records::seat_of(db, "old", c).await.unwrap(), None);
    assert_eq!(records::seat_of(db, "bridge-only", a).await.unwrap(), None);
    assert_eq!(
        records::seat_of(db, "bridge-only", c).await.unwrap(),
        Some(1)
    );
    assert_eq!(records::seat_of(db, "nowhere", a).await.unwrap(), None);
    records::open(
        db,
        "two-chairs",
        &[Seat::default(), Seat::played_by(a), Seat::played_by(a)],
    )
    .await
    .unwrap();
    assert_eq!(
        records::seat_of(db, "two-chairs", a).await.unwrap(),
        Some(1)
    );

    // Every seat of a game, as the list shows them; none for no game.
    assert_eq!(records::seats_of(db, "old").await.unwrap(), old.seats);
    assert_eq!(records::seats_of(db, "nowhere").await.unwrap(), Vec::new());

    // The size and the record the job reads, whoever sat there.
    assert_eq!(
        records::stored_size(db, "old").await.unwrap(),
        Some((12, true))
    );
    assert_eq!(
        records::stored_size(db, "new").await.unwrap(),
        Some((0, false))
    );
    assert_eq!(records::stored_size(db, "nowhere").await.unwrap(), None);
    assert_eq!(
        records::for_projection(db, "old").await.unwrap(),
        Some(records::Projectable {
            data: b"hello world!".to_vec(),
            complete: true,
            build: Some(build.to_owned()),
        })
    );
    assert_eq!(
        records::for_projection(db, "new").await.unwrap(),
        Some(records::Projectable {
            data: Vec::new(),
            complete: false,
            build: None,
        })
    );
    assert_eq!(records::for_projection(db, "nowhere").await.unwrap(), None);

    // Alice goes: Bob's list keeps the game, her seat a person's with no
    // account and no name, and the deck's colours stay (card data).
    baylee_db::accounts::delete(db, a).await.unwrap();
    let (games, total) = records::seated_games(db, b, false, 0, 10).await.unwrap();
    assert_eq!(total, 1);
    let gone = &games[0].seats[0];
    assert_eq!(gone.account, None);
    assert_eq!(gone.name, None);
    assert_eq!(
        gone.seated,
        Seated::Person,
        "a former player, not the house"
    );
    assert_eq!(gone.colours.as_deref(), Some("UB"));
    assert_eq!(
        games[0].seats[1].name.as_ref().map(|n| n.0.as_str()),
        Some("bob")
    );
    assert_eq!(records::seat_of(db, "old", a).await.unwrap(), None);

    sandbox.close().await;
}

/// A game starting twice rewrites nothing, an end is noted as given (a draw
/// is an end with no winner), and the database refuses what the columns
/// must never hold.
#[tokio::test]
#[allow(clippy::too_many_lines)] // one scenario, read top to bottom
async fn the_opening_is_kept_the_end_is_noted_and_bad_values_are_refused() {
    use baylee_db::records::{self, Opening, Seat, SeatFacts, Seated};

    let sandbox = Sandbox::open("replay_checks").await;
    let db = &sandbox.db;
    let me = an_account("checks@example.com");
    let me_id = me.id.clone().unwrap();
    Account::insert(me).exec(db).await.expect("an account");
    let facts = |seated, colours: &str, commander| SeatFacts {
        seated,
        colours: Some(colours.to_owned()),
        commander,
    };
    let count = |table: &'static str, game: &'static str| async move {
        db.query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            format!("SELECT count(*) AS n FROM {table} WHERE game_id = '{game}'"),
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<i64>("", "n")
        .unwrap()
    };

    // Opened again, a game keeps the build and the seats it began with.
    let first = Opening {
        build: Some("first".to_owned()),
        checked: true,
        allow_spectators: Some(false),
    };
    records::open_with(
        db,
        "g",
        &first,
        &[(Seat::played_by(me_id), facts(Seated::Person, "W", Some(1)))],
    )
    .await
    .unwrap();
    records::open_with(
        db,
        "g",
        &Opening {
            build: Some("second".to_owned()),
            checked: false,
            allow_spectators: Some(true),
        },
        &[
            (Seat::default(), facts(Seated::House, "R", Some(2))),
            (Seat::default(), facts(Seated::House, "R", Some(2))),
        ],
    )
    .await
    .unwrap();
    let (games, _) = records::seated_games(db, me_id, false, 0, 10)
        .await
        .unwrap();
    assert_eq!(games[0].build.as_deref(), Some("first"));
    assert!(games[0].checked);
    assert_eq!(games[0].allow_spectators, Some(false));
    assert_eq!(games[0].seats.len(), 1);
    assert_eq!(games[0].seats[0].commander, Some(1));

    // How a game ended is noted as given: a draw has no winner, which is a
    // list and not a missing one; a team win has several; no turn is none.
    records::append(db, "g", 0, b"x".to_vec(), true)
        .await
        .unwrap();
    records::finish(db, "g", &[], None, "Draw").await.unwrap();
    let ended = |game: &'static str| async move {
        records::seated_games(db, me_id, false, 0, 10)
            .await
            .unwrap()
            .0
            .into_iter()
            .find(|g| g.game_id == game)
            .unwrap()
    };
    let draw = ended("g").await;
    assert_eq!(draw.winners, Some(Vec::new()));
    assert_eq!(draw.turn, None);
    assert_eq!(draw.end_reason.as_deref(), Some("Draw"));
    records::finish(db, "g", &[0, 2], Some(31), "Won")
        .await
        .unwrap();
    let team = ended("g").await;
    assert_eq!(team.winners, Some(vec![0, 2]));
    assert_eq!(team.turn, Some(31));
    assert_eq!(team.end_reason.as_deref(), Some("Won"));
    // A game with no record is left alone, not made.
    records::finish(db, "nowhere", &[0], Some(1), "Won")
        .await
        .unwrap();
    assert_eq!(count("game_record", "nowhere").await, 0);
    // And a number the column cannot hold is an error, not a wrap.
    assert!(
        records::finish(db, "g", &[0], Some(u32::MAX), "Won")
            .await
            .is_err()
    );
    assert_eq!(ended("g").await.turn, Some(31), "nothing changed");

    // The colour identity is WUBRG letters in that order, or nothing.
    for (i, good) in ["", "W", "U", "WU", "WBG", "UBRG", "WUBRG"]
        .iter()
        .enumerate()
    {
        let game = format!("good-{i}");
        records::open_with(
            db,
            &game,
            &Opening::default(),
            &[(Seat::played_by(me_id), facts(Seated::Person, good, None))],
        )
        .await
        .unwrap_or_else(|e| panic!("{good:?}: {e}"));
    }
    for (i, bad) in ["GW", "wubrg", "WW", "X", "W U", "WUBRGW", "BU"]
        .iter()
        .enumerate()
    {
        let game = format!("bad-{i}");
        let refused = records::open_with(
            db,
            &game,
            &Opening::default(),
            &[(Seat::played_by(me_id), facts(Seated::Person, bad, None))],
        )
        .await;
        assert!(refused.is_err(), "{bad:?} was stored");
        // Its transaction went with it: no half-opened record.
        let row = db
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT count(*) AS n FROM game_record WHERE game_id = $1",
                [game.into()],
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            row.try_get::<i64>("", "n").unwrap(),
            0,
            "{bad:?} left a row"
        );
    }
    // A commander the column cannot hold is refused too.
    assert!(
        records::open_with(
            db,
            "huge",
            &Opening::default(),
            &[(Seat::default(), facts(Seated::House, "", Some(u32::MAX)))],
        )
        .await
        .is_err()
    );

    // Who sat is one of three words, or not said.
    for bad in ["robot", "Person", "", "human"] {
        let refused = db
            .execute_unprepared(&format!(
                "UPDATE game_record_seat SET seated = '{bad}' WHERE game_id = 'g'"
            ))
            .await;
        assert!(refused.is_err(), "{bad:?} was stored");
    }
    for good in ["person", "house", "model"] {
        db.execute_unprepared(&format!(
            "UPDATE game_record_seat SET seated = '{good}' WHERE game_id = 'g'"
        ))
        .await
        .unwrap_or_else(|e| panic!("{good:?}: {e}"));
    }
    // Not said reads as unknown.
    db.execute_unprepared("UPDATE game_record_seat SET seated = NULL WHERE game_id = 'g'")
        .await
        .unwrap();
    assert_eq!(ended("g").await.seats[0].seated, Seated::Unknown);
    // Words the code does not know (a later build's) read as unknown too,
    // rather than as a person: the CHECK is the migration's, the reader's
    // fallback is its own.
    db.execute_unprepared("ALTER TABLE game_record_seat DROP CONSTRAINT game_record_seat_seated")
        .await
        .unwrap();
    db.execute_unprepared("UPDATE game_record_seat SET seated = 'spectre' WHERE game_id = 'g'")
        .await
        .unwrap();
    assert_eq!(ended("g").await.seats[0].seated, Seated::Unknown);

    sandbox.close().await;
}

/// Rows of `table` for `game_id`.
async fn rows_of(db: &DatabaseConnection, table: &str, game_id: &str) -> i64 {
    db.query_one_raw(Statement::from_string(
        DbBackend::Postgres,
        format!("SELECT count(*) AS n FROM {table} WHERE game_id = '{game_id}'"),
    ))
    .await
    .unwrap()
    .unwrap()
    .try_get("", "n")
    .unwrap()
}

/// A count, from any statement that answers one `n`.
async fn count(db: &DatabaseConnection, sql: &str) -> i64 {
    db.query_one_raw(Statement::from_string(
        DbBackend::Postgres,
        format!("SELECT count(*) AS n FROM ({sql}) AS q"),
    ))
    .await
    .unwrap()
    .unwrap()
    .try_get("", "n")
    .unwrap()
}

/// Replays (13.10.2026): a seat's consent to show its hidden cards is one
/// row per game and seat, written over by the next word; a seat that never
/// said is shared in a game recorded from the cutoff on.
#[tokio::test]
#[allow(clippy::too_many_lines)] // one scenario, read top to bottom
async fn a_seats_consent_is_one_row_per_game_and_seat_and_none_follows_the_cutoff() {
    use baylee_db::records::{self, Seat};

    let sandbox = Sandbox::open("replay_share").await;
    let db = &sandbox.db;
    let (alice, bob) = (an_account("a@example.com"), an_account("b@example.com"));
    let (a, b) = (alice.id.clone().unwrap(), bob.id.clone().unwrap());
    for account in [alice, bob] {
        Account::insert(account).exec(db).await.expect("an account");
    }
    for game in ["g", "h"] {
        records::open(
            db,
            game,
            &[Seat::played_by(a), Seat::played_by(b), Seat::default()],
        )
        .await
        .unwrap();
    }
    let shared = |game: &'static str| async move {
        records::seats_of(db, game)
            .await
            .unwrap()
            .into_iter()
            .map(|seat| seat.shared)
            .collect::<Vec<_>>()
    };
    // Recorded after the cutoff (the migration ran before the test began),
    // nobody having said: everyone shares, and no row says so.
    assert_eq!(shared("g").await, [true, true, true], "nobody has said");
    assert_eq!(rows_of(db, "game_record_share", "g").await, 0);

    // Bob unticks: his seat reads false, no other seat and no other game
    // moves.
    assert!(records::set_share(db, "g", 1, false).await.unwrap());
    assert_eq!(shared("g").await, [true, false, true]);
    assert_eq!(shared("h").await, [true, true, true]);
    // And again: the same row.
    assert!(records::set_share(db, "g", 1, false).await.unwrap());
    assert_eq!(rows_of(db, "game_record_share", "g").await, 1);
    // He ticks it again: written over, not added, and the row says so.
    assert!(records::set_share(db, "g", 1, true).await.unwrap());
    assert_eq!(shared("g").await, [true, true, true]);
    assert_eq!(rows_of(db, "game_record_share", "g").await, 1);
    assert_eq!(
        count(db, "SELECT 1 FROM game_record_share WHERE shared").await,
        1,
        "an explicit yes is a row, not the default"
    );
    assert!(records::set_share(db, "g", 1, false).await.unwrap());
    // Alice says too: her own row.
    assert!(records::set_share(db, "g", 0, false).await.unwrap());
    assert_eq!(shared("g").await, [false, false, true]);
    assert_eq!(rows_of(db, "game_record_share", "g").await, 2);

    // The list reads the same.
    let (games, _) = records::seated_games(db, a, false, 0, 10).await.unwrap();
    let g = games.iter().find(|g| g.game_id == "g").unwrap();
    assert_eq!(
        g.seats.iter().map(|s| s.shared).collect::<Vec<_>>(),
        [false, false, true]
    );

    // A game with no record: nothing is written.
    assert!(!records::set_share(db, "nowhere", 0, false).await.unwrap());
    assert_eq!(rows_of(db, "game_record_share", "nowhere").await, 0);
    assert_eq!(count(db, "SELECT 1 FROM game_record_share").await, 2);

    // A seat is a number of a set: 0 to 15 (a seat the table lacks is the
    // caller's to refuse, as `seat_of` does).
    assert!(records::set_share(db, "g", 15, true).await.unwrap());
    assert!(
        records::set_share(db, "g", 16, true).await.is_err(),
        "a seat past a set's bound"
    );
    for bad in [-1_i32, 16, 300] {
        let refused = db
            .execute_unprepared(&format!(
                "INSERT INTO game_record_share (game_id, seat) VALUES ('h', {bad})"
            ))
            .await;
        assert!(refused.is_err(), "seat {bad}");
    }
    db.execute_unprepared("INSERT INTO game_record_share (game_id, seat) VALUES ('h', 0)")
        .await
        .expect("seat 0 is a seat; shared defaults to true");
    assert_eq!(shared("h").await, [true, true, true]);
    assert!(
        db.execute_unprepared("INSERT INTO game_record_share (game_id, seat) VALUES ('h', 0)")
            .await
            .is_err(),
        "one row per game and seat"
    );

    sandbox.close().await;
}

/// Replays (13.10.2026): a seat that never said is shared in a game recorded
/// from the cutoff on, not shared in one recorded before it, and an explicit
/// word beats either default.
#[tokio::test]
#[allow(clippy::too_many_lines)] // one scenario, read top to bottom
async fn a_seat_that_never_said_is_shared_from_the_cutoff_on_and_not_before() {
    use baylee_db::records::{self, Seat};

    let sandbox = Sandbox::open("replay_cutoff").await;
    let db = &sandbox.db;
    let (alice, bob) = (an_account("a@example.com"), an_account("b@example.com"));
    let (a, b) = (alice.id.clone().unwrap(), bob.id.clone().unwrap());
    for account in [alice, bob] {
        Account::insert(account).exec(db).await.expect("an account");
    }
    for game in ["early", "exact", "late"] {
        records::open(db, game, &[Seat::played_by(a), Seat::played_by(b)])
            .await
            .unwrap();
    }
    let at = |game: &'static str, when: &'static str| async move {
        db.execute_unprepared(&format!(
            "UPDATE game_record SET started_at = {when} WHERE game_id = '{game}'"
        ))
        .await
    };
    at(
        "early",
        "(SELECT at FROM replay_share_cutoff) - interval '1 microsecond'",
    )
    .await
    .unwrap();
    at("exact", "(SELECT at FROM replay_share_cutoff)")
        .await
        .unwrap();
    at(
        "late",
        "(SELECT at FROM replay_share_cutoff) + interval '1 hour'",
    )
    .await
    .unwrap();
    let shared = |game: &'static str| async move {
        records::seats_of(db, game)
            .await
            .unwrap()
            .into_iter()
            .map(|seat| seat.shared)
            .collect::<Vec<_>>()
    };
    assert_eq!(shared("early").await, [false, false], "before: not shared");
    assert_eq!(shared("exact").await, [true, true], "at the cutoff: shared");
    assert_eq!(shared("late").await, [true, true], "after: shared");
    // And in the list, which is the other reader of the same column.
    let (games, _) = records::seated_games(db, a, false, 0, 10).await.unwrap();
    let flags = |id: &str| -> Vec<bool> {
        games
            .iter()
            .find(|g| g.game_id == id)
            .unwrap()
            .seats
            .iter()
            .map(|s| s.shared)
            .collect()
    };
    assert_eq!(flags("early"), [false, false]);
    assert_eq!(flags("exact"), [true, true]);
    assert_eq!(flags("late"), [true, true]);

    // Explicit words beat the default both ways, for the seat that spoke.
    assert!(records::set_share(db, "early", 1, true).await.unwrap());
    assert!(records::set_share(db, "late", 0, false).await.unwrap());
    assert_eq!(shared("early").await, [false, true]);
    assert_eq!(shared("late").await, [false, true]);
    // Saying no where the default is no is a row, and stays no.
    assert!(records::set_share(db, "early", 0, false).await.unwrap());
    assert_eq!(shared("early").await, [false, true]);

    // The cutoff is a moment, not a flag: moved past every game, every
    // unsaid seat is not shared; moved back before them, all are.
    db.execute_unprepared("UPDATE replay_share_cutoff SET at = now() + interval '1 day'")
        .await
        .unwrap();
    assert_eq!(shared("exact").await, [false, false]);
    assert_eq!(shared("late").await, [false, false], "what was said stands");
    db.execute_unprepared("UPDATE replay_share_cutoff SET at = '-infinity'")
        .await
        .unwrap();
    assert_eq!(shared("early").await, [false, true]);
    assert_eq!(shared("exact").await, [true, true]);
    // A schema with no cutoff row (nothing deletes it) reads every game as
    // recorded after it, rather than failing.
    db.execute_unprepared("DELETE FROM replay_share_cutoff")
        .await
        .unwrap();
    assert_eq!(
        shared("exact").await,
        [true, true],
        "-infinity: all after it"
    );

    sandbox.close().await;
}

/// Replays (13.10.2026): the migration stores the cutoff once, at the moment
/// it runs, and a game recorded before it reads as not shared where nobody
/// said.
#[tokio::test]
async fn the_cutoff_is_written_once_by_the_migration_that_adds_it() {
    use baylee_db::records::{self, Seat};

    // The schema as it was before the consent tables, holding a game.
    let before = u32::try_from(
        Migrator::migrations()
            .iter()
            .position(|migration| migration.name() == "m20261013_000019_replay_shares")
            .expect("replay consent migration registered"),
    )
    .expect("a handful");
    let sandbox = Sandbox::open_at("replay_cutoff_once", Some(before)).await;
    let db = &sandbox.db;
    let alice = an_account("a@example.com");
    let a = alice.id.clone().unwrap();
    Account::insert(alice).exec(db).await.expect("an account");
    records::open(db, "old", &[Seat::played_by(a), Seat::default()])
        .await
        .unwrap();
    assert_eq!(
        count(db, "SELECT 1 FROM information_schema.tables WHERE table_schema = current_schema() \
                   AND table_name IN ('replay_share_cutoff', 'game_record_share', 'replay_archived')")
            .await,
        0,
        "the consent tables are not there yet"
    );

    Migrator::up(db, None)
        .await
        .expect("the remaining migrations");
    let cutoff = |db: &DatabaseConnection| {
        let db = db.clone();
        async move {
            db.query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT at::text AS at, count(*) OVER () AS n FROM replay_share_cutoff",
            ))
            .await
            .unwrap()
            .map(|row| {
                (
                    row.try_get::<String>("", "at").unwrap(),
                    row.try_get::<i64>("", "n").unwrap(),
                )
            })
        }
    };
    let (first, rows) = cutoff(db).await.expect("the migration wrote the cutoff");
    assert_eq!(rows, 1);
    assert_eq!(
        count(
            db,
            "SELECT 1 FROM replay_share_cutoff WHERE at > now() - interval '1 minute' AND at <= now()"
        )
        .await,
        1,
        "the moment the migration ran"
    );

    // The game recorded before it: nobody said, so nobody shared. A game
    // recorded after: shared.
    let shared = |game: &'static str| async move {
        records::seats_of(db, game)
            .await
            .unwrap()
            .into_iter()
            .map(|seat| seat.shared)
            .collect::<Vec<_>>()
    };
    assert_eq!(shared("old").await, [false, false]);
    records::open(db, "new", &[Seat::played_by(a), Seat::default()])
        .await
        .unwrap();
    assert_eq!(shared("new").await, [true, true]);

    // Run again, as a restart does: the first moment stays.
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    Migrator::up(db, None).await.expect("a second migrate");
    assert_eq!(cutoff(db).await, Some((first.clone(), 1)));
    // The migration's own statements, run a second time on the live schema
    // (each is idempotent): the same.
    for statement in [
        "CREATE TABLE IF NOT EXISTS replay_share_cutoff ( \
             only_row boolean PRIMARY KEY DEFAULT true \
                 CONSTRAINT replay_share_cutoff_one CHECK (only_row), \
             at timestamptz NOT NULL DEFAULT now() )",
        "INSERT INTO replay_share_cutoff (only_row) VALUES (true) ON CONFLICT DO NOTHING",
    ] {
        db.execute_unprepared(statement).await.unwrap();
    }
    assert_eq!(cutoff(db).await, Some((first, 1)));
    assert_eq!(shared("old").await, [false, false]);
    assert_eq!(shared("new").await, [true, true]);

    // It is one row, whatever is written.
    assert!(
        db.execute_unprepared("INSERT INTO replay_share_cutoff (only_row) VALUES (false)")
            .await
            .is_err(),
        "the check admits only the one row"
    );
    assert!(
        db.execute_unprepared(
            "INSERT INTO replay_share_cutoff (only_row, at) VALUES (true, now())"
        )
        .await
        .is_err(),
        "and the key admits it once"
    );

    sandbox.close().await;
}

/// Replays (13.10.2026): archiving a game moves it to one account's archived
/// list and takes it off its main one, and does nothing else; the record, the
/// seats and every other account's lists stay.
#[tokio::test]
#[allow(clippy::too_many_lines)] // one scenario, read top to bottom
async fn archiving_a_game_moves_it_between_one_accounts_lists_and_nothing_else() {
    use baylee_db::records::{self, Seat};

    let sandbox = Sandbox::open("replay_archive").await;
    let db = &sandbox.db;
    let (alice, bob, carol) = (
        an_account("a@example.com"),
        an_account("b@example.com"),
        an_account("c@example.com"),
    );
    let (a, b, c) = (
        alice.id.clone().unwrap(),
        bob.id.clone().unwrap(),
        carol.id.clone().unwrap(),
    );
    for account in [alice, bob, carol] {
        Account::insert(account).exec(db).await.expect("an account");
    }
    for (game, at) in [
        ("g1", "2026-10-01T00:00:00Z"),
        ("g2", "2026-10-02T00:00:00Z"),
    ] {
        records::open(db, game, &[Seat::played_by(a), Seat::played_by(b)])
            .await
            .unwrap();
        db.execute_unprepared(&format!(
            "UPDATE game_record SET started_at = '{at}' WHERE game_id = '{game}'"
        ))
        .await
        .unwrap();
    }
    let listed = |who: Uuid, archived: bool, offset: u64, limit: u64| async move {
        let (games, total) = records::seated_games(db, who, archived, offset, limit)
            .await
            .unwrap();
        (
            games.into_iter().map(|g| g.game_id).collect::<Vec<_>>(),
            total,
        )
    };
    let two = || vec!["g2".to_owned(), "g1".to_owned()];
    assert_eq!(listed(a, false, 0, 10).await, (two(), 2));
    assert_eq!(listed(a, true, 0, 10).await, (Vec::new(), 0));

    records::set_archived(db, a, "g2", true).await.unwrap();
    assert_eq!(
        listed(a, false, 0, 10).await,
        (vec!["g1".into()], 1),
        "the total too"
    );
    assert_eq!(listed(a, true, 0, 10).await, (vec!["g2".into()], 1));
    assert_eq!(
        listed(a, false, 1, 10).await,
        (Vec::<String>::new(), 1),
        "pages count what is left"
    );
    assert_eq!(listed(a, true, 1, 10).await, (Vec::<String>::new(), 1));
    assert_eq!(listed(a, false, 0, 1).await, (vec!["g1".into()], 1));
    // Bob's lists are whole; Carol sat nowhere, in neither list.
    assert_eq!(listed(b, false, 0, 10).await, (two(), 2));
    assert_eq!(listed(b, true, 0, 10).await, (Vec::new(), 0));
    assert_eq!(listed(c, false, 0, 10).await, (Vec::new(), 0));
    assert_eq!(listed(c, true, 0, 10).await, (Vec::new(), 0));

    // Twice is once.
    records::set_archived(db, a, "g2", true).await.unwrap();
    assert_eq!(count(db, "SELECT 1 FROM replay_archived").await, 1);

    // Only the lists: the record, the seats and what the job reads stay, and
    // the archived game's seat is still hers (her tape is still answered).
    assert_eq!(records::seat_of(db, "g2", a).await.unwrap(), Some(0));
    assert_eq!(records::seats_of(db, "g2").await.unwrap().len(), 2);
    assert!(records::for_projection(db, "g2").await.unwrap().is_some());
    assert_eq!(rows_of(db, "game_record", "g2").await, 1);

    // Everyone archives on their own.
    records::set_archived(db, b, "g2", true).await.unwrap();
    assert_eq!(listed(b, false, 0, 10).await, (vec!["g1".into()], 1));
    assert_eq!(listed(b, true, 0, 10).await, (vec!["g2".into()], 1));
    assert_eq!(listed(a, true, 0, 10).await, (vec!["g2".into()], 1));
    // A game that is not there is no row.
    assert!(records::set_archived(db, a, "nowhere", true).await.is_err());
    // Taking out what was never put in, or a game that is not there, is
    // nothing.
    records::set_archived(db, c, "g1", false).await.unwrap();
    records::set_archived(db, a, "nowhere", false)
        .await
        .unwrap();

    // Back out: Alice's one row goes, Bob's stays; twice is once.
    records::set_archived(db, a, "g2", false).await.unwrap();
    records::set_archived(db, a, "g2", false).await.unwrap();
    assert_eq!(listed(a, false, 0, 10).await, (two(), 2));
    assert_eq!(listed(a, true, 0, 10).await, (Vec::new(), 0));
    assert_eq!(listed(b, true, 0, 10).await, (vec!["g2".into()], 1));

    // The account goes: its archive goes (the row is its own), the other
    // account's stays, the record stays.
    records::set_archived(db, a, "g1", true).await.unwrap();
    baylee_db::accounts::delete(db, a).await.unwrap();
    let rows: Vec<(Uuid, String)> = db
        .query_all_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT account_id, game_id FROM replay_archived",
        ))
        .await
        .unwrap()
        .iter()
        .map(|row| {
            (
                row.try_get("", "account_id").unwrap(),
                row.try_get("", "game_id").unwrap(),
            )
        })
        .collect();
    assert_eq!(rows, vec![(b, "g2".to_owned())]);
    assert_eq!(rows_of(db, "game_record", "g1").await, 1);

    sandbox.close().await;
}

/// Replays (13.10.2026): a consent belongs to the record, which is never
/// deleted, so it stays when its account goes, and keeps governing the seat.
#[tokio::test]
async fn a_consent_stays_when_its_account_goes() {
    use baylee_db::records::{self, Seat};

    let sandbox = Sandbox::open("replay_share_kept").await;
    let db = &sandbox.db;
    let (alice, bob) = (an_account("a@example.com"), an_account("b@example.com"));
    let (a, b) = (alice.id.clone().unwrap(), bob.id.clone().unwrap());
    for account in [alice, bob] {
        Account::insert(account).exec(db).await.expect("an account");
    }
    records::open(db, "g", &[Seat::played_by(a), Seat::played_by(b)])
        .await
        .unwrap();
    records::open(db, "early", &[Seat::played_by(a), Seat::played_by(b)])
        .await
        .unwrap();
    db.execute_unprepared(
        "UPDATE game_record SET started_at = (SELECT at FROM replay_share_cutoff) - interval '1 day' \
         WHERE game_id = 'early'",
    )
    .await
    .unwrap();
    assert!(records::set_share(db, "g", 1, false).await.unwrap());
    assert!(records::set_share(db, "g", 0, true).await.unwrap());
    assert!(records::set_share(db, "early", 1, true).await.unwrap());
    records::set_archived(db, b, "g", true).await.unwrap();

    baylee_db::accounts::delete(db, b).await.unwrap();
    assert_eq!(
        rows_of(db, "game_record_share", "g").await,
        2,
        "both consents"
    );
    assert_eq!(rows_of(db, "game_record_share", "early").await, 1);
    let seats = records::seats_of(db, "g").await.unwrap();
    assert_eq!(seats[1].account, None, "his seat is a former player's");
    assert!(!seats[1].shared, "and still his word");
    assert!(seats[0].shared);
    // What he said before the cutoff game stands too, over its default; what
    // nobody said there is still the default (Alice's).
    let early = records::seats_of(db, "early").await.unwrap();
    assert_eq!((early[0].shared, early[1].shared), (false, true));
    assert_eq!(
        count(db, "SELECT 1 FROM replay_archived").await,
        0,
        "his archive went"
    );
    // A game he left is still Alice's.
    let (games, total) = records::seated_games(db, a, false, 0, 10).await.unwrap();
    assert_eq!((games.len(), total), (2, 2));
    assert!(!games.iter().find(|g| g.game_id == "g").unwrap().seats[1].shared);

    sandbox.close().await;
}
