//! baylee-protocol — binary websocket protocol (protobuf), wasm-safe.
//!
//! v0: transport handshake + preset transfer (see `docs/protocol.md`).
//! Choice/action/view messages land with the engine API (M1–M3).

#![warn(missing_docs)]

/// Wire protocol version; incompatible versions refuse the session.
pub const PROTOCOL_VERSION: u32 = 1;

/// Generated protobuf types (`baylee.v1`).
#[allow(missing_docs, clippy::all, clippy::pedantic)]
pub mod v1 {
    include!(concat!(env!("OUT_DIR"), "/baylee.v1.rs"));
}

/// One remembered answer, as it travels from the gateway to an engine.
///
/// The gateway keeps these per account and the engine turns them back into
/// `SetStandingAnswer` actions. It lives here rather than in either end
/// because it is the shape of a payload on the wire, and the two ends must
/// not each own their own idea of it — the gateway cannot build a
/// `PlayerAction` (it does not link the engine) and the engine has never
/// heard of an account.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StandingAnswer {
    /// Registry index of the card the ability is printed on.
    pub card: u32,
    /// Index into that card's ability list (`AbilityRef::index`).
    pub ability: u32,
    /// What to answer without asking.
    pub yes: bool,
}
