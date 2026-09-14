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
    ActiveValue::Set, ColumnTrait, ConnectionTrait, Database, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter,
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
    account::ActiveModel {
        id: Set(Uuid::now_v7()),
        email: Set(email.to_owned()),
        display_name: Set(email.split('@').next().unwrap_or(email).to_owned()),
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
        token_hash: Set("f".repeat(64)),
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
