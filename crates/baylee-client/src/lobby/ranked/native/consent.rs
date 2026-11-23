//! The selected host's terms use the same readable, scroll-gated document
//! sheet as the home gateway. All responses belong to the ranked account job.
use super::{LobbyState, ResMut};
pub(super) use crate::lobby::front::terms::localized;
use crate::lobby::{List, Scrolled, front::terms::Subject};
use baylee_client_core::{i18n::Lang, terms::Ask};
use std::time::Instant;

pub(in crate::lobby) fn host_set(state: &LobbyState) -> bool {
    state.ranked_net.documents.is_some()
}

pub(in crate::lobby) fn origin(state: &LobbyState) -> Option<&str> {
    state
        .ranked_net
        .consent
        .as_ref()
        .map(baylee_account::RankedConsent::origin)
}

/// Only opening and closing the actual dialog mark the lobby as changed.
pub(super) fn sync(
    state: &mut ResMut<LobbyState>,
    scroll: Option<&mut ResMut<Scrolled>>,
    closed: bool,
) {
    if state.terms_subject == Subject::Ranked {
        if (state.ranked_net.consent.is_none() && !host_set(state))
            || !state.central_ready()
            || state.central_terms_pending()
        {
            state.terms.close();
            state.terms_subject = Subject::Gateway;
        } else if let Some(documents) = &state.ranked_net.documents
            && state
                .ranked_net
                .displayed_documents
                .as_ref()
                .is_none_or(|shown| !std::sync::Arc::ptr_eq(documents, shown))
        {
            let lang = state.terms.lang();
            state.terms.ask(lang);
            if let Some(scroll) = scroll {
                scroll.set(List::Terms, 0.0);
            }
            load(state, lang);
        }
        return;
    }
    if !closed
        || !super::ready(state)
        || state.terms_stale == Some(true)
        || (state.ranked_net.consent.is_none() && !host_set(state))
    {
        return;
    }
    let lang = state.lobby.lang();
    state.terms_subject = Subject::Ranked;
    state.terms.ask(lang);
    if let Some(scroll) = scroll {
        scroll.set(List::Terms, 0.0);
    }
    load(state, lang);
}

fn load(state: &mut LobbyState, lang: Lang) {
    state.ranked_net.displayed_documents = state.ranked_net.documents.clone();
    let doc = if let Some(documents) = &state.ranked_net.documents {
        super::host_documents::sheet(documents, lang)
    } else {
        state
            .ranked_net
            .consent
            .as_ref()
            .and_then(|c| localized(c.document(), lang))
    };
    if let Some(doc) = doc {
        state.terms.loaded(doc, lang);
    } else {
        state.terms.failed();
    }
}

pub(in crate::lobby) fn perform(ask: Ask, state: &mut LobbyState, scroll: &mut Scrolled) {
    match ask {
        Ask::Fetch(lang) => {
            scroll.set(List::Terms, 0.0);
            load(state, lang);
        }
        Ask::Accept(version) => {
            let worker = &mut state.ranked_net;
            let shown = state.terms.shown();
            if let Some(documents) = &worker.documents {
                if super::host_documents::sheet(documents, state.terms.lang())
                    .as_ref()
                    .is_some_and(|doc| Some(doc) == shown && doc.version == version)
                {
                    worker.host_choice = Some(true);
                    worker.next = Some(Instant::now());
                } else {
                    state.terms.send_failed();
                }
            } else if worker.consent.as_ref().is_some_and(|c| {
                c.matches(&worker.queue)
                    && c.document().version.version == version
                    && localized(c.document(), state.terms.lang())
                        .as_ref()
                        .is_some_and(|d| Some(d) == shown)
            }) {
                worker.accepting = true;
                worker.next = Some(Instant::now());
            } else {
                state.terms.send_failed();
            }
        }
        Ask::SignOut => {
            if host_set(state) {
                state.ranked_net.host_choice = Some(false);
                state.ranked_net.next = Some(Instant::now());
                return;
            }
            super::cancel(state);
            state.terms.close();
            state.terms_subject = Subject::Gateway;
        }
    }
}
