//! The builder's doors into a deck's history. The builder's own full-screen
//! history page is retired (`DESIGN` §C.3): its History opens the one sheet
//! the Decks screen draws (`lobby::history`), over the builder.
use super::press::Cx;
#[allow(clippy::wildcard_imports)] // the lobby widget vocabulary
use super::*;
use client_core::lobby::library::Page;

/// A control of the builder's doors into a deck's history.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum LibraryPress {
    /// The builder's History: the history sheet over it.
    BrowseHistory,
}

impl LibraryPress {
    /// What a click on this control does.
    pub(super) fn handle(self, cx: Cx<'_, '_, '_, '_, '_>) {
        let Cx { state, mailbox, .. } = cx;
        match self {
            LibraryPress::BrowseHistory => {
                state.decks.show_all = false;
                state.confirm_restore = false;
                let request = if let Some(Page::History(id)) = state.lobby.library().page.clone() {
                    state.lobby.browse_deck_history(&id)
                } else {
                    state.lobby.browse_history()
                };
                dispatch(state, mailbox, request);
            }
        }
    }
}
