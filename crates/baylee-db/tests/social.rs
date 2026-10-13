//! What only a real server can answer about `baylee_db::social`.
//!
//! The module is hand-written SQL whose point is that each decision is one
//! transaction: a request meeting the other's request, a group's last seat,
//! a device cap, a message and its slots. Those are claims about PostgreSQL
//! (constraints, locks, cascades, `ON CONFLICT`), so they are tested against
//! one, in a schema of its own per test, exactly as `schema.rs` does.
//!
//! Without `DATABASE_URL` they fail rather than skip.
// People are `a`, `b`, `c`: these tests read as stories about Alice, Bob and
// Carol, and each story is one test on purpose (one schema, one scenario).
#![allow(clippy::many_single_char_names, clippy::too_many_lines)]

use baylee_db::entity::account;
use baylee_db::migration::Migrator;
use baylee_db::social::{
    self, Accepted, Blocked, Counts, Device, GroupInvited, Joined, NewDevice, NewMessage, NewSlot,
    Registered, Reowned, Requested, Settings, Stored, Swept, Who,
};
use sea_orm::{
    ActiveModelTrait,
    ActiveValue::{NotSet, Set},
    ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement,
};
use sea_orm_migration::MigratorTrait as _;
use time::{Duration, OffsetDateTime};
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

        // `public` stays on the path behind the test's schema, as in
        // `schema.rs`.
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

/// One account under a name of its own, ready to insert.
fn named(email: &str, display_name: &str) -> account::ActiveModel {
    account::ActiveModel {
        id: Set(Uuid::now_v7()),
        email: Set(Some(email.to_owned())),
        username: Set(None),
        username_key: Set(None),
        display_name: Set(display_name.to_owned()),
        // The database hands this out.
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
    }
}

/// Inserts an account called `name` and answers its id.
async fn person(sandbox: &Sandbox, name: &str) -> Uuid {
    named(&format!("{}@example.org", name.to_lowercase()), name)
        .insert(&sandbox.db)
        .await
        .expect("inserting an account")
        .id
}

/// Runs a statement the module under test has no function for.
async fn sql(sandbox: &Sandbox, statement: &str) {
    sandbox
        .db
        .execute_unprepared(statement)
        .await
        .expect("running a statement");
}

/// The number of rows in `table`.
async fn rows(sandbox: &Sandbox, table: &str) -> i64 {
    sandbox
        .db
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            format!("SELECT count(*) AS n FROM {table}"),
        ))
        .await
        .expect("counting")
        .expect("a count row")
        .try_get("", "n")
        .expect("a number")
}

/// The account `id` as the social side sees it.
async fn who(sandbox: &Sandbox, id: Uuid) -> Who {
    social::who_by_ids(&sandbox.db, &[id])
        .await
        .expect("looking an account up")
        .pop()
        .expect("the account exists")
}

/// The ids of `list`.
fn ids(list: &[Who]) -> Vec<Uuid> {
    list.iter().map(|w| w.id).collect()
}

/// A device whose keys and id derive from `n`: 32-byte keys, a 64-byte
/// signature, 16 lowercase hex characters.
fn new_device(n: u8) -> NewDevice {
    NewDevice {
        id: format!("{n:016x}"),
        ik: vec![n; 32],
        dk: vec![n.wrapping_add(100); 32],
        dsig: vec![n.wrapping_add(200); 64],
        label: format!("Linux {n}"),
    }
}

/// Registers device `n` for `account` with room for `max`.
async fn register(sandbox: &Sandbox, account: Uuid, n: u8, max: usize) -> Registered {
    social::register_device(&sandbox.db, account, &new_device(n), max, false)
        .await
        .expect("registering a device")
}

/// A slot for `device`, wrapped with `n` bytes, valid for an hour.
fn slot(device: &str, n: u8) -> NewSlot {
    slot_until(device, n, OffsetDateTime::now_utc() + Duration::hours(1))
}

fn slot_until(device: &str, n: u8, expires_at: OffsetDateTime) -> NewSlot {
    NewSlot {
        device: device.to_owned(),
        wrap: vec![n; 48],
        expires_at,
    }
}

/// A message from `sender`'s device `sender_dev`.
fn message(sender: Uuid, sender_dev: &str, tag: u8) -> NewMessage {
    NewMessage {
        id: Uuid::now_v7(),
        conv: format!("conv-{tag}"),
        sender,
        sender_dev: sender_dev.to_owned(),
        sealed: format!("{{\"sealed\":{tag}}}").into_bytes(),
    }
}

