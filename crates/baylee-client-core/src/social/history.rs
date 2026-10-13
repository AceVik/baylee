//! A conversation's history on disk (§10.2): one log per conversation,
//! named by [`baylee_seal::history_name`] so the name says nothing of the
//! partner, each record a sealed message line. Up to the last
//! [`super::convs::LINES_KEPT`] load at start; a record that doesn't open
//! is skipped and counted; pruning rewrites the file without old records.

use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use baylee_protocol::social::{Delivered, Person};
use baylee_seal::{Entropy, RestKind, at_rest_open, at_rest_seal};

use super::convs::{Body, ChatLine, ConvId, LineKey, LineKind, LineState};
use super::disk::{frame, frames, history_path};

/// What a received message keeps so it can be reported later (§4.7): the
/// delivery as it came, its key and its exact plaintext. Kept only inside
/// the sealed history and in memory; `Debug` prints none of it.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    /// The delivery, every field base64url as on the wire.
    pub sealed: Delivered,
    /// The message key, base64url.
    pub mk: String,
    /// The plaintext body's exact bytes, base64url.
    pub body: String,
}

impl std::fmt::Debug for Evidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Evidence(..)")
    }
}

impl Drop for Evidence {
    fn drop(&mut self) {
        use zeroize::Zeroize as _;
        self.mk.zeroize();
        self.body.zeroize();
    }
}

/// One message line as a history record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    /// The message id.
    pub id: String,
    /// When the gateway received it, Unix milliseconds.
    pub at: u64,
    /// Who wrote it.
    pub from: Person,
    /// Whether this account wrote it.
    pub mine: bool,
    /// What it says.
    pub body: Body,
    /// What a report needs, for a received message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<Evidence>,
}

impl Record {
    /// The line it draws as, in `conv`.
    #[must_use]
    pub fn line(&self, conv: &ConvId, me: &str) -> ChatLine {
        ChatLine {
            key: LineKey {
                conv: conv.clone(),
                id: self.id.clone(),
            },
            at: self.at,
            kind: LineKind::Message {
                from: self.from.clone(),
                mine: self.mine,
                body: self.body.clone(),
            },
            state: LineState::Final,
            hidden: false,
            mentions_me: !self.mine && self.body.mentions_tag(me),
            failure: None,
        }
    }
}

/// A conversation's log file name (`<h>.log`), which is also its records'
/// at-rest name.
#[must_use]
pub fn file_name(vault_key: &[u8; 32], conv: &ConvId) -> String {
    format!(
        "{}.log",
        baylee_seal::history_name(vault_key, &conv.to_string())
    )
}

/// The path of `conv`'s log in the chat folder.
#[must_use]
pub fn file_path(vault_key: &[u8; 32], conv: &ConvId) -> String {
    history_path(&file_name(vault_key, conv))
}

/// One record, sealed and framed for appending.
///
/// # Errors
///
/// `None` when it can't be serialised or sealed.
pub fn seal_record(
    vault_key: &[u8; 32],
    conv: &ConvId,
    record: &Record,
    rng: &mut dyn Entropy,
) -> Option<Vec<u8>> {
    let json = Zeroizing::new(serde_json::to_vec(record).ok()?);
    let sealed = at_rest_seal(
        vault_key,
        RestKind::History,
        &file_name(vault_key, conv),
        &json,
        rng,
    )
    .ok()?;
    Some(frame(&sealed))
}

/// A log read back: its records, oldest first, and how many didn't open.
#[derive(Debug, Default)]
pub struct Loaded {
    /// The records that opened.
    pub records: Vec<Record>,
    /// Records that didn't (another key, damage): skipped, never fatal.
    pub skipped: usize,
}

/// Reads a log's bytes. `name` is the file's own name (`<h>.log`).
#[must_use]
pub fn read(vault_key: &[u8; 32], name: &str, bytes: &[u8]) -> Loaded {
    let mut loaded = Loaded::default();
    for sealed in frames(bytes) {
        let record = at_rest_open(vault_key, RestKind::History, name, sealed)
            .ok()
            .and_then(|plain| serde_json::from_slice::<Record>(&plain).ok());
        match record {
            Some(record) => loaded.records.push(record),
            None => loaded.skipped += 1,
        }
    }
    loaded
}
