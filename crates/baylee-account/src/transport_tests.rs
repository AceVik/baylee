//! Both transports against a server on this machine that answers as told:
//! what they send, what they believe, and what they refuse to send twice.

use crate::{CentralHttp, Endpoint, Error, GatewayHttp, SignedToken, Token, wire::Request};
use std::{
    io::{Read as _, Write as _},
    net::{TcpListener, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    time::Duration,
};
use zeroize::Zeroizing;

/// One answer.
struct Reply {
    status: u16,
    headers: String,
    body: Vec<u8>,
}

impl Reply {
    fn new(status: u16) -> Self {
        Self {
            status,
            headers: String::new(),
            body: Vec::new(),
        }
    }

    fn of_type(mut self, kind: &str, body: impl Into<Vec<u8>>) -> Self {
        self.headers = format!("Content-Type: {kind}\r\n");
        self.body = body.into();
        self
    }

    fn json(status: u16, body: impl Into<Vec<u8>>) -> Self {
        Self::new(status).of_type("application/json", body)
    }
}

/// One request as it arrived.
#[derive(Debug)]
struct Seen {
    head: String,
    body: Vec<u8>,
}

impl Seen {
    fn header(&self, name: &str) -> Option<&str> {
        self.head.lines().skip(1).find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.eq_ignore_ascii_case(name).then(|| value.trim())
        })
    }

    fn line(&self) -> &str {
        self.head.lines().next().unwrap()
    }
}

/// Answers in order; a request beyond the last answer is recorded and gets 500.
struct Server {
    origin: String,
    stop: Arc<AtomicBool>,
    thread: JoinHandle<Vec<Seen>>,
}

impl Server {
    fn new(replies: Vec<Reply>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&stop);
        let thread = std::thread::spawn(move || {
            let mut replies = replies.into_iter();
            let mut seen = Vec::new();
            while !stopped.load(Ordering::Acquire) {
                let Ok((stream, _)) = listener.accept() else {
                    std::thread::sleep(Duration::from_millis(2));
                    continue;
                };
                seen.push(answer(
                    stream,
                    &replies.next().unwrap_or_else(|| Reply::new(500)),
                ));
            }
            seen
        });
        Self {
            origin,
            stop,
            thread,
        }
    }

    fn central(&self) -> CentralHttp {
        CentralHttp::new(Endpoint::development(&self.origin).unwrap())
    }

    fn gateway(&self) -> GatewayHttp {
        GatewayHttp::development(&self.origin).unwrap()
    }

    /// Everything that was asked.
    fn seen(self) -> Vec<Seen> {
        self.stop.store(true, Ordering::Release);
        self.thread.join().unwrap()
    }
}

fn answer(mut stream: TcpStream, reply: &Reply) -> Seen {
    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut head = Vec::new();
    let mut byte = [0];
    while !head.ends_with(b"\r\n\r\n") && stream.read_exact(&mut byte).is_ok() {
        head.push(byte[0]);
    }
    let mut seen = Seen {
        head: String::from_utf8(head).unwrap(),
        body: Vec::new(),
    };
    let length = seen
        .header("Content-Length")
        .map_or(0, |n| n.parse().unwrap());
    seen.body.resize(length, 0);
    stream.read_exact(&mut seen.body).unwrap();
    // A client that has read enough may hang up before the last byte.
    let _ = write!(
        stream,
        "HTTP/1.1 {} Test\r\nContent-Length: {}\r\nConnection: close\r\n{}\r\n",
        reply.status,
        reply.body.len(),
        reply.headers
    );
    let _ = stream.write_all(&reply.body);
    seen
}

fn token() -> Token {
    Token::generate().unwrap()
}

fn signed() -> SignedToken {
    SignedToken::new(Zeroizing::new("aaa.bbb.ccc".into())).unwrap()
}

const HEX: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// What each status the transports tell apart comes to.
const STATUSES: [(u16, Error); 8] = [
    (400, Error::Rejected),
    (401, Error::Rejected),
    (404, Error::Rejected),
    (409, Error::Conflict),
    (426, Error::UpdateRequired),
    (500, Error::Unavailable),
    (503, Error::Unavailable),
    (599, Error::Unavailable),
];

