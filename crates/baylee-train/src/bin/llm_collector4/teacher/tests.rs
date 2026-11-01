//! The trainer's four conditions for the teachers' way in (30.10.2026):
//! exactly the collector's own gateway's database, no serialised setup, the
//! export seeing a teacher as it saw a registered one, and the teacher
//! deleted whichever way the slot ends.
//!
//! Against PostgreSQL, a fresh schema per database: without `DATABASE_URL`
//! these tests fail rather than skip, as `baylee-db`'s do. The gateway is a
//! stand-in that keeps its closed-beta keys in one of those schemas.
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use baylee_db::sea_orm::{
    ConnectionTrait as _, Database, DatabaseConnection, DbBackend, Statement,
};
use sha2::{Digest as _, Sha256};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::{TcpListener, TcpStream};

use super::{OwnDatabase, b64};
use crate::admin_cleanup::AdminCleanup;
use crate::table::{SlotAccount, account_init_retry_safe};

const ADMIN_TOKEN: &str = "collector-admin-token-for-the-teacher-tests-0001";

/// A schema of its own, migrated; dropped by [`Self::close`].
struct Sandbox {
    db: DatabaseConnection,
    admin: DatabaseConnection,
    schema: String,
}

impl Sandbox {
    async fn open(what: &str) -> Self {
        let url = std::env::var("DATABASE_URL")
            .ok()
            .filter(|url| !url.is_empty())
            .expect(
                "DATABASE_URL is not set, and these tests are about PostgreSQL:\n  \
                 export DATABASE_URL=postgres://baylee:baylee@127.0.0.1:5432/baylee",
            );
        let schema: String = format!("t_teacher_{what}_{}", uuid::Uuid::now_v7().simple())
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
        let db = baylee_db::connect(&scoped, 4)
            .await
            .expect("migrating a fresh schema");
        Self { db, admin, schema }
    }

    async fn count(&self, sql: &str) -> i64 {
        self.db
            .query_one_raw(Statement::from_string(DbBackend::Postgres, sql))
            .await
            .unwrap()
            .unwrap()
            .try_get::<i64>("", "n")
            .unwrap()
    }

    async fn close(self) {
        self.db.close().await.unwrap();
        self.admin
            .execute_unprepared(&format!("DROP SCHEMA \"{}\" CASCADE", self.schema))
            .await
            .expect("dropping the schema");
    }
}

/// What the stand-in gateway does at the player's door.
#[derive(Clone, Copy)]
struct Door {
    /// Whether it knows the teachers' sessions (it reads their database).
    knows: bool,
    /// How long `/ws-ticket` takes to answer.
    delay: Duration,
}

/// A request the stand-in saw: method, path, bearer, body.
type Seen = Arc<Mutex<Vec<(String, String, Option<String>, String)>>>;

/// A gateway on 127.0.0.1 whose admin listener keeps its closed-beta keys
/// in `keys` and whose player door is `door`. Answers its base URL.
async fn gateway(keys: DatabaseConnection, door: Door, seen: Seen) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let keys = keys.clone();
            let seen = Arc::clone(&seen);
            tokio::spawn(async move { serve(stream, keys, door, seen).await });
        }
    });
    base
}

