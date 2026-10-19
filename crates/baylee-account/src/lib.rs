//! Shared native Central transport. Strict TLS even in debug builds; never the game's relaxed TLS policy.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

#[cfg(feature = "client")]
mod account;
pub mod data;
pub mod migration;
#[cfg(feature = "client")]
pub use account::Account;
mod http;
pub use central_api::{Endpoint, Error, Token, wire};
#[cfg(feature = "client")]
pub use central_client as sdk;
pub use http::CentralHttp;
