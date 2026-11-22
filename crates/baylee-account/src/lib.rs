//! Shared native Central transport. Strict TLS even in debug builds; never the game's relaxed TLS policy.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

#[cfg(feature = "client")]
mod account;
#[cfg(feature = "client")]
mod tower;
#[cfg(any(feature = "client", feature = "tower-host"))]
pub use baylee_tower_api as tower_api;
#[cfg(feature = "tower-host")]
mod tower_host;
#[cfg(feature = "client")]
pub use tower::{TowerHttp, TowerSocket};
#[cfg(feature = "tower-host")]
pub use tower_host::{Refused as TowerRefused, TowerHost};
pub mod chat;
pub mod client_record;
pub mod data;
pub mod decks;
pub mod reports;
#[cfg(feature = "client")]
pub use account::TowerPlay;
#[cfg(feature = "client")]
pub use account::{Account, GatewayDeckImport, PairedSession, PairingClaim};
#[cfg(feature = "client")]
pub use account::{RankedHost, RankedPlay, ranked_read};
#[path = "gateway.rs"]
mod admission;
mod http;
#[cfg(feature = "client")]
pub use admission::TowerSeat;
pub use admission::{Admission, ChallengeSeat, GatewayHttp, Turned};
#[cfg(feature = "client")]
pub use admission::{RankedDeck, RankedSeat};
pub use central_api::{
    Bearer, Endpoint, Error, SignedToken, Token, challenges, chat as chat_api, gateway, groups,
    owner, ranked, social, terms, wire,
};
#[cfg(feature = "client")]
pub use central_client as sdk;
pub use http::CentralHttp;