async fn serve(mut stream: TcpStream, keys: DatabaseConnection, door: Door, seen: Seen) {
    let mut peek = [0_u8; 32];
    let n = stream.peek(&mut peek).await.unwrap_or(0);
    if peek[..n].starts_with(b"GET /lobby/ws") {
        // The presence socket: held open until the client lets go.
        if let Ok(mut socket) = tokio_tungstenite::accept_async(stream).await {
            use futures_util::StreamExt as _;
            while let Some(Ok(_)) = socket.next().await {}
        }
        return;
    }
    let mut raw = Vec::new();
    let mut buf = [0_u8; 4096];
    let (head, mut body) = loop {
        let read = stream.read(&mut buf).await.unwrap();
        assert!(read > 0, "request ended early");
        raw.extend_from_slice(&buf[..read]);
        if let Some(end) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
            break (
                String::from_utf8_lossy(&raw[..end]).into_owned(),
                raw[end + 4..].to_vec(),
            );
        }
    };
    let header = |name: &str| {
        head.lines().skip(1).find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.eq_ignore_ascii_case(name)
                .then(|| value.trim().to_owned())
        })
    };
    let length: usize = header("content-length").map_or(0, |n| n.parse().unwrap());
    while body.len() < length {
        let read = stream.read(&mut buf).await.unwrap();
        body.extend_from_slice(&buf[..read]);
    }
    let mut words = head.split_whitespace();
    let (method, path) = (
        words.next().unwrap().to_owned(),
        words.next().unwrap().to_owned(),
    );
    let bearer = header("authorization").and_then(|a| a.strip_prefix("Bearer ").map(str::to_owned));
    seen.lock().unwrap().push((
        method.clone(),
        path.clone(),
        bearer.clone(),
        String::from_utf8_lossy(&body).into_owned(),
    ));
    let admin = bearer.as_deref() == Some(ADMIN_TOKEN);
    let (status, answer) = match (method.as_str(), path.as_str()) {
        ("POST", "/admin/invites") if admin => {
            let id = baylee_db::invites::create(
                &keys,
                &baylee_db::invites::NewInvite {
                    // A fresh key each time, as the gateway makes one.
                    key_hash: Sha256::digest(uuid::Uuid::now_v7().as_bytes()).to_vec(),
                    note: Some("probe".into()),
                    uses: 1,
                    expires_at: None,
                },
            )
            .await
            .unwrap();
            (
                "201 Created",
                format!(
                    r#"{{"keys":[{{"id":"{id}","key":"never-kept"}}],"uses":1,"expires_at":null}}"#
                ),
            )
        }
        ("DELETE", p) if admin && p.starts_with("/admin/invites/") => {
            let id = uuid::Uuid::parse_str(&p["/admin/invites/".len()..]).unwrap();
            let now = time::OffsetDateTime::now_utc();
            let revoked = baylee_db::invites::revoke(&keys, id, now).await.unwrap();
            (
                if revoked {
                    "204 No Content"
                } else {
                    "404 Not Found"
                },
                String::new(),
            )
        }
        ("POST", "/ws-ticket") if door.knows => {
            tokio::time::sleep(door.delay).await;
            ("200 OK", r#"{"ticket":"lobby-ticket"}"#.into())
        }
        ("DELETE", "/account") if door.knows => ("204 No Content", String::new()),
        _ => ("401 Unauthorized", r#"{"error":"who"}"#.into()),
    };
    let reply = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{answer}",
        answer.len()
    );
    let _ = stream.write_all(reply.as_bytes()).await;
}

fn admin(base: &str) -> AdminCleanup {
    AdminCleanup::for_test(base, ADMIN_TOKEN).unwrap()
}

const OPEN: Door = Door {
    knows: true,
    delay: Duration::ZERO,
};

const TEACHERS: &str = "SELECT count(*) AS n FROM account WHERE username LIKE 'llmt%'";

