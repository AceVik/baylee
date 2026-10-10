//! The gateway's HTTP and socket routes, one concern to a file. `main.rs`
//! builds the router from them; everything they share (the state, the
//! answers, the session check) stays there.

mod catalog;
mod deck_lines;
mod decks;
mod info;
mod listing;
mod lobby_feed;
mod rematch;
mod rooms;
mod seat_socket;
mod settings;
mod signin;
mod stats;
mod table_view;
mod tables;
mod terms;
mod tickets;
mod watch;

pub(crate) use catalog::*;
pub(crate) use deck_lines::*;
pub(crate) use decks::*;
pub(crate) use info::*;
pub(crate) use listing::*;
pub(crate) use lobby_feed::*;
pub(crate) use rematch::*;
pub(crate) use rooms::*;
pub(crate) use seat_socket::*;
pub(crate) use settings::*;
pub(crate) use signin::*;
pub(crate) use stats::*;
pub(crate) use table_view::*;
pub(crate) use tables::*;
pub(crate) use tickets::*;
pub(crate) use watch::*;