// ------------------------------------------------------------- the key set

const JWKS: &str = "application/jwk-set+json";

#[test]
fn the_key_set_is_fetched_without_credentials_and_returned_as_sent() {
    let server = Server::new(vec![Reply::new(200).of_type(JWKS, "{\"keys\":[]}")]);
    assert_eq!(server.central().jwks().unwrap(), b"{\"keys\":[]}");
    let seen = server.seen();
    assert_eq!(seen[0].line(), "GET /.well-known/jwks.json HTTP/1.1");
    assert_eq!(seen[0].header("Authorization"), None);
}

#[test]
fn a_key_set_may_be_sixteen_kibibytes_and_not_one_byte_more() {
    for (size, taken) in [(16 * 1024, true), (16 * 1024 + 1, false)] {
        let server = Server::new(vec![Reply::new(200).of_type(JWKS, vec![b' '; size])]);
        assert_eq!(server.central().jwks().is_ok(), taken, "{size}");
    }
}

#[test]
fn a_key_set_of_another_type_or_status_is_refused() {
    let ask = |reply| Server::new(vec![reply]).central().jwks();
    let body = || Reply::json(200, "{\"keys\":[]}");
    assert_eq!(ask(body()), Err(Error::Rejected));
    assert_eq!(ask(Reply::new(200)), Err(Error::Rejected));
    for (status, error) in [
        (201, Error::Rejected),
        (404, Error::Rejected),
        (429, Error::Busy),
        (500, Error::Unavailable),
        (599, Error::Unavailable),
    ] {
        let reply = Reply::new(status).of_type(JWKS, "{\"keys\":[]}");
        assert_eq!(ask(reply), Err(error), "{status}");
    }
}

#[test]
fn a_redirect_is_never_followed() {
    let elsewhere = Server::new(vec![Reply::new(200).of_type(JWKS, "{\"keys\":[]}")]);
    let mut reply = Reply::new(302);
    reply.headers = format!("Location: {}/.well-known/jwks.json\r\n", elsewhere.origin);
    let server = Server::new(vec![reply]);
    assert_eq!(server.central().jwks(), Err(Error::Rejected));
    assert_eq!(elsewhere.seen().len(), 0);
}

// ------------------------------------------------------------ the exchange

#[test]
fn an_exchange_names_its_protocol_and_presents_its_bearer() {
    let server = Server::new(vec![Reply::json(200, "\"nonsense\"")]);
    let central = server.central();
    let bearer = token();
    // Nothing in the answer decodes, which is refused rather than guessed.
    assert!(matches!(
        central.exchange(&Request::TowerToken {}, Some((&bearer).into())),
        Err(Error::Rejected)
    ));
    let seen = server.seen();
    assert_eq!(seen[0].header("Baylee-Central-Protocol"), Some("2"));
    assert_eq!(seen[0].header("Content-Type"), Some("application/json"));
    assert_eq!(
        seen[0].header("Authorization"),
        Some(bearer.bearer().as_str())
    );
    let sent: serde_json::Value = serde_json::from_slice(&seen[0].body).unwrap();
    assert_eq!(sent, serde_json::to_value(Request::TowerToken {}).unwrap());
}

#[test]
fn every_refusal_of_an_exchange_keeps_its_meaning_and_is_not_sent_again() {
    for (status, error) in STATUSES
        .into_iter()
        .chain([(403, Error::TermsRequired), (302, Error::Rejected)])
    {
        let server = Server::new(vec![Reply::json(status, "{}")]);
        let result = server.central().exchange(&Request::TowerToken {}, None);
        assert_eq!(result.err(), Some(error), "{status}");
        assert_eq!(server.seen().len(), 1, "{status}");
    }
}

#[test]
fn a_busy_exchange_is_sent_again_with_the_same_body() {
    let server = Server::new(vec![Reply::new(429), Reply::json(409, "{}")]);
    let result = server.central().exchange(&Request::TowerToken {}, None);
    assert_eq!(result.err(), Some(Error::Conflict));
    let seen = server.seen();
    assert_eq!(seen.len(), 2);
    assert_eq!(seen[0].body, seen[1].body);
}

