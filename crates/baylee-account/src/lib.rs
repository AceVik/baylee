//! Shared native Central transport. Strict TLS even in debug builds; never the game's relaxed TLS policy.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

#[cfg(feature = "client")]
mod account;
pub mod chat;
pub mod data;
pub mod migration;
pub mod reports;
#[cfg(feature = "client")]
pub use account::Account;
#[path = "gateway.rs"]
mod admission;
mod http;
pub use admission::{Admission, GatewayHttp};
pub use central_api::{Endpoint, Error, Token, gateway, owner, terms, wire};
#[cfg(feature = "client")]
pub use central_client as sdk;
pub use http::CentralHttp;
