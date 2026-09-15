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

use baylee_db::entity::prelude::*;
use baylee_db::entity::{account, client_settings, deck, session_token};
use sea_orm::{
    ActiveValue::{NotSet, Set},
    ColumnTrait, ConnectionTrait, Database, DatabaseConnection, EntityTrait, PaginatorTrait,
    QueryFilter,
};
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
        let db = baylee_db::connect(&scoped, 2)
            .await
            .expect("migrating a fresh schema");

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
        email: Set(email.to_owned()),
        display_name: Set(display_name.to_owned()),
        // The database hands this out. Setting it here would be the one
        // way to collide with a later registration, because an explicit
        // value does not move the sequence.
        tag: NotSet,
        password_hash: Set("$argon2id$v=19$m=19456,t=2,p=1$c2FsdA$aGFzaA".to_owned()),
        created_at: Set(OffsetDateTime::now_utc()),
        confirmed_at: Set(None),
        lang: Set("de".to_owned()),
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
        account_id: Set(first.id),
        name: Set("Mono Red".to_owned()),
        cards: Set(vec!["4 Lightning Bolt".to_owned()]),
        sideboard: Set(Vec::new()),
        commander: Set(None),
        sleeve: Set(None),
        playmat: Set(None),
        updated_at: Set(OffsetDateTime::now_utc()),
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
        account_id: Set(id),
        name: Set("Mono Green".to_owned()),
        cards: Set(vec!["4 Llanowar Elves".to_owned(), "20 Forest".to_owned()]),
        sideboard: Set(Vec::new()),
        commander: Set(None),
        sleeve: Set(None),
        playmat: Set(None),
        updated_at: Set(OffsetDateTime::now_utc()),
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

    // The decklist survives the round trip as an ordered array, which is the
    // one thing `text[]` has to do that a join table with a position column
    // would have done more elaborately.
    let saved = Deck::find().one(&sandbox.db).await.unwrap().unwrap();
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

    sandbox.close().await;
}

/// A whole store file, with something in every one of the six maps, so the
/// import's foreign keys are exercised rather than only its accounts.
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
    assert_eq!(tally.answers, 1);
    assert_eq!(tally.settings, 1);

    let saved = Account::find().one(&sandbox.db).await.unwrap().unwrap();
    assert_eq!(
        saved.email, "Keeper@Example.COM",
        "the address is kept as it was typed"
    );
    assert!(saved.confirmed_at.is_some());

    let deck = Deck::find().one(&sandbox.db).await.unwrap().unwrap();
    assert_eq!(deck.account_id, saved.id, "the deck found its owner");
    assert_eq!(deck.sideboard, ["2 Naturalize"]);

    // The reserved ability index counts down from `u32::MAX`, which is why
    // both numbers are `i64` in the table: as `i32` this row would not fit.
    let answer = StandingAnswer::find()
        .one(&sandbox.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(answer.ability, i64::from(u32::MAX));

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