#[test]
fn an_answer_that_is_not_json_or_too_long_is_refused() {
    let ask = |reply| {
        Server::new(vec![reply])
            .central()
            .exchange(&Request::TowerToken {}, None)
            .err()
    };
    let page = Reply::new(200).of_type("text/html", "{}");
    assert_eq!(ask(page), Some(Error::Rejected));
    let long = Reply::json(200, vec![b' '; central_api::MAX_WIRE_BYTES + 1]);
    assert_eq!(ask(long), Some(Error::Rejected));
}

#[test]
fn the_last_bearer_is_remembered_until_the_session_signs_out() {
    let server = Server::new(Vec::new());
    let central = server.central();
    assert!(central.last_bearer().is_none());
    let bearer = token();
    let _ = central.exchange(&Request::TowerToken {}, Some((&bearer).into()));
    assert_eq!(central.last_bearer(), Some(bearer.bearer()));
    // An exchange without one leaves the last in place.
    let _ = central.exchange(&Request::TowerToken {}, None);
    assert_eq!(central.last_bearer(), Some(bearer.bearer()));
    let _ = central.exchange(&Request::Logout, Some((&bearer).into()));
    assert!(central.last_bearer().is_none());
}

// ------------------------------------------------------------- the reports

fn intake() -> crate::reports::ReportIntake {
    crate::reports::ReportIntake {
        gateway: crate::reports::GatewaySays::default(),
        channel: crate::reports::Channel::Gateway,
        reporter: HEX.into(),
        kind: crate::reports::ReportKind::Crash,
        text: "It crashed".into(),
        game_id: None,
        build: None,
        crash: None,
        client: None,
        record: None,
    }
}

const REPORT: [u8; 16] = [0xAB; 16];

#[test]
fn a_filed_report_answers_with_the_number_central_gave_it() {
    let filed = format!("{{\"report_id\":\"{}\"}}", "ab".repeat(16));
    let reply = Reply::new(201).of_type("Application/JSON; charset=utf-8", filed);
    let server = Server::new(vec![reply]);
    let bearer = token();
    let id = server.central().file_report(&intake(), &bearer);
    assert_eq!(id, Ok(REPORT));
    let seen = server.seen();
    assert_eq!(seen[0].line(), "POST /v1/reports HTTP/1.1");
    assert_eq!(
        seen[0].header("Authorization"),
        Some(bearer.bearer().as_str())
    );
    assert_eq!(seen[0].header("Baylee-Central-Protocol"), Some("2"));
    let sent: serde_json::Value = serde_json::from_slice(&seen[0].body).unwrap();
    assert_eq!(sent, serde_json::to_value(intake()).unwrap());
}

#[test]
fn a_filed_report_that_is_not_confirmed_exactly_is_refused() {
    let id = "ab".repeat(16);
    let good = format!("{{\"report_id\":\"{id}\"}}");
    let ask = |reply| {
        Server::new(vec![reply])
            .central()
            .file_report(&intake(), &token())
    };
    assert!(ask(Reply::json(201, good.clone())).is_ok());
    let wrong = [
        Reply::json(200, good.clone()),
        Reply::new(201).of_type("text/plain", good.clone()),
        Reply::new(201),
        Reply::json(201, "{\"report_id\":\"ab\"}"),
        Reply::json(
            201,
            format!("{{\"report_id\":\"{id}\",\"next\":\"elsewhere\"}}"),
        ),
        // Two hundred and fifty-six bytes are read, so this stops mid-way.
        Reply::json(201, format!("{}{good}", " ".repeat(256))),
    ];
    for (at, reply) in wrong.into_iter().enumerate() {
        assert_eq!(ask(reply), Err(Error::Rejected), "{at}");
    }
    for (status, error) in STATUSES {
        assert_eq!(ask(Reply::new(status)), Err(error), "{status}");
    }
}

#[test]
fn a_report_longer_than_central_takes_is_not_sent() {
    let server = Server::new(Vec::new());
    let mut long = intake();
    long.text = "x".repeat(crate::reports::MAX_INTAKE_BYTES);
    assert_eq!(
        server.central().file_report(&long, &token()),
        Err(Error::Rejected)
    );
    assert_eq!(server.seen().len(), 0);
}

