//! The six tables, one module each.
//!
//! Each is a `SeaORM` entity rather than a hand-written statement, so a
//! column's Postgres type and its Rust type are declared in one place and a
//! disagreement between them is a compile error instead of a row that fails
//! to decode at run time.
//!
//! # What the foreign keys are for
//!
//! Every table here hangs off [`account`], and every one of those references
//! is `ON DELETE CASCADE`. That is the whole deletion story: closing an
//! account is one `DELETE` and the database removes the decks, the sessions,
//! the unspent confirmation links and the settings.
//! The JSON store had to walk six maps by hand to do the same thing, and a
//! map it forgot left a deck owned by nobody.

pub mod account;
pub mod client_settings;
pub mod confirmation;
pub mod deck;
pub mod deck_version;
pub mod session_token;

/// Everything a caller normally wants, under one `use`.
pub mod prelude {
    pub use super::account::Entity as Account;
    pub use super::client_settings::Entity as ClientSettings;
    pub use super::confirmation::Entity as Confirmation;
    pub use super::deck::Entity as Deck;
    pub use super::deck_version::Entity as DeckVersion;
    pub use super::session_token::Entity as SessionToken;
}
