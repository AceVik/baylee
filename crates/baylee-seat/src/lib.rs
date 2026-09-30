//! The seat bridge: a mind at a table, as an ordinary socket player.
//!
//! The bridge signs in, takes a chair, opens the seat's socket with a ticket
//! like any client, and plays: every question the table asks this seat is
//! either answered by the seat's standing orders or handed to a [`Mind`],
//! whose answer is checked against the question before it is sent. It knows
//! nothing the seat's own view does not show, and the gateway cannot tell it
//! from a person, except by the name it must sit under ([`Disclosure`]).
//!
//! The pieces, from the wire in:
//!
//! - [`link`]: the socket, bought with a ticket and dialled again when it
//!   drops, by the client's own rules ([`baylee_client_core::wsticket`],
//!   [`baylee_client_core::reconnect`]);
//! - [`memory`]: what the seat has been told (the table, the newest view,
//!   the seat's log);
//! - [`wake`]: the standing orders, which answer what is not a decision;
//! - [`referee`]: an answer held to its question before it is sent;
//! - [`seat`]: the rules between a frame and an answer, with no socket and
//!   no clock ([`SeatCore`]);
//! - [`mind`]: what answers the real decisions ([`HouseMind`] and
//!   [`ScriptedMind`] here; a language model and a trained net later);
//! - [`transcript`]: what happened, one JSON line at a time;
//! - [`lobby`] and [`deck`]: signing in, the deck, the room.
//!
//! [`bridge::play`] runs them all against a gateway.

pub mod bridge;
pub mod deck;
pub mod house;
pub mod link;
pub mod llm;
pub mod lobby;
pub mod memory;
pub mod mind;
pub mod narrator;
pub mod referee;
pub mod scripted;
pub mod seat;
pub mod transcript;
pub mod wake;

pub use house::HouseMind;
pub use memory::TableMemory;
pub use mind::{
    Answer, BatchMind, Batched, DeckCard, DeckList, Deliberation, Disclosure, GameContext, Mind,
    MindError, Readiness, Refusal, RefusedBy, Request, Thinking,
};
pub use scripted::ScriptedMind;
pub use seat::{BridgeConfig, By, SeatCore, Stats, Step};
pub use transcript::Transcript;
pub use wake::WakeFilter;