#[test]
fn a_report_is_filed_again_while_central_is_busy() {
    let filed = format!("{{\"report_id\":\"{}\"}}", "ab".repeat(16));
    let server = Server::new(vec![Reply::new(429), Reply::json(201, filed)]);
    let id = server.central().file_report(&intake(), &token());
    assert_eq!(id, Ok(REPORT));
    let seen = server.seen();
    assert_eq!(seen.len(), 2);
    assert_eq!(seen[0].body, seen[1].body);
}

#[test]
fn a_report_of_exactly_the_longest_length_is_sent() {
    let empty = crate::reports::ReportIntake {
        text: String::new(),
        ..intake()
    };
    let bare = serde_json::to_vec(&empty).unwrap().len();
    let longest = crate::reports::ReportIntake {
        text: "x".repeat(crate::reports::MAX_INTAKE_BYTES - bare),
        ..empty
    };
    let server = Server::new(vec![Reply::new(409)]);
    let refused = server.central().file_report(&longest, &token());
    assert_eq!(refused, Err(Error::Conflict));
    let seen = server.seen();
    assert_eq!(seen[0].body.len(), crate::reports::MAX_INTAKE_BYTES);
}

#[test]
fn an_answer_is_read_to_the_last_byte_of_the_limit_and_not_past_it() {
    let admit = |length: usize| {
        let mut body = serde_json::json!({ "token": HEX }).to_string();
        body.push_str(&" ".repeat(length - body.len()));
        Server::new(vec![Reply::json(200, body)])
            .gateway()
            .admit(&signed())
            .map(drop)
            .map_err(|turned| turned.error)
    };
    assert_eq!(admit(central_api::MAX_WIRE_BYTES), Ok(()));
    assert_eq!(admit(central_api::MAX_WIRE_BYTES + 1), Err(Error::Rejected));
}

#[test]
fn a_part_is_put_under_its_report_as_the_bytes_it_is() {
    let server = Server::new(vec![Reply::new(204)]);
    let bearer = token();
    server
        .central()
        .put_report_part(
            &REPORT,
            crate::reports::Part::Record,
            &[0x1F, 0x8B, 0],
            &bearer,
        )
        .unwrap();
    let seen = server.seen();
    assert_eq!(
        seen[0].line(),
        format!("PUT /v1/reports/{}/record HTTP/1.1", "ab".repeat(16))
    );
    assert_eq!(seen[0].header("Content-Type"), Some("application/gzip"));
    assert_eq!(
        seen[0].header("Authorization"),
        Some(bearer.bearer().as_str())
    );
    assert_eq!(seen[0].body, [0x1F, 0x8B, 0]);
}

#[test]
fn a_refused_part_says_why_and_is_sent_once() {
    for (status, error) in STATUSES.into_iter().filter(|(status, _)| *status < 500) {
        let server = Server::new(vec![Reply::new(status)]);
        let put =
            server
                .central()
                .put_report_part(&REPORT, crate::reports::Part::Client, b"x", &token());
        assert_eq!(put, Err(error), "{status}");
        assert_eq!(server.seen().len(), 1, "{status}");
    }
    // Only an empty answer confirms a part.
    let server = Server::new(vec![Reply::json(200, "{}")]);
    let put =
        server
            .central()
            .put_report_part(&REPORT, crate::reports::Part::Client, b"x", &token());
    assert_eq!(put, Err(Error::Rejected));
}

// ----------------------------------------------------- signing in at a gateway

fn admission(token: &str, username: &str) -> Reply {
    let body = serde_json::json!({ "token": token, "username": username });
    Reply::json(200, body.to_string())
}

