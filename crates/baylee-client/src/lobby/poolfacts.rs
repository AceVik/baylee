//! The pool's table (`GET /pool/facts`, `docs/protocol.md` §"Pool facts"),
//! asked of the gateway off the one-request chain and kept by version.
//!
//! The table is a few hundred kilobytes and changes with an ingest, not with a
//! session, so a copy is kept on this machine, one per gateway, as the bytes
//! it was sent in (the settings store's named documents, as card text is).
//! The next session names the version it keeps and is told `304`. The copy is
//! used only on that `304`, so it can never stand for a table the gateway no
//! longer holds, and no copy is read offline: without a session nobody can
//! say it is still this gateway's, and the keys say they are not available.

#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;
use baylee_client_core::cardquery::{PoolFacts, PoolFactsBody, pool_facts_document};
use std::sync::Arc;

/// `GET /banlists`.
#[derive(serde::Deserialize)]
struct ListsBody {
    lists: Vec<baylee_cardtext::ListRow>,
}

/// The version a kept copy holds, read without keeping the rest.
fn version_of(kept: &str) -> Option<String> {
    #[derive(serde::Deserialize)]
    struct Version {
        version: String,
    }
    serde_json::from_str::<Version>(kept)
        .ok()
        .map(|kept| kept.version)
        .filter(|version| !version.is_empty())
}

/// What an answer comes to.
#[derive(Debug, PartialEq, Eq)]
enum Reading {
    /// The table to use, and the bytes to keep when they are new.
    Table(PoolFacts, Option<String>),
    /// The session is spent.
    Expired,
    /// Nothing to go on: an older gateway, a failure, a body nobody can read.
    /// The keys go on saying they are not available.
    Nothing,
}

/// Reads an answer against the copy that was named in the question.
fn read(answer: &Result<ehttp::Response, String>, kept: Option<&str>) -> Reading {
    let parse = |text: &str| {
        serde_json::from_str::<PoolFactsBody>(text)
            .ok()
            .map(PoolFacts::from)
    };
    match answer {
        Ok(response) if response.status == 304 => kept
            .and_then(parse)
            .map_or(Reading::Nothing, |table| Reading::Table(table, None)),
        Ok(response) if response.ok => {
            let Some(text) = response.text() else {
                return Reading::Nothing;
            };
            parse(text).map_or(Reading::Nothing, |table| {
                // A table with no version says nothing and is not worth a file.
                let keep = (!table.version().is_empty()).then(|| text.to_string());
                Reading::Table(table, keep)
            })
        }
        Ok(response) if response.status == 401 => Reading::Expired,
        _ => Reading::Nothing,
    }
}

