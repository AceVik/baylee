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
pub use tower::TowerHttp;
#[cfg(feature = "tower-host")]
pub use tower_host::TowerHost;
pub mod chat;
pub mod client_record;
pub mod data;
pub mod reports;
#[cfg(feature = "client")]
pub use account::{Account, PairedSession, PairingClaim};
#[path = "gateway.rs"]
mod admission;
mod http;
pub use admission::{Admission, ChallengeSeat, GatewayHttp, Turned};
pub use central_api::{
    Bearer, Endpoint, Error, SignedToken, Token, challenges, chat as chat_api, gateway, groups,
    owner, social, terms, wire,
};
#[cfg(feature = "client")]
pub use central_client as sdk;
pub use http::CentralHttp;