#[test]
fn a_gateway_token_is_presented_once_in_the_body_of_a_login() {
    let server = Server::new(vec![admission(HEX, "Ada"), admission(HEX, "Ada")]);
    let gateway = server.gateway();
    let admitted = gateway.admit(&signed()).unwrap();
    assert_eq!(admitted.token, HEX);
    assert_eq!(admitted.username.as_deref(), Some("Ada"));
    assert_eq!(admitted.renew_in, None);
    gateway.refresh(&signed()).unwrap();
    let seen = server.seen();
    assert_eq!(seen[0].line(), "POST /auth/central/login HTTP/1.1");
    assert_eq!(seen[1].line(), "POST /auth/central/refresh HTTP/1.1");
    assert_eq!(seen[0].body, br#"{"token":"aaa.bbb.ccc"}"#);
    assert_eq!(seen[0].header("Authorization"), None);
}

#[test]
fn a_session_that_is_no_lowercase_key_is_refused() {
    let admit = |reply| {
        Server::new(vec![reply])
            .gateway()
            .admit(&signed())
            .map(drop)
            .map_err(|turned| turned.error)
    };
    assert_eq!(admit(admission(HEX, "Ada")), Ok(()));
    for bad in [
        &HEX[1..],
        &format!("{HEX}0"),
        &HEX.to_uppercase(),
        &HEX.replace('0', "g"),
        &HEX.replace('0', "\n"),
    ] {
        assert_eq!(admit(admission(bad, "Ada")), Err(Error::Rejected), "{bad}");
    }
}

#[test]
fn a_session_name_is_short_printable_and_not_empty() {
    let admit = |name: &str| {
        Server::new(vec![admission(HEX, name)])
            .gateway()
            .admit(&signed())
            .is_ok()
    };
    assert!(admit(&"x".repeat(96)));
    assert!(!admit(&"x".repeat(97)));
    assert!(!admit(""));
    assert!(!admit("Ada\u{7}"));
}

#[test]
fn a_refused_login_says_at_most_one_kibibyte_and_one_byte() {
    for (length, read) in [(0, 0), (1024, 1024), (1025, 1025), (4096, 1025)] {
        let server = Server::new(vec![Reply::json(409, vec![b'x'; length])]);
        let turned = server.gateway().admit(&signed()).map(drop).unwrap_err();
        assert_eq!(turned.error, Error::Conflict);
        assert_eq!(turned.said.len(), read, "{length}");
    }
}

// ---------------------------------------------------------- the client's seats

#[cfg(feature = "client")]
mod seats {
    use super::*;
    use crate::advent::{AssetMeta, Finish, Kind, Mime};
    use uuid::Uuid;

    fn seat(change: impl FnOnce(&mut serde_json::Value)) -> Reply {
        let mut body = serde_json::json!({
            "game_id": Uuid::now_v7(), "seat": 0, "seat_token": HEX,
        });
        change(&mut body);
        Reply::json(200, body.to_string())
    }

    #[test]
    fn a_challenge_seat_is_claimed_for_one_attempt_of_one_player() {
        let server = Server::new(vec![seat(|_| ())]);
        let attempt = Uuid::now_v7();
        let claimed = server
            .gateway()
            .challenge_play("bearer", *attempt.as_bytes())
            .unwrap();
        assert_eq!(claimed.seat_token.as_str(), HEX);
        let seen = server.seen();
        assert_eq!(
            seen[0].line(),
            format!("POST /challenges/{attempt}/play HTTP/1.1")
        );
        assert_eq!(seen[0].header("Authorization"), Some("Bearer bearer"));
    }

    #[test]
    fn an_attempt_of_another_kind_of_identifier_is_not_asked_about() {
        let server = Server::new(vec![seat(|_| ())]);
        let gateway = server.gateway();
        assert!(gateway.challenge_play("bearer", [0; 16]).is_err());
        let run = Uuid::nil();
        assert!(gateway.tower_play("bearer", run).is_err());
        assert_eq!(server.seen().len(), 0);
    }

    #[test]
    fn a_challenge_seat_that_is_not_the_players_own_is_refused() {
        let wrong: [(&str, serde_json::Value); 5] = [
            ("seat", 1.into()),
            ("game_id", "the one in progress".into()),
            ("seat_token", HEX[1..].into()),
            ("seat_token", format!("{HEX}0").into()),
            ("seat_token", HEX.replace('0', "g").into()),
        ];
        for (field, value) in wrong {
            let server = Server::new(vec![seat(|body| body[field] = value.clone())]);
            let attempt = *Uuid::now_v7().as_bytes();
            let result = server.gateway().challenge_play("bearer", attempt);
            assert_eq!(result.err(), Some(Error::Rejected), "{field} {value}");
        }
    }

    #[test]
    fn a_ranked_seat_is_one_of_two_with_a_canonical_key() {
        let valid = serde_json::json!({
            "game_id": Uuid::now_v7(), "seat": 1, "seat_token": HEX,
        });
        let read = |value: serde_json::Value| {
            serde_json::from_value::<crate::RankedSeat>(value)
                .is_ok_and(|seat| seat.validate().is_ok())
        };
        assert!(read(valid.clone()));
        for (field, value) in [
            ("seat", serde_json::json!(2)),
            ("game_id", serde_json::json!(Uuid::nil())),
            ("seat_token", serde_json::json!(HEX.to_uppercase())),
            ("seat_token", serde_json::json!(&HEX[1..])),
            ("gateway", serde_json::json!("https://untrusted.invalid")),
        ] {
            let mut changed = valid.clone();
            changed[field] = value;
            assert!(!read(changed), "{field}");
        }
        // Version seven's nibble with another variant is not a version seven.
        let mut bytes = *Uuid::now_v7().as_bytes();
        bytes[8] |= 0xC0;
        let mut changed = valid;
        changed["game_id"] = serde_json::json!(Uuid::from_bytes(bytes));
        assert!(!read(changed));
    }

    // --------------------------------------------------- published decks

    fn published(id: Uuid, change: impl FnOnce(&mut serde_json::Value)) -> Reply {
        let mut body = serde_json::json!({
            "id": id, "kind": "house", "version": 3, "name": "Starter", "format": "modern",
            "description": null, "cards": ["4 Shock"], "sideboard": ["1 Duress"],
            "commanders": [],
        });
        change(&mut body);
        Reply::json(200, body.to_string())
    }

    #[test]
    fn a_published_deck_is_copied_with_where_it_came_from() {
        let id = Uuid::now_v7();
        let server = Server::new(vec![published(id, |_| ())]);
        let draft = server.gateway().published_deck("bearer", id, 3).unwrap();
        assert_eq!(draft.name, "Starter");
        assert_eq!(draft.description, "");
        assert_eq!(draft.cards, ["4 Shock"]);
        assert_eq!(draft.sideboard, ["1 Duress"]);
        let source = draft.copied_from.unwrap();
        assert_eq!(source.gateway.as_deref(), Some(server.origin.as_str()));
        assert_eq!(source.account, None, "a starter belongs to nobody");
        assert_eq!(source.deck, id.to_string());
        assert_eq!(source.revision, 3);
    }

    #[test]
    fn a_deck_that_is_not_the_one_chosen_is_a_conflict() {
        let id = Uuid::now_v7();
        let ask = |change: fn(&mut serde_json::Value)| {
            Server::new(vec![published(id, change)])
                .gateway()
                .published_deck("bearer", id, 3)
                .err()
        };
        assert_eq!(ask(|_| ()), None);
        assert_eq!(ask(|d| d["kind"] = "preconstructed".into()), None);
        assert_eq!(ask(|d| d["version"] = 4.into()), Some(Error::Conflict));
        assert_eq!(ask(|d| d["kind"] = "user".into()), Some(Error::Conflict));
        assert_eq!(
            ask(|d| d["id"] = Uuid::nil().to_string().into()),
            Some(Error::Conflict)
        );
    }

    #[test]
    fn a_private_export_asks_for_one_revision_or_the_next_deck() {
        let (deck, after) = (Uuid::now_v7(), Uuid::now_v7());
        let server = Server::new((0..3).map(|_| Reply::json(200, "null")).collect());
        let gateway = server.gateway();
        for (after, current) in [
            (None, None),
            (Some(after), None),
            (Some(after), Some((deck, 9))),
        ] {
            let answer = gateway.private_deck_save("bearer", after, current).unwrap();
            assert!(answer.is_none(), "the end of the list");
        }
        let seen = server.seen();
        let path = "GET /decks/private-export";
        assert_eq!(seen[0].line(), format!("{path} HTTP/1.1"));
        assert_eq!(seen[1].line(), format!("{path}?after={after} HTTP/1.1"));
        assert_eq!(
            seen[2].line(),
            format!("{path}?deck={deck}&version=9 HTTP/1.1")
        );
    }

    // ------------------------------------------------------ table cosmetics

    fn meta(size: u32) -> AssetMeta {
        AssetMeta {
            mat_framing: None,
            tier: Finish::Standard,
            digest: [1; 32],
            size,
            mime: Mime::Webp,
            width: 8,
            height: 8,
        }
    }

    #[test]
    fn table_art_is_only_asked_for_with_a_seat_of_a_real_game() {
        let server = Server::new(Vec::new());
        let gateway = server.gateway();
        let game = Uuid::now_v7();
        let art = |game, token: &str, seat, meta: &AssetMeta| {
            gateway
                .advent_table_asset(game, token, seat, Kind::Sleeve, meta, || true)
                .err()
        };
        let refused = Some(Error::Rejected);
        assert_eq!(art(Uuid::nil(), HEX, 0, &meta(1)), refused);
        assert_eq!(art(game, &HEX[1..], 0, &meta(1)), refused);
        assert_eq!(art(game, &HEX.to_uppercase(), 0, &meta(1)), refused);
        assert_eq!(art(game, HEX, 8, &meta(1)), refused);
        assert_eq!(art(game, HEX, 0, &meta(0)), refused);
        assert_eq!(
            gateway.advent_table_looks(Uuid::nil(), HEX).err(),
            refused,
            "nor is what the seats wear"
        );
        assert_eq!(server.seen().len(), 0);
    }

    #[test]
    fn a_player_who_left_the_table_downloads_no_more_art() {
        let server = Server::new(Vec::new());
        let left = server.gateway().advent_table_asset(
            Uuid::now_v7(),
            HEX,
            7,
            Kind::Playmat,
            &meta(1),
            || false,
        );
        assert_eq!(left.err(), Some(Error::Conflict));
        assert_eq!(server.seen().len(), 0);
    }

    #[test]
    fn the_looks_of_a_table_have_eight_seats_and_no_ninth() {
        let body = |seat: u8, size: u32| {
            let sleeve = serde_json::to_value(meta(size)).unwrap();
            serde_json::json!({ seat.to_string(): { "event_sleeve": sleeve } }).to_string()
        };
        let ask = |body: String| {
            Server::new(vec![Reply::json(200, body)])
                .gateway()
                .advent_table_looks(Uuid::now_v7(), HEX)
        };
        let looks = ask(body(7, 9)).unwrap();
        assert!(looks[7].sleeve.as_ref().is_some_and(|art| art.size == 9));
        assert!(looks[7].playmat.is_none());
        assert!(looks[0].sleeve.is_none());
        assert_eq!(ask(body(8, 9)).err(), Some(Error::Rejected));
        assert_eq!(ask(body(7, 0)).err(), Some(Error::Rejected));
    }

    /// A picture no compression makes smaller than one page, and its meta.
    fn picture() -> (AssetMeta, Vec<u8>) {
        use sha2::{Digest as _, Sha256};
        let mut noise = 1_u32;
        let pixels: Vec<u8> = (0..320 * 320 * 3)
            .map(|_| {
                noise = noise.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                noise.to_be_bytes()[0]
            })
            .collect();
        let mut bytes = Vec::new();
        image::codecs::webp::WebPEncoder::new_lossless(&mut bytes)
            .encode(&pixels, 320, 320, image::ExtendedColorType::Rgb8)
            .unwrap();
        let meta = AssetMeta {
            digest: Sha256::digest(&bytes).into(),
            width: 320,
            height: 320,
            ..meta(u32::try_from(bytes.len()).unwrap())
        };
        (meta, bytes)
    }

    fn page(asset: AssetMeta, offset: u32, bytes: &[u8]) -> Reply {
        let page = crate::advent::AssetPage {
            asset,
            offset,
            bytes: bytes.to_vec(),
        };
        let body = serde_json::to_vec(&crate::advent::Response::Asset { page }).unwrap();
        Reply::json(200, body)
    }

    /// Downloads `meta` from a table that answers with these pages.
    fn download(meta: &AssetMeta, pages: Vec<Reply>) -> (Result<Vec<u8>, Error>, Vec<Seen>) {
        let server = Server::new(pages);
        let game = Uuid::now_v7();
        let result = server
            .gateway()
            .advent_table_asset(game, HEX, 3, Kind::Playmat, meta, || true);
        (result, server.seen())
    }

    #[test]
    fn table_art_is_fetched_page_by_page_and_checked_as_a_whole() {
        let (meta, bytes) = picture();
        let cut = crate::advent::PAGE_BYTES as usize;
        assert!(bytes.len() > cut && bytes.len() < 2 * cut, "two pages");
        let pages = vec![
            page(meta, 0, &bytes[..cut]),
            page(meta, crate::advent::PAGE_BYTES, &bytes[cut..]),
        ];
        let (result, seen) = download(&meta, pages);
        assert_eq!(result, Ok(bytes));
        assert_eq!(seen.len(), 2, "and nothing is asked after the last byte");
        for (asked, offset) in seen.iter().zip([0, crate::advent::PAGE_BYTES]) {
            let body: serde_json::Value = serde_json::from_slice(&asked.body).unwrap();
            assert_eq!(
                body,
                serde_json::json!({"seat": 3, "kind": Kind::Playmat, "offset": offset})
            );
            let bearer = format!("Bearer {HEX}");
            assert_eq!(asked.header("Authorization"), Some(bearer.as_str()));
        }
    }

    #[test]
    fn a_page_of_other_art_another_place_or_another_size_ends_the_download() {
        let (meta, bytes) = picture();
        let cut = crate::advent::PAGE_BYTES as usize;
        let refused = |reply| {
            let (result, seen) = download(&meta, vec![reply]);
            assert_eq!(seen.len(), 1);
            result.map(drop)
        };
        let other = AssetMeta {
            digest: [9; 32],
            ..meta
        };
        assert_eq!(refused(page(other, 0, &bytes[..cut])), Err(Error::Conflict));
        assert_eq!(refused(page(meta, 1, &bytes[..cut])), Err(Error::Rejected));
        assert_eq!(
            refused(page(meta, 0, &bytes[..cut - 1])),
            Err(Error::Rejected)
        );
        let exchange = Reply::json(200, "{\"answer\":\"updated\"}");
        assert_eq!(refused(exchange), Err(Error::Rejected));
    }

    #[test]
    fn art_whose_bytes_are_not_the_ones_promised_is_not_shown() {
        let (meta, mut bytes) = picture();
        let cut = crate::advent::PAGE_BYTES as usize;
        *bytes.last_mut().unwrap() ^= 1;
        let pages = vec![
            page(meta, 0, &bytes[..cut]),
            page(meta, crate::advent::PAGE_BYTES, &bytes[cut..]),
        ];
        assert_eq!(download(&meta, pages).0.map(drop), Err(Error::Rejected));
    }

    #[test]
    fn a_game_of_another_variant_of_identifier_is_not_asked_about() {
        let server = Server::new(Vec::new());
        let mut bytes = *Uuid::now_v7().as_bytes();
        bytes[8] |= 0xC0;
        let game = Uuid::from_bytes(bytes);
        let looks = server.gateway().advent_table_looks(game, HEX);
        assert_eq!(looks.err(), Some(Error::Rejected));
        assert_eq!(server.seen().len(), 0);
    }

    #[test]
    fn every_field_of_a_published_deck_is_copied() {
        let id = Uuid::now_v7();
        let server = Server::new(vec![published(id, |deck| {
            deck["description"] = "Burn them.".into();
            deck["commanders"] = serde_json::json!(["Krenko, Mob Boss"]);
        })]);
        let draft = server.gateway().published_deck("bearer", id, 3).unwrap();
        assert_eq!(draft.format, "modern");
        assert_eq!(draft.description, "Burn them.");
        assert_eq!(draft.commanders, ["Krenko, Mob Boss"]);
        let seen = server.seen();
        assert_eq!(seen[0].line(), format!("GET /decks/{id} HTTP/1.1"));
        assert_eq!(seen[0].header("Authorization"), Some("Bearer bearer"));
    }
}