/// Stores `message` and answers its slots, which must have gone in.
async fn stored(
    sandbox: &Sandbox,
    message: &NewMessage,
    slots: &[NewSlot],
    cap: usize,
) -> Vec<i64> {
    match social::store_message(&sandbox.db, message, slots, cap)
        .await
        .expect("storing a message")
    {
        Stored::Done { slots, .. } => slots.into_iter().map(|(_, seq)| seq).collect(),
        other => panic!("expected the message to be stored, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Migration and settings.
// ---------------------------------------------------------------------------

/// The migration applies a second time without complaint, and the three
/// account columns default as the protocol says. The settings round-trip.
#[tokio::test]
async fn the_migration_reapplies_and_the_settings_default_and_round_trip() {
    let sandbox = Sandbox::open("social_migrate").await;
    let a = person(&sandbox, "Alice").await;

    // A gateway restarting against its own database, twice over.
    let again = Database::connect(&sandbox.scoped)
        .await
        .expect("connecting");
    Migrator::up(&again, None).await.expect("migrating again");
    let third = baylee_db::connect(&sandbox.scoped, 1)
        .await
        .expect("connecting through the crate");
    drop(third);

    let defaults = Settings {
        whispers_from: 1,
        requests_open: true,
        share_table: true,
    };
    assert_eq!(
        social::settings_of(&sandbox.db, &[a]).await.unwrap(),
        vec![(a, defaults)]
    );
    assert!(
        social::settings_of(&sandbox.db, &[Uuid::now_v7()])
            .await
            .unwrap()
            .is_empty(),
        "an account that does not exist has no settings"
    );

    let changed = Settings {
        whispers_from: 0,
        requests_open: false,
        share_table: false,
    };
    social::put_settings(&sandbox.db, a, changed).await.unwrap();
    assert_eq!(
        social::settings_of(&sandbox.db, &[a]).await.unwrap(),
        vec![(a, changed)]
    );
    social::put_settings(&sandbox.db, a, defaults)
        .await
        .unwrap();
    assert_eq!(
        social::settings_of(&sandbox.db, &[a]).await.unwrap(),
        vec![(a, defaults)]
    );
    sandbox.close().await;
}

/// The three ways of finding an account agree with each other.
#[tokio::test]
async fn an_account_is_found_by_tag_by_tags_and_by_id() {
    let sandbox = Sandbox::open("social_who").await;
    let a = person(&sandbox, "Alice").await;
    let b = person(&sandbox, "Bob").await;
    let alice = who(&sandbox, a).await;
    let bob = who(&sandbox, b).await;
    assert_eq!(alice.name, "Alice");
    assert!(!alice.guest);
    assert_ne!(alice.tag, bob.tag, "the database mints a tag of its own");

    assert_eq!(
        social::who_by_tag(&sandbox.db, bob.tag).await.unwrap(),
        Some(bob.clone())
    );
    assert_eq!(
        social::who_by_tag(&sandbox.db, alice.tag + bob.tag + 1000)
            .await
            .unwrap(),
        None
    );
    let mut both = social::who_by_tags(&sandbox.db, &[alice.tag, bob.tag])
        .await
        .unwrap();
    both.sort_by_key(|w| w.tag);
    let mut expected = vec![alice, bob];
    expected.sort_by_key(|w| w.tag);
    assert_eq!(both, expected);
    sandbox.close().await;
}

// ---------------------------------------------------------------------------
// Devices.
// ---------------------------------------------------------------------------

/// A device is added; the same one again changes nothing; its key under
/// another account is taken; a registered account stops at its maximum.
#[tokio::test]
async fn a_device_is_added_once_and_a_registered_account_stops_at_its_maximum() {
    let sandbox = Sandbox::open("social_dev_add").await;
    let a = person(&sandbox, "Alice").await;
    let b = person(&sandbox, "Bob").await;

    assert_eq!(
        register(&sandbox, a, 1, 5).await,
        Registered::Added { replaced: vec![] }
    );
    assert_eq!(register(&sandbox, a, 1, 5).await, Registered::Again);
    assert_eq!(
        register(&sandbox, b, 1, 5).await,
        Registered::Taken,
        "another account's device has that key"
    );
    // The key is a device's identity: the same key under another id, or the
    // same id under another key, is not the same device.
    let mut other_id = new_device(1);
    other_id.id = format!("{:016x}", 99);
    assert_eq!(
        social::register_device(&sandbox.db, a, &other_id, 5, false)
            .await
            .unwrap(),
        Registered::Taken
    );
    let mut other_key = new_device(1);
    other_key.ik = vec![77; 32];
    assert_eq!(
        social::register_device(&sandbox.db, b, &other_key, 5, false)
            .await
            .unwrap(),
        Registered::Taken,
        "the id is Alice's"
    );

    for n in 2..=5 {
        assert_eq!(
            register(&sandbox, a, n, 5).await,
            Registered::Added { replaced: vec![] }
        );
    }
    assert_eq!(register(&sandbox, a, 6, 5).await, Registered::Full);
    assert!(
        social::device_by_id(&sandbox.db, &new_device(6).id)
            .await
            .unwrap()
            .is_none(),
        "a refused device is not stored"
    );
    // Bob's count is his own.
    assert_eq!(
        register(&sandbox, b, 7, 5).await,
        Registered::Added { replaced: vec![] }
    );
    assert_eq!(social::counts(&sandbox.db).await.unwrap().devices, 6);
    sandbox.close().await;
}

/// A guest (one device, `replace`) swaps its old device for the new one and
/// is told which went.
#[tokio::test]
async fn a_guests_new_device_replaces_its_old_one() {
    let sandbox = Sandbox::open("social_dev_replace").await;
    let g = person(&sandbox, "Guest").await;

    let first = new_device(1);
    let second = new_device(2);
    assert_eq!(
        social::register_device(&sandbox.db, g, &first, 1, true)
            .await
            .unwrap(),
        Registered::Added { replaced: vec![] }
    );
    assert_eq!(
        social::register_device(&sandbox.db, g, &second, 1, true)
            .await
            .unwrap(),
        Registered::Added {
            replaced: vec![first.id.clone()]
        }
    );
    let devices = social::devices_of(&sandbox.db, &[g]).await.unwrap();
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].id, second.id);
    // Without `replace` the same situation is a refusal.
    assert_eq!(
        social::register_device(&sandbox.db, g, &new_device(3), 1, false)
            .await
            .unwrap(),
        Registered::Full
    );
    sandbox.close().await;
}

/// Reading, touching and removing devices; the database's own shape checks.
#[tokio::test]
async fn devices_are_read_touched_and_removed_by_their_owner_only() {
    let sandbox = Sandbox::open("social_dev_read").await;
    let a = person(&sandbox, "Alice").await;
    let b = person(&sandbox, "Bob").await;
    for n in [1, 2, 3] {
        register(&sandbox, a, n, 5).await;
    }
    register(&sandbox, b, 4, 5).await;

    let one: Device = social::device_by_id(&sandbox.db, &new_device(1).id)
        .await
        .unwrap()
        .expect("device 1");
    let want = new_device(1);
    assert_eq!(one.account_id, a);
    assert_eq!(
        (one.ik, one.dk, one.dsig, one.label),
        (want.ik, want.dk, want.dsig, want.label)
    );
    assert!(
        social::device_by_id(&sandbox.db, "ffffffffffffffff")
            .await
            .unwrap()
            .is_none()
    );

    let listed = social::devices_of(&sandbox.db, &[a, b]).await.unwrap();
    assert_eq!(
        listed.iter().map(|d| d.id.clone()).collect::<Vec<_>>(),
        [1, 2, 3, 4].map(|n| new_device(n).id),
        "oldest first"
    );
    assert!(
        social::devices_of(&sandbox.db, &[])
            .await
            .unwrap()
            .is_empty()
    );

    // The device is seen later than it was added.
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    social::touch_device(&sandbox.db, &new_device(2).id)
        .await
        .unwrap();
    let touched = social::device_by_id(&sandbox.db, &new_device(2).id)
        .await
        .unwrap()
        .unwrap();
    assert!(touched.seen_at > touched.added_at);
    let untouched = social::device_by_id(&sandbox.db, &new_device(3).id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(untouched.seen_at, untouched.added_at);

    assert!(
        !social::remove_device(&sandbox.db, b, &new_device(1).id)
            .await
            .unwrap(),
        "not Bob's device"
    );
    assert!(
        social::device_by_id(&sandbox.db, &new_device(1).id)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        social::remove_device(&sandbox.db, a, &new_device(1).id)
            .await
            .unwrap()
    );
    assert!(
        !social::remove_device(&sandbox.db, a, &new_device(1).id)
            .await
            .unwrap()
    );

    // The shape checks the migration declares.
    let mut bad_key = new_device(8);
    bad_key.ik = vec![8; 31];
    assert!(
        social::register_device(&sandbox.db, a, &bad_key, 5, false)
            .await
            .is_err()
    );
    let mut bad_label = new_device(9);
    bad_label.label = "x".repeat(25);
    assert!(
        social::register_device(&sandbox.db, a, &bad_label, 5, false)
            .await
            .is_err()
    );
    bad_label.label = String::new();
    assert!(
        social::register_device(&sandbox.db, a, &bad_label, 5, false)
            .await
            .is_err()
    );
    let mut bad_id = new_device(10);
    bad_id.id = "ABCDEF0123456789".to_owned();
    assert!(
        social::register_device(&sandbox.db, a, &bad_id, 5, false)
            .await
            .is_err(),
        "ids are lowercase hex"
    );
    let mut longest = new_device(11);
    longest.label = "y".repeat(24);
    assert_eq!(
        social::register_device(&sandbox.db, a, &longest, 5, false)
            .await
            .unwrap(),
        Registered::Added { replaced: vec![] }
    );
    sandbox.close().await;
}

// ---------------------------------------------------------------------------
// Friends.
// ---------------------------------------------------------------------------

/// A request is pending until the other asks back; then both are friends and
/// no request is left.
#[tokio::test]
async fn a_request_met_by_a_request_makes_friends() {
    let sandbox = Sandbox::open("social_friends").await;
    let a = person(&sandbox, "Alice").await;
    let b = person(&sandbox, "Bob").await;
    let db = &sandbox.db;

    assert_eq!(
        social::request_friend(db, a, b, 10, 10).await.unwrap(),
        Requested::Pending
    );
    // Asking twice is the same request.
    assert_eq!(
        social::request_friend(db, a, b, 10, 10).await.unwrap(),
        Requested::Pending
    );
    assert_eq!(ids(&social::incoming(db, b).await.unwrap()), vec![a]);
    assert_eq!(ids(&social::outgoing(db, a).await.unwrap()), vec![b]);
    assert!(social::incoming(db, a).await.unwrap().is_empty());
    assert!(social::outgoing(db, b).await.unwrap().is_empty());
    let from_b = social::relation(db, b, a).await.unwrap();
    assert!(from_b.incoming && !from_b.outgoing && !from_b.friend);
    let from_a = social::relation(db, a, b).await.unwrap();
    assert!(from_a.outgoing && !from_a.incoming && !from_a.friend);

    assert_eq!(
        social::request_friend(db, b, a, 10, 10).await.unwrap(),
        Requested::Friends
    );
    assert!(social::incoming(db, a).await.unwrap().is_empty());
    assert!(social::incoming(db, b).await.unwrap().is_empty());
    assert!(social::outgoing(db, a).await.unwrap().is_empty());
    assert!(social::outgoing(db, b).await.unwrap().is_empty());
    assert_eq!(rows(&sandbox, "friend_request").await, 0);
    assert_eq!(rows(&sandbox, "friendship").await, 1);

    let of_a = social::friends(db, a).await.unwrap();
    assert_eq!(
        ids(&of_a.iter().map(|(w, _)| w.clone()).collect::<Vec<_>>()),
        vec![b]
    );
    assert_eq!(of_a[0].0.name, "Bob");
    assert!(of_a[0].1 <= OffsetDateTime::now_utc());
    let of_b = social::friends(db, b).await.unwrap();
    assert_eq!(of_b[0].0.id, a);
    assert_eq!(social::friend_ids(db, a).await.unwrap(), vec![b]);
    assert_eq!(social::friend_ids(db, b).await.unwrap(), vec![a]);
    let r = social::relation(db, a, b).await.unwrap();
    assert!(r.friend && !r.incoming && !r.outgoing && !r.blocked && !r.blocks_me);

    for (x, y) in [(a, b), (b, a)] {
        assert_eq!(
            social::request_friend(db, x, y, 10, 10).await.unwrap(),
            Requested::AlreadyFriends
        );
    }
    assert!(social::unfriend(db, b, a).await.unwrap());
    assert!(!social::unfriend(db, b, a).await.unwrap());
    assert!(social::friends(db, a).await.unwrap().is_empty());
    assert!(social::friend_ids(db, b).await.unwrap().is_empty());
    sandbox.close().await;
}

/// Accepting, declining and cancelling; and the places a request is refused.
#[tokio::test]
async fn requests_are_accepted_dropped_and_refused() {
    let sandbox = Sandbox::open("social_requests").await;
    let a = person(&sandbox, "Alice").await;
    let b = person(&sandbox, "Bob").await;
    let c = person(&sandbox, "Carol").await;
    let d = person(&sandbox, "Dave").await;
    let db = &sandbox.db;

    // Accept.
    assert_eq!(accept(&sandbox, b, a).await, Accepted::NoRequest);
    social::request_friend(db, a, b, 10, 10).await.unwrap();
    assert_eq!(
        accept(&sandbox, a, b).await,
        Accepted::NoRequest,
        "only the one asked can accept"
    );
    assert_eq!(accept(&sandbox, b, a).await, Accepted::Friends);
    assert_eq!(accept(&sandbox, b, a).await, Accepted::NoRequest);
    assert_eq!(social::friend_ids(db, a).await.unwrap(), vec![b]);
    assert_eq!(rows(&sandbox, "friend_request").await, 0);

    // Decline (by the one asked) and cancel (by the asker) are one call.
    social::request_friend(db, c, a, 10, 10).await.unwrap();
    assert!(social::drop_request(db, c, a).await.unwrap());
    assert!(!social::drop_request(db, c, a).await.unwrap());
    assert_eq!(accept(&sandbox, a, c).await, Accepted::NoRequest);
    social::request_friend(db, a, c, 10, 10).await.unwrap();
    assert!(
        !social::drop_request(db, c, a).await.unwrap(),
        "the request goes the other way"
    );
    assert!(social::drop_request(db, a, c).await.unwrap());

    // A closed door.
    social::put_settings(
        db,
        d,
        Settings {
            whispers_from: 1,
            requests_open: false,
            share_table: true,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        social::request_friend(db, a, d, 10, 10).await.unwrap(),
        Requested::Closed
    );
    assert!(social::outgoing(db, a).await.unwrap().is_empty());
    // ... which does not stop the one behind it from asking out.
    assert_eq!(
        social::request_friend(db, d, a, 10, 10).await.unwrap(),
        Requested::Pending
    );

    // Too many requests out.
    let e = person(&sandbox, "Eve").await;
    let f = person(&sandbox, "Frank").await;
    assert_eq!(
        social::request_friend(db, e, a, 1, 10).await.unwrap(),
        Requested::Pending
    );
    assert_eq!(
        social::request_friend(db, e, f, 1, 10).await.unwrap(),
        Requested::TooMany
    );
    assert_eq!(
        social::request_friend(db, e, a, 1, 10).await.unwrap(),
        Requested::Pending,
        "the request already out is not a new one"
    );
    assert_eq!(ids(&social::outgoing(db, e).await.unwrap()), vec![a]);
    sandbox.close().await;
}

async fn accept(sandbox: &Sandbox, me: Uuid, from: Uuid) -> Accepted {
    social::accept(&sandbox.db, me, from, 10).await.unwrap()
}

/// A friends cap on either side stops a friendship both ways it can start.
#[tokio::test]
async fn the_friend_cap_stops_a_friendship_on_either_side() {
    let sandbox = Sandbox::open("social_friend_cap").await;
    let a = person(&sandbox, "Alice").await;
    let b = person(&sandbox, "Bob").await;
    let c = person(&sandbox, "Carol").await;
    let db = &sandbox.db;

    social::request_friend(db, a, b, 10, 1).await.unwrap();
    assert_eq!(accept(&sandbox, b, a).await, Accepted::Friends);

    // Carol asks Alice, who has her one friend: accepting is refused ...
    social::request_friend(db, c, a, 10, 1).await.unwrap();
    assert_eq!(
        social::accept(db, a, c, 1).await.unwrap(),
        Accepted::TooMany
    );
    assert_eq!(social::friend_ids(db, c).await.unwrap(), Vec::<Uuid>::new());
    assert_eq!(
        ids(&social::incoming(db, a).await.unwrap()),
        vec![c],
        "the request stays"
    );
    // ... and so is asking back, which would meet it.
    assert_eq!(
        social::request_friend(db, a, c, 10, 1).await.unwrap(),
        Requested::TooMany
    );
    // Carol's side counts too: Bob is Alice's only friend, Dave asks Carol
    // after she has one.
    social::unfriend(db, a, b).await.unwrap();
    assert_eq!(accept(&sandbox, a, c).await, Accepted::Friends);
    let d = person(&sandbox, "Dave").await;
    social::request_friend(db, a, d, 10, 1).await.unwrap();
    assert_eq!(
        social::accept(db, d, a, 1).await.unwrap(),
        Accepted::TooMany,
        "Alice is at her cap, Dave is not"
    );
    sandbox.close().await;
}

// ---------------------------------------------------------------------------
// Blocks.
// ---------------------------------------------------------------------------

/// Blocking ends a friendship and both requests; the blocked one's later
/// request looks sent and is never shown to the blocker.
#[tokio::test]
async fn a_block_ends_what_there_was_and_hides_what_follows() {
    let sandbox = Sandbox::open("social_block").await;
    let a = person(&sandbox, "Alice").await;
    let b = person(&sandbox, "Bob").await;
    let c = person(&sandbox, "Carol").await;
    let db = &sandbox.db;

    social::request_friend(db, a, b, 10, 10).await.unwrap();
    social::request_friend(db, b, a, 10, 10).await.unwrap();
    // Both ways at once, beside the friendship: nothing in the API makes
    // this state, and the block clears it all the same.
    sql(
        &sandbox,
        &format!(
            "INSERT INTO friend_request (from_id, to_id) VALUES ('{a}', '{b}'), ('{b}', '{a}')"
        ),
    )
    .await;
    assert_eq!(
        social::block(db, a, b).await.unwrap(),
        Blocked {
            was_friend: true,
            had_outgoing: true,
            had_incoming: true
        }
    );
    assert_eq!(rows(&sandbox, "friendship").await, 0);
    assert_eq!(rows(&sandbox, "friend_request").await, 0);
    assert_eq!(ids(&social::blocked(db, a).await.unwrap()), vec![b]);
    assert!(social::blocked(db, b).await.unwrap().is_empty());
    let r = social::relation(db, a, b).await.unwrap();
    assert!(r.blocked && !r.blocks_me && !r.friend);
    let r = social::relation(db, b, a).await.unwrap();
    assert!(r.blocks_me && !r.blocked);
    // Blocking twice is not an error and ends nothing more.
    assert_eq!(social::block(db, a, b).await.unwrap(), Blocked::default());

    // Bob asks Alice: it looks sent ...
    assert_eq!(
        social::request_friend(db, b, a, 10, 10).await.unwrap(),
        Requested::Pending
    );
    assert_eq!(ids(&social::outgoing(db, b).await.unwrap()), vec![a]);
    assert!(social::relation(db, b, a).await.unwrap().outgoing);
    // ... and is never shown to her, nor can she accept it.
    assert!(social::incoming(db, a).await.unwrap().is_empty());
    assert!(!social::relation(db, a, b).await.unwrap().incoming);
    assert_eq!(accept(&sandbox, a, b).await, Accepted::NoRequest);
    assert_eq!(rows(&sandbox, "friendship").await, 0);
    // Alice asking Bob back does not meet the hidden request either.
    assert_eq!(
        social::request_friend(db, a, b, 10, 10).await.unwrap(),
        Requested::Pending
    );
    assert_eq!(rows(&sandbox, "friendship").await, 0);

    // Who blocks whom, among others.
    social::block(db, c, a).await.unwrap();
    assert_eq!(social::blockers_of(db, b, &[a, c]).await.unwrap(), vec![a]);
    let mut both = social::blockers_of(db, a, &[b, c]).await.unwrap();
    both.sort();
    assert_eq!(both, vec![c], "Bob does not block Alice");
    assert!(social::blockers_of(db, b, &[]).await.unwrap().is_empty());

    assert!(social::unblock(db, a, b).await.unwrap());
    assert!(!social::unblock(db, a, b).await.unwrap());
    assert!(
        social::blockers_of(db, b, &[a, c])
            .await
            .unwrap()
            .is_empty()
    );
    assert!(!social::relation(db, a, b).await.unwrap().blocked);
    // The request Bob sent while blocked never arrives, not even late.
    assert!(!social::relation(db, a, b).await.unwrap().incoming);
    assert!(social::outgoing(db, b).await.unwrap().is_empty());
    sandbox.close().await;
}

// ---------------------------------------------------------------------------
// Groups.
// ---------------------------------------------------------------------------

/// Making a group, inviting to it, taking an invite up or dropping it.
#[tokio::test]
async fn a_group_takes_invites_up_to_its_size_and_members_up_to_theirs() {
    let sandbox = Sandbox::open("social_group").await;
    let a = person(&sandbox, "Alice").await;
    let b = person(&sandbox, "Bob").await;
    let c = person(&sandbox, "Carol").await;
    let d = person(&sandbox, "Dave").await;
    let db = &sandbox.db;

    let g = social::create_group(db, a, "Friday", 2)
        .await
        .unwrap()
        .expect("room for a group");
    let group = social::group(db, g).await.unwrap().unwrap();
    assert_eq!(group.name, "Friday");
    assert_eq!(group.role(a), Some(true));
    assert_eq!(group.role(b), None);
    assert_eq!(group.member_ids(), vec![a]);
    assert_eq!(social::group_ids_of(db, a).await.unwrap(), vec![g]);
    assert!(social::group(db, Uuid::now_v7()).await.unwrap().is_none());

    // Members and invites together fill the three seats.
    assert_eq!(invite(&sandbox, g, a, a, 3).await, GroupInvited::Member);
    assert_eq!(invite(&sandbox, g, b, a, 3).await, GroupInvited::Done);
    assert_eq!(invite(&sandbox, g, b, a, 3).await, GroupInvited::Again);
    assert_eq!(invite(&sandbox, g, c, a, 3).await, GroupInvited::Done);
    assert_eq!(invite(&sandbox, g, d, a, 3).await, GroupInvited::Full);
    let group = social::group(db, g).await.unwrap().unwrap();
    assert_eq!(ids(&group.invited), vec![b, c]);
    let invites = social::group_invites_of(db, b).await.unwrap();
    assert_eq!(invites.len(), 1);
    assert_eq!(invites[0].group, g);
    assert_eq!(invites[0].name, "Friday");
    assert_eq!(invites[0].inviter.id, a);
    assert!(social::group_invites_of(db, d).await.unwrap().is_empty());

    // Taking up an invite makes a member and uses the invite up.
    assert_eq!(join(&sandbox, g, d, 2).await, Joined::NoInvite);
    assert_eq!(join(&sandbox, g, b, 2).await, Joined::Joined);
    assert_eq!(join(&sandbox, g, b, 2).await, Joined::NoInvite);
    assert!(social::group_invites_of(db, b).await.unwrap().is_empty());
    assert_eq!(social::group_ids_of(db, b).await.unwrap(), vec![g]);
    // A member takes a seat as an invite did: still full.
    assert_eq!(invite(&sandbox, g, d, a, 3).await, GroupInvited::Full);

    // Dropping an invite answers who sent it.
    assert_eq!(social::drop_group_invite(db, g, c).await.unwrap(), Some(a));
    assert_eq!(social::drop_group_invite(db, g, c).await.unwrap(), None);
    assert_eq!(invite(&sandbox, g, d, b, 3).await, GroupInvited::Done);
    assert_eq!(
        social::group_invites_of(db, d).await.unwrap()[0].inviter.id,
        b
    );

    // Everyone is in at most two groups: Bob's second is his own, then
    // Alice's second invitation to him is one too many.
    let own = social::create_group(db, b, "Mine", 2)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        social::create_group(db, b, "Third", 2).await.unwrap(),
        None,
        "two groups already"
    );
    assert_eq!(rows(&sandbox, "chat_group").await, 2);
    let other = social::create_group(db, a, "Saturday", 2)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(invite(&sandbox, other, b, a, 10).await, GroupInvited::Done);
    assert_eq!(join(&sandbox, other, b, 2).await, Joined::TooMany);
    assert_eq!(
        social::group_invites_of(db, b).await.unwrap()[0].group,
        other,
        "the invite is still there"
    );
    assert!(
        !social::group(db, other)
            .await
            .unwrap()
            .unwrap()
            .member_ids()
            .contains(&b)
    );
    // Making room makes it join.
    assert!(social::remove_member(db, own, b).await.unwrap());
    assert_eq!(join(&sandbox, other, b, 2).await, Joined::Joined);

    // Co-members: those who share any group, once, never oneself.
    assert_eq!(social::co_members(db, a).await.unwrap(), vec![b]);
    assert!(social::co_members(db, c).await.unwrap().is_empty());
    assert_eq!(social::co_members(db, b).await.unwrap(), vec![a]);

    // A group list, in creation order, with members and invites filled in.
    let listed = social::groups(db, &[other, g]).await.unwrap();
    assert_eq!(
        listed.iter().map(|x| x.id).collect::<Vec<_>>(),
        vec![g, other]
    );
    assert_eq!(listed[0].member_ids(), vec![a, b]);
    assert_eq!(ids(&listed[0].invited), vec![d]);
    sandbox.close().await;
}

async fn invite(
    sandbox: &Sandbox,
    group: Uuid,
    invitee: Uuid,
    inviter: Uuid,
    max: usize,
) -> GroupInvited {
    social::invite_to_group(&sandbox.db, group, invitee, inviter, max)
        .await
        .unwrap()
}

async fn join(sandbox: &Sandbox, group: Uuid, me: Uuid, max: usize) -> Joined {
    social::join_group(&sandbox.db, group, me, max)
        .await
        .unwrap()
}

/// Handing a group over, renaming it, and what happens to it as members go.
#[tokio::test]
async fn a_group_changes_hands_and_goes_with_its_last_member() {
    let sandbox = Sandbox::open("social_group_owner").await;
    let a = person(&sandbox, "Alice").await;
    let b = person(&sandbox, "Bob").await;
    let c = person(&sandbox, "Carol").await;
    let d = person(&sandbox, "Dave").await;
    let db = &sandbox.db;

    let g = social::create_group(db, a, "Club", 5)
        .await
        .unwrap()
        .unwrap();
    // Members join in a known order: Alice (owner), Bob, Carol.
    for who in [b, c] {
        assert_eq!(invite(&sandbox, g, who, a, 10).await, GroupInvited::Done);
        assert_eq!(join(&sandbox, g, who, 5).await, Joined::Joined);
    }
    let roles = |group: &social::Group| {
        group
            .members
            .iter()
            .map(|(w, o)| (w.id, *o))
            .collect::<Vec<_>>()
    };
    let group = social::group(db, g).await.unwrap().unwrap();
    assert_eq!(roles(&group), vec![(a, true), (b, false), (c, false)]);

    social::rename_group(db, g, "Club 41").await.unwrap();
    assert_eq!(social::group(db, g).await.unwrap().unwrap().name, "Club 41");

    // Refusals leave the owner where she was.
    assert!(
        !social::hand_over(db, g, a, d).await.unwrap(),
        "Dave is no member"
    );
    assert!(
        !social::hand_over(db, g, b, c).await.unwrap(),
        "Bob owns nothing"
    );
    assert!(!social::hand_over(db, g, a, a).await.unwrap(), "to herself");
    assert!(!social::hand_over(db, Uuid::now_v7(), a, b).await.unwrap());
    assert_eq!(
        roles(&social::group(db, g).await.unwrap().unwrap()),
        vec![(a, true), (b, false), (c, false)]
    );
    // A real one.
    assert!(social::hand_over(db, g, a, c).await.unwrap());
    assert_eq!(
        roles(&social::group(db, g).await.unwrap().unwrap()),
        vec![(a, false), (b, false), (c, true)]
    );

    // A member who is not the owner leaves: nothing else moves.
    assert!(social::remove_member(db, g, a).await.unwrap());
    assert!(!social::remove_member(db, g, a).await.unwrap());
    assert_eq!(
        roles(&social::group(db, g).await.unwrap().unwrap()),
        vec![(b, false), (c, true)]
    );
    assert!(social::group_ids_of(db, a).await.unwrap().is_empty());

    // The owner leaves: the group passes to the member who joined first.
    invite(&sandbox, g, d, c, 10).await;
    join(&sandbox, g, d, 5).await;
    assert!(social::remove_member(db, g, c).await.unwrap());
    assert_eq!(
        roles(&social::group(db, g).await.unwrap().unwrap()),
        vec![(b, true), (d, false)]
    );

    // The last one out turns the light off.
    assert!(social::remove_member(db, g, b).await.unwrap());
    assert!(social::remove_member(db, g, d).await.unwrap());
    assert!(social::group(db, g).await.unwrap().is_none());
    assert_eq!(rows(&sandbox, "chat_group").await, 0);

    // `delete_group` takes members and invites with it.
    let h = social::create_group(db, a, "Gone", 5)
        .await
        .unwrap()
        .unwrap();
    invite(&sandbox, h, b, a, 10).await;
    assert!(social::delete_group(db, h).await.unwrap());
    assert!(!social::delete_group(db, h).await.unwrap());
    assert_eq!(rows(&sandbox, "group_member").await, 0);
    assert_eq!(rows(&sandbox, "group_invite").await, 0);
    sandbox.close().await;
}

/// An account that is deleted takes its seat out of a group; `reown` hands
/// the group on or deletes it.
#[tokio::test]
async fn reown_hands_on_or_deletes_after_an_account_is_deleted() {
    let sandbox = Sandbox::open("social_reown").await;
    let a = person(&sandbox, "Alice").await;
    let b = person(&sandbox, "Bob").await;
    let c = person(&sandbox, "Carol").await;
    let d = person(&sandbox, "Dave").await;
    let db = &sandbox.db;

    // Alice owns a group with Bob and Dave (in that order); Carol is alone.
    let shared = social::create_group(db, a, "Shared", 5)
        .await
        .unwrap()
        .unwrap();
    for who in [b, d] {
        invite(&sandbox, shared, who, a, 10).await;
        join(&sandbox, shared, who, 5).await;
    }
    let alone = social::create_group(db, c, "Alone", 5)
        .await
        .unwrap()
        .unwrap();
    let kept = social::create_group(db, b, "Kept", 5)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(social::reown(db).await.unwrap(), Reowned::default());
    sql(&sandbox, &format!("DELETE FROM account WHERE id = '{a}'")).await;
    sql(&sandbox, &format!("DELETE FROM account WHERE id = '{c}'")).await;
    assert_eq!(
        social::reown(db).await.unwrap(),
        Reowned {
            handed_on: 1,
            deleted: 1
        }
    );
    let shared = social::group(db, shared).await.unwrap().unwrap();
    assert_eq!(
        shared
            .members
            .iter()
            .map(|(w, o)| (w.id, *o))
            .collect::<Vec<_>>(),
        vec![(b, true), (d, false)],
        "Bob joined before Dave"
    );
    assert!(social::group(db, alone).await.unwrap().is_none());
    assert_eq!(
        social::group(db, kept).await.unwrap().unwrap().role(b),
        Some(true)
    );
    // Nothing is left to do the second time.
    assert_eq!(social::reown(db).await.unwrap(), Reowned::default());
    sandbox.close().await;
}

// ---------------------------------------------------------------------------
// Messages.
// ---------------------------------------------------------------------------

/// Two devices of Bob, one of Alice, ready to send and receive.
struct Chat {
    sandbox: Sandbox,
    a: Uuid,
    b: Uuid,
    a1: String,
    b1: String,
    b2: String,
}

async fn chat(what: &str) -> Chat {
    let sandbox = Sandbox::open(what).await;
    let a = person(&sandbox, "Alice").await;
    let b = person(&sandbox, "Bob").await;
    register(&sandbox, a, 1, 5).await;
    register(&sandbox, b, 2, 5).await;
    register(&sandbox, b, 3, 5).await;
    Chat {
        a,
        b,
        a1: new_device(1).id,
        b1: new_device(2).id,
        b2: new_device(3).id,
        sandbox,
    }
}

/// A message with two slots, stored again, and stored past a device's cap.
#[tokio::test]
async fn a_message_is_stored_with_its_slots_once_and_not_past_the_cap() {
    let Chat {
        sandbox,
        a,
        a1,
        b1,
        b2,
        ..
    } = chat("social_store").await;
    let db = &sandbox.db;

    let m = message(a, &a1, 1);
    let Stored::Done { at, slots } =
        social::store_message(db, &m, &[slot(&b1, 1), slot(&b2, 2)], 10)
            .await
            .unwrap()
    else {
        panic!("expected the message to be stored");
    };
    assert!(at <= OffsetDateTime::now_utc() + Duration::seconds(1));
    assert_eq!(slots.len(), 2);
    assert_eq!(
        (slots[0].0.as_str(), slots[1].0.as_str()),
        (b1.as_str(), b2.as_str())
    );
    assert!(slots[0].1 < slots[1].1, "slot numbers rise");

    // The same id again stores nothing.
    assert_eq!(
        social::store_message(db, &m, &[slot(&b1, 1), slot(&b2, 2)], 10)
            .await
            .unwrap(),
        Stored::Duplicate
    );
    assert_eq!(rows(&sandbox, "chat_message").await, 1);
    assert_eq!(rows(&sandbox, "chat_delivery").await, 2);

    // The next message's slots rise further.
    let next = stored(&sandbox, &message(a, &a1, 2), &[slot(&b1, 3)], 10).await;
    assert!(next[0] > slots[1].1);

    // Cap: device a1 holds `cap` slots, the next is refused, and the
    // message does not stay behind without them.
    let cap = 3;
    for n in 0..cap {
        stored(
            &sandbox,
            &message(a, &a1, 10 + u8::try_from(n).unwrap()),
            &[slot(&a1, 9)],
            cap,
        )
        .await;
    }
    let refused = message(a, &a1, 99);
    assert_eq!(
        social::store_message(db, &refused, &[slot(&b1, 1), slot(&a1, 9)], cap)
            .await
            .unwrap(),
        Stored::Full,
        "one full device among the recipients refuses the whole message"
    );
    assert_eq!(
        rows(&sandbox, "chat_message").await,
        2 + i64::try_from(cap).unwrap(),
        "the refused message is not stored"
    );
    // Not even the slot for the device that had room.
    assert_eq!(social::pending(db, &b1, &[], 100).await.unwrap().len(), 2);
    let stored_again = social::store_message(db, &refused, &[slot(&b2, 1)], cap)
        .await
        .unwrap();
    assert!(
        matches!(stored_again, Stored::Done { .. }),
        "its id was never taken"
    );
    sandbox.close().await;
}

/// What a device is handed, in order, and what an ack takes away.
#[tokio::test]
async fn pending_is_oldest_first_and_ack_takes_only_the_devices_own_slots() {
    let Chat {
        sandbox,
        a,
        a1,
        b1,
        b2,
        ..
    } = chat("social_pending").await;
    let db = &sandbox.db;

    let mut first_for_b1 = Vec::new();
    let mut for_b2 = Vec::new();
    for n in 1..=4u8 {
        let both = n == 2;
        let slots = if both {
            vec![slot(&b1, n), slot(&b2, n + 50)]
        } else {
            vec![slot(&b1, n)]
        };
        let seqs = stored(&sandbox, &message(a, &a1, n), &slots, 10).await;
        first_for_b1.push(seqs[0]);
        if both {
            for_b2.push(seqs[1]);
        }
    }

    let alice = who(&sandbox, a).await;
    let all = social::pending(db, &b1, &[], 10).await.unwrap();
    assert_eq!(
        all.iter().map(|p| p.seq).collect::<Vec<_>>(),
        first_for_b1,
        "oldest first"
    );
    assert!(first_for_b1.windows(2).all(|w| w[0] < w[1]));
    let second = &all[1];
    assert_eq!(second.conv, "conv-2");
    assert_eq!(second.sender, alice);
    assert_eq!(second.sender_dev, a1);
    assert_eq!(second.sealed, b"{\"sealed\":2}".to_vec());
    assert_eq!(second.wrap, vec![2; 48]);
    assert!(second.at <= OffsetDateTime::now_utc() + Duration::seconds(1));
    assert_eq!(all[0].wrap, vec![1; 48]);

    // The same message on Bob's other device wraps its key for that device.
    let other = social::pending(db, &b2, &[], 10).await.unwrap();
    assert_eq!(other.len(), 1);
    assert_eq!(other[0].seq, for_b2[0]);
    assert_eq!(other[0].wrap, vec![52; 48]);
    assert_eq!(other[0].sealed, second.sealed);
    assert!(social::pending(db, &a1, &[], 10).await.unwrap().is_empty());

    // Skip and limit.
    let skipped = social::pending(db, &b1, &[first_for_b1[0], first_for_b1[2]], 10)
        .await
        .unwrap();
    assert_eq!(
        skipped.iter().map(|p| p.seq).collect::<Vec<_>>(),
        vec![first_for_b1[1], first_for_b1[3]]
    );
    let limited = social::pending(db, &b1, &[], 2).await.unwrap();
    assert_eq!(
        limited.iter().map(|p| p.seq).collect::<Vec<_>>(),
        first_for_b1[..2].to_vec()
    );
    let both = social::pending(db, &b1, &[first_for_b1[0]], 2)
        .await
        .unwrap();
    assert_eq!(
        both.iter().map(|p| p.seq).collect::<Vec<_>>(),
        first_for_b1[1..3].to_vec(),
        "skipped ones do not use up the limit"
    );
    assert!(social::pending(db, &b1, &[], 0).await.unwrap().is_empty());

    // Ack: only slots of that device go.
    assert_eq!(social::ack(db, &b1, &[]).await.unwrap(), 0);
    assert_eq!(
        social::ack(db, &b1, &[first_for_b1[0], for_b2[0], 999_999])
            .await
            .unwrap(),
        1,
        "the other device's slot and a made-up one are not touched"
    );
    assert_eq!(social::pending(db, &b2, &[], 10).await.unwrap().len(), 1);
    assert_eq!(social::pending(db, &b1, &[], 10).await.unwrap().len(), 3);
    assert_eq!(social::ack(db, &b1, &[first_for_b1[0]]).await.unwrap(), 0);
    assert_eq!(social::ack(db, &b2, &for_b2).await.unwrap(), 1);
    // An acked message's row stays until the sweep.
    assert_eq!(rows(&sandbox, "chat_message").await, 4);
    sandbox.close().await;
}

/// The sweep removes the slots at or past their expiry, then the messages
/// nobody holds a slot for, and nothing live.
#[tokio::test]
async fn the_sweep_removes_expired_slots_and_then_orphaned_messages() {
    let Chat {
        sandbox,
        a,
        a1,
        b1,
        b2,
        ..
    } = chat("social_sweep").await;
    let db = &sandbox.db;
    // Whole seconds, so that the boundary is the same number on both sides.
    let now = OffsetDateTime::now_utc().replace_nanosecond(0).unwrap();
    let hour = Duration::hours(1);

    // 1: its one slot expires exactly now. 2: lives on. 3: one slot dead, one
    // alive. 4: both dead.
    let m1 = message(a, &a1, 1);
    let m2 = message(a, &a1, 2);
    let m3 = message(a, &a1, 3);
    let m4 = message(a, &a1, 4);
    stored(&sandbox, &m1, &[slot_until(&b1, 1, now)], 10).await;
    stored(&sandbox, &m2, &[slot_until(&b1, 2, now + hour)], 10).await;
    stored(
        &sandbox,
        &m3,
        &[
            slot_until(&b1, 3, now - hour),
            slot_until(&b2, 3, now + hour),
        ],
        10,
    )
    .await;
    stored(
        &sandbox,
        &m4,
        &[
            slot_until(&b1, 4, now - hour),
            slot_until(&b2, 4, now - hour),
        ],
        10,
    )
    .await;
    assert_eq!(social::counts(db).await.unwrap().pending_slots, 6);

    // Nothing is due a minute before the earliest expiry.
    assert_eq!(
        social::sweep(db, now - hour - Duration::minutes(1))
            .await
            .unwrap(),
        Swept::default()
    );
    assert_eq!(
        social::sweep(db, now).await.unwrap(),
        Swept {
            slots: 4,
            messages: 2
        }
    );
    assert_eq!(rows(&sandbox, "chat_message").await, 2);
    let ids_left: Vec<String> = social::pending(db, &b1, &[], 10)
        .await
        .unwrap()
        .into_iter()
        .map(|p| p.conv)
        .collect();
    assert_eq!(ids_left, vec!["conv-2".to_owned()]);
    let on_b2 = social::pending(db, &b2, &[], 10).await.unwrap();
    assert_eq!(on_b2.len(), 1);
    assert_eq!(
        on_b2[0].conv, "conv-3",
        "the live slot of a half-dead message"
    );
    assert_eq!(social::counts(db).await.unwrap().pending_slots, 2);

    // Later, the rest.
    assert_eq!(
        social::sweep(db, now + hour).await.unwrap(),
        Swept {
            slots: 2,
            messages: 2
        }
    );
    assert_eq!(rows(&sandbox, "chat_message").await, 0);
    assert_eq!(
        social::sweep(db, now + hour).await.unwrap(),
        Swept::default()
    );
    sandbox.close().await;
}

/// A slot past its expiry is not handed out even before the sweep gets to
/// it, and a wrap of the wrong length is refused by the database.
#[tokio::test]
async fn an_expired_slot_is_not_pending_and_a_wrap_has_one_length() {
    let Chat {
        sandbox, a, a1, b1, ..
    } = chat("social_expired").await;
    let db = &sandbox.db;
    let now = OffsetDateTime::now_utc();

    stored(
        &sandbox,
        &message(a, &a1, 1),
        &[slot_until(&b1, 1, now - Duration::seconds(5))],
        10,
    )
    .await;
    stored(&sandbox, &message(a, &a1, 2), &[slot(&b1, 2)], 10).await;
    let pending = social::pending(db, &b1, &[], 10).await.unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].conv, "conv-2");

    for len in [0, 47, 49] {
        let wrong = NewSlot {
            device: b1.clone(),
            wrap: vec![0; len],
            expires_at: now + Duration::hours(1),
        };
        let m = message(a, &a1, 7);
        assert!(
            social::store_message(db, &m, &[wrong], 10).await.is_err(),
            "{len} bytes"
        );
        assert_eq!(
            rows(&sandbox, "chat_message").await,
            2,
            "the message is not stored without its slot"
        );
    }
    sandbox.close().await;
}