#[tokio::test]
async fn proves_its_own_gateways_database_and_writes_nothing_into_another() {
    let own = Sandbox::open("own").await;
    let other = Sandbox::open("other").await;
    let seen = Seen::default();
    let base = gateway(own.db.clone(), OPEN, seen).await;

    let proved = OwnDatabase::prove(&admin(&base), own.db.clone()).await;
    assert!(proved.is_ok(), "{:?}", proved.err());
    // The probe key is the gateway's own write, revoked again.
    assert_eq!(
        own.count("SELECT count(*) AS n FROM invite WHERE revoked_at IS NULL")
            .await,
        0
    );
    assert_eq!(own.count("SELECT count(*) AS n FROM invite").await, 1);

    // Another gateway's database: refused before anything is written there.
    let accounts = other.count("SELECT count(*) AS n FROM account").await;
    let refused = OwnDatabase::prove(&admin(&base), other.db.clone()).await;
    let error = format!("{:#}", refused.err().expect("another database is refused"));
    assert!(error.contains("not the database of the gateway"), "{error}");
    assert_eq!(
        other.count("SELECT count(*) AS n FROM account").await,
        accounts
    );
    assert_eq!(other.count("SELECT count(*) AS n FROM invite").await, 0);
    assert_eq!(
        other.count("SELECT count(*) AS n FROM session_token").await,
        0
    );
    // Its probe key went to the gateway's own database, and is revoked.
    assert_eq!(own.count("SELECT count(*) AS n FROM invite").await, 2);
    assert_eq!(
        own.count("SELECT count(*) AS n FROM invite WHERE revoked_at IS NULL")
            .await,
        0
    );

    // A gateway that refuses the collector's token proves nothing either.
    let stranger =
        AdminCleanup::for_test(&base, "a-token-this-gateway-was-never-given-0001").unwrap();
    assert!(OwnDatabase::prove(&stranger, own.db.clone()).await.is_err());

    own.close().await;
    other.close().await;
}

#[tokio::test]
async fn a_teacher_is_what_registration_made() {
    let own = Sandbox::open("shape").await;
    let base = gateway(own.db.clone(), OPEN, Seen::default()).await;
    let proved = OwnDatabase::prove(&admin(&base), own.db.clone())
        .await
        .unwrap();
    let teacher = proved.teacher(3).await.unwrap();
    let row = own
        .db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT username, username_key, display_name, email, invite_id::text AS invite,
                    lang, guest, central_only, tag, password_hash,
                    created_at = date_trunc('second', created_at) AS whole
             FROM account WHERE id = $1",
            [teacher.account.into()],
        ))
        .await
        .unwrap()
        .expect("the teacher's row");
    let text = |column: &str| row.try_get::<Option<String>>("", column).unwrap();
    let username = text("username").unwrap();
    // The fields the official export reads and anonymises
    // (`baylee_db::records::seated_identities`), as registration wrote them.
    assert!(
        baylee_protocol::names::username(&username).is_ok(),
        "{username}"
    );
    assert!(
        username.starts_with("llmt") && username.len() == 24,
        "{username}"
    );
    assert_eq!(text("username_key"), Some(username.to_lowercase()));
    assert_eq!(text("display_name").as_deref(), Some("LLM-teacher-3"));
    assert_eq!(text("email"), None);
    assert_eq!(text("invite"), None);
    assert_eq!(text("lang").as_deref(), Some("en"));
    assert!(!row.try_get::<bool>("", "guest").unwrap());
    assert!(!row.try_get::<bool>("", "central_only").unwrap());
    assert!(row.try_get::<i32>("", "tag").unwrap() > 0);
    assert!(row.try_get::<bool>("", "whole").unwrap());
    // A hash for which no password exists, in the column's shape.
    let hash = text("password_hash").unwrap();
    let parts: Vec<&str> = hash.split('$').collect();
    assert_eq!(&parts[..4], ["", "argon2id", "v=19", "m=19456,t=2,p=1"]);
    assert_eq!((parts[4].len(), parts[5].len()), (22, 43));
    // Its session is the bearer's digest, an account session's twelve hours.
    let session = own
        .db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT extract(epoch FROM expires_at - clock_timestamp())::bigint AS left
             FROM session_token WHERE token_hash = $1 AND account_id = $2",
            [
                Sha256::digest(teacher.token.as_bytes()).to_vec().into(),
                teacher.account.into(),
            ],
        ))
        .await
        .unwrap()
        .expect("the teacher's session");
    let left = session.try_get::<i64>("", "left").unwrap();
    assert!((11 * 3600..=12 * 3600).contains(&left), "{left}");
    own.close().await;
}