/// Asks for the table, once a session, wherever there is a gateway to ask.
///
/// Called beside the pool, and by a table whose player never opened the deck
/// builder. Offline there is no session, and so no question.
pub(super) fn ask(state: &mut LobbyState, mailbox: &Mailbox) {
    if state.offline.is_some() {
        return;
    }
    let Some(token) = state.lobby.token().map(str::to_owned) else {
        return;
    };
    if !state.lobby.builder_mut().ask_pool_facts() {
        return;
    }
    // `list:` reads the lists as `r:` reads the table, so they are asked at
    // the same moment; a gateway from before lists answers nothing, and the
    // key says so. A `Banlists` chain keeps them fresh after an edit.
    super::http::ask_aside::<ListsBody>(
        &state.gateway,
        "/banlists",
        &token,
        state.gateway_epoch,
        mailbox,
        |body| Reply::Event(LobbyEvent::Banlists(body.lists)),
    );
    let document = pool_facts_document(&state.gateway);
    let kept = crate::settings::store::read_named(&document);
    let have = kept.as_deref().and_then(version_of);
    let path = have.map_or_else(
        || "/pool/facts".to_string(),
        |version| format!("/pool/facts?have={}", super::http::escape(&version)),
    );
    let url = format!("{}{path}", state.gateway.trim_end_matches('/'));
    let request = super::http::bearer(
        ehttp::Request::get(url).with_timeout(Some(super::http::PROBE_TIMEOUT)),
        Some(&token),
    );
    let box_ = Arc::clone(&mailbox.0);
    let epoch = state.gateway_epoch;
    crate::transport::fetch(request, move |answer| {
        let reply = match read(&answer, kept.as_deref()) {
            Reading::Table(table, keep) => {
                if let Some(text) = keep {
                    crate::settings::store::write_named(&document, &text);
                }
                Reply::Event(LobbyEvent::PoolFacts(Arc::new(table)))
            }
            Reading::Expired => Reply::Expired,
            Reading::Nothing => return,
        };
        if let Ok(mut box_) = box_.lock() {
            box_.push(Reply::Remote(epoch, Box::new(reply)));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::cardquery::PoolSet;

    const TABLE: &str = r#"{"version":"abc","formats":["modern"],"sets":[{"code":"m21","name":"Core Set 2021"}],"verdicts":["b"],"cards":[[7,0,4]]}"#;

    fn response(status: u16, body: &str) -> ehttp::Response {
        ehttp::Response {
            url: String::new(),
            ok: (200..300).contains(&status),
            status,
            status_text: String::new(),
            headers: ehttp::Headers::default(),
            bytes: body.as_bytes().to_vec(),
        }
    }

    #[test]
    fn a_new_table_is_read_and_kept_as_it_was_sent() {
        let Reading::Table(table, keep) = read(&Ok(response(200, TABLE)), None) else {
            panic!("a table was sent");
        };
        assert_eq!(table.version(), "abc");
        assert_eq!(
            table.sets(),
            [PoolSet {
                code: "m21".to_string(),
                name: "Core Set 2021".to_string()
            }]
        );
        assert_eq!(keep.as_deref(), Some(TABLE));
    }

    /// `304` is the only answer the copy is ever used on.
    #[test]
    fn the_kept_copy_stands_only_when_the_gateway_says_it_still_does() {
        let Reading::Table(table, keep) = read(&Ok(response(304, "")), Some(TABLE)) else {
            panic!("the kept copy is the table");
        };
        assert_eq!(table.version(), "abc");
        assert_eq!(keep, None, "it is on disk already");
        assert_eq!(read(&Ok(response(304, "")), None), Reading::Nothing);
        assert_eq!(
            read(&Ok(response(304, "")), Some("not json")),
            Reading::Nothing,
            "a copy gone bad is no copy"
        );
    }

    #[test]
    fn a_table_with_nothing_in_it_is_used_and_not_kept() {
        let empty = r#"{"version":"","formats":[],"sets":[],"verdicts":[],"cards":[]}"#;
        let Reading::Table(table, keep) = read(&Ok(response(200, empty)), None) else {
            panic!("an empty table is still an answer");
        };
        assert!(table.is_empty());
        assert_eq!(keep, None);
    }

    /// A gateway from before the route, a refusal, a garbled body: the keys
    /// stay unavailable and nothing else about the session changes.
    #[test]
    fn anything_else_leaves_the_keys_unavailable() {
        assert_eq!(read(&Ok(response(404, "")), Some(TABLE)), Reading::Nothing);
        assert_eq!(read(&Ok(response(502, "")), None), Reading::Nothing);
        assert_eq!(read(&Ok(response(200, "<html>")), None), Reading::Nothing);
        assert_eq!(read(&Err("no route".to_string()), None), Reading::Nothing);
        assert_eq!(read(&Ok(response(401, "")), None), Reading::Expired);
    }

    #[test]
    fn a_copy_is_read_by_its_version() {
        assert_eq!(version_of(TABLE).as_deref(), Some("abc"));
        assert_eq!(version_of("{\"version\":\"\"}"), None);
        assert_eq!(version_of("not json"), None);
    }
}