// ---------------------------------------------------------------------------
// Cascade and counts.
// ---------------------------------------------------------------------------

/// Deleting an account leaves nothing about its chat behind.
#[tokio::test]
async fn deleting_an_account_leaves_nothing_of_its_chat_behind() {
    let Chat {
        sandbox,
        a,
        b,
        a1,
        b1,
        ..
    } = chat("social_cascade").await;
    let db = &sandbox.db;
    let c = person(&sandbox, "Carol").await;
    let d = person(&sandbox, "Dave").await;
    let e = person(&sandbox, "Eve").await;

    // Alice: a friend (Bob), a request out (to Carol), one in (from Dave), a
    // block on Eve, a block by Eve on her, two groups (one shared with
    // Bob), an invite to Carol and one from Bob to her.
    social::request_friend(db, a, b, 10, 10).await.unwrap();
    social::accept(db, b, a, 10).await.unwrap();
    social::request_friend(db, a, c, 10, 10).await.unwrap();
    social::request_friend(db, d, a, 10, 10).await.unwrap();
    social::block(db, a, e).await.unwrap();
    social::block(db, e, a).await.unwrap();
    let shared = social::create_group(db, a, "Shared", 5)
        .await
        .unwrap()
        .unwrap();
    invite(&sandbox, shared, b, a, 10).await;
    join(&sandbox, shared, b, 5).await;
    invite(&sandbox, shared, c, a, 10).await;
    let lonely = social::create_group(db, a, "Lonely", 5)
        .await
        .unwrap()
        .unwrap();
    let theirs = social::create_group(db, b, "Theirs", 5)
        .await
        .unwrap()
        .unwrap();
    invite(&sandbox, theirs, a, b, 10).await;
    // Chat: Alice to Bob's device (slot), Bob to Alice's device (slot).
    stored(&sandbox, &message(a, &a1, 1), &[slot(&b1, 1)], 10).await;
    stored(&sandbox, &message(b, &b1, 2), &[slot(&a1, 2)], 10).await;
    // And something that is not Alice's.
    stored(
        &sandbox,
        &message(b, &b1, 3),
        &[slot(&new_device(3).id, 3)],
        10,
    )
    .await;

    assert_eq!(
        social::counts(db).await.unwrap(),
        Counts {
            devices: 3,
            friendships: 1,
            groups: 3,
            pending_slots: 3
        }
    );

    sql(&sandbox, &format!("DELETE FROM account WHERE id = '{a}'")).await;

    for table in [
        "friendship",
        "friend_request",
        "social_block",
        "group_invite",
    ] {
        assert_eq!(rows(&sandbox, table).await, 0, "{table}");
    }
    assert_eq!(rows(&sandbox, "social_device").await, 2);
    assert!(
        social::devices_of(db, &[a]).await.unwrap().is_empty()
            && social::device_by_id(db, &a1).await.unwrap().is_none()
    );
    // Alice's message and its slot, and the slot on her device; Bob's other
    // message to his own device stays.
    assert_eq!(rows(&sandbox, "chat_message").await, 2);
    assert_eq!(rows(&sandbox, "chat_delivery").await, 1);
    assert_eq!(social::pending(db, &b1, &[], 10).await.unwrap().len(), 0);
    assert!(social::group_ids_of(db, a).await.unwrap().is_empty());
    assert_eq!(rows(&sandbox, "group_member").await, 2);
    assert!(social::friends(db, b).await.unwrap().is_empty());
    assert!(social::incoming(db, c).await.unwrap().is_empty());

    // Her groups are still counted until the statement that follows a
    // deletion; then the empty one goes and the shared one is Bob's.
    assert_eq!(social::counts(db).await.unwrap().groups, 3);
    assert_eq!(
        social::reown(db).await.unwrap(),
        Reowned {
            handed_on: 1,
            deleted: 1
        }
    );
    assert!(social::group(db, lonely).await.unwrap().is_none());
    assert_eq!(
        social::group(db, shared).await.unwrap().unwrap().role(b),
        Some(true)
    );
    assert_eq!(
        social::counts(db).await.unwrap(),
        Counts {
            devices: 2,
            friendships: 0,
            groups: 2,
            pending_slots: 1
        }
    );
    // The message Bob sent to Alice's device has no slot left; the sweep
    // takes it, and nothing else.
    assert_eq!(
        social::sweep(db, OffsetDateTime::now_utc()).await.unwrap(),
        Swept {
            slots: 0,
            messages: 1
        }
    );
    assert_eq!(rows(&sandbox, "chat_message").await, 1);
    sandbox.close().await;
}