#[tokio::test]
async fn ten_slots_come_in_together() {
    let own = Sandbox::open("together").await;
    let delay = Duration::from_millis(400);
    let base = gateway(own.db.clone(), Door { knows: true, delay }, Seen::default()).await;
    let proved = Arc::new(
        OwnDatabase::prove(&admin(&base), own.db.clone())
            .await
            .unwrap(),
    );
    let started = Instant::now();
    let slots: Vec<_> = (0..10)
        .map(|slot| {
            let proved = Arc::clone(&proved);
            let base = base.clone();
            tokio::spawn(async move { SlotAccount::init(&proved, &base, slot).await })
        })
        .collect();
    let mut accounts = Vec::new();
    for slot in slots {
        accounts.push(slot.await.unwrap().expect("a slot's teacher"));
    }
    // One after the other would take ten times the door's delay.
    let took = started.elapsed();
    assert!(took < delay * 5, "{took:?}");
    assert_eq!(own.count(TEACHERS).await, 10);
    for mut account in accounts {
        account.shutdown().await.unwrap();
    }
    own.close().await;
}

#[tokio::test]
async fn a_gateway_that_does_not_know_the_teacher_leaves_no_teacher_behind() {
    let own = Sandbox::open("stranger").await;
    let seen = Seen::default();
    let proving = gateway(own.db.clone(), OPEN, Seen::default()).await;
    // `--gateway` names a gateway on another database: the session is unknown.
    let stranger = gateway(
        own.db.clone(),
        Door {
            knows: false,
            delay: Duration::ZERO,
        },
        Arc::clone(&seen),
    )
    .await;
    let proved = OwnDatabase::prove(&admin(&proving), own.db.clone())
        .await
        .unwrap();
    let refused = SlotAccount::init(&proved, &stranger, 0).await;
    let error = refused.err().expect("an unknown session is refused");
    // Cleaned up, so a retry may make another; and nothing is left over.
    assert!(account_init_retry_safe(&error), "{error:#}");
    assert_eq!(own.count(TEACHERS).await, 0);
    assert_eq!(
        own.count("SELECT count(*) AS n FROM session_token").await,
        0
    );
    // It asked the stranger to delete it first, which did not know it either.
    let asked: Vec<_> = seen
        .lock()
        .unwrap()
        .iter()
        .map(|r| (r.0.clone(), r.1.clone()))
        .collect();
    assert_eq!(
        asked,
        [
            ("POST".to_owned(), "/ws-ticket".to_owned()),
            ("DELETE".to_owned(), "/account".to_owned())
        ]
    );
    own.close().await;
}

#[tokio::test]
async fn the_slot_deletes_its_teacher_through_the_gateway_with_no_password() {
    let own = Sandbox::open("farewell").await;
    let seen = Seen::default();
    let base = gateway(own.db.clone(), OPEN, Arc::clone(&seen)).await;
    let proved = OwnDatabase::prove(&admin(&base), own.db.clone())
        .await
        .unwrap();
    let mut account = SlotAccount::init(&proved, &base, 0).await.unwrap();
    let token = account.token.clone();
    account.shutdown().await.unwrap();
    let requests = seen.lock().unwrap().clone();
    let farewell = requests
        .iter()
        .find(|(method, path, ..)| method == "DELETE" && path == "/account")
        .expect("DELETE /account");
    assert_eq!(farewell.2.as_deref(), Some(token.as_str()));
    assert_eq!(farewell.3, "{}");
    assert!(
        requests
            .iter()
            .all(|(_, _, _, body)| !body.contains("password"))
    );
    own.close().await;
}

#[test]
fn the_hash_is_written_as_phc_writes_base64() {
    assert_eq!(b64(b""), "");
    assert_eq!(b64(b"f"), "Zg");
    assert_eq!(b64(b"fo"), "Zm8");
    assert_eq!(b64(b"foo"), "Zm9v");
    assert_eq!(b64(b"foobar"), "Zm9vYmFy");
    assert_eq!(b64(&[0xff, 0xfe, 0xfd]), "//79");
    assert_eq!(b64(&[0_u8; 16]).len(), 22);
    assert_eq!(b64(&[0_u8; 32]).len(), 43);
}
