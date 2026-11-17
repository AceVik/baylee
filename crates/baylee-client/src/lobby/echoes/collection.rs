//! Permanent, account-owned cosmetics. Only Central supplies earned progress.
use baylee_client_core::echoes::awards_api::{BadgeKind, Progress};

#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Arc, Mutex};

pub(super) struct Collection {
    pub progress: Option<Progress>,
    pub selected: BadgeKind,
    pub failed: bool,
    pub pending: bool,
    #[cfg(not(target_arch = "wasm32"))]
    next_read: f64,
    #[cfg(not(target_arch = "wasm32"))]
    inbox: Arc<Mutex<Option<Result<Progress, baylee_client_core::echoes::Error>>>>,
}

impl Default for Collection {
    fn default() -> Self {
        Self {
            progress: None,
            selected: BadgeKind::Victories,
            failed: false,
            pending: false,
            #[cfg(not(target_arch = "wasm32"))]
            next_read: 0.0,
            #[cfg(not(target_arch = "wasm32"))]
            inbox: Arc::default(),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Collection {
    pub fn refresh(&mut self) {
        self.next_read = 0.0;
    }

    fn receive(&mut self, reply: Result<Progress, baylee_client_core::echoes::Error>) {
        self.pending = false;
        match reply.and_then(|progress| progress.validate().map(|()| progress)) {
            Ok(progress) => {
                self.progress = Some(progress);
                self.failed = false;
            }
            Err(_) => self.failed = true,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn poll(
    time: bevy::prelude::Res<bevy::prelude::Time>,
    mut state: bevy::prelude::ResMut<super::LobbyState>,
) {
    // `native::tick` discarded the departed account's entire Ui before this runs.
    let reply = state
        .echoes
        .collection
        .inbox
        .lock()
        .ok()
        .and_then(|mut inbox| inbox.take());
    if let Some(reply) = reply {
        state.echoes.collection.receive(reply);
    }
    let now = time.elapsed_secs_f64();
    if !state.can_enter_tower()
        || state.central_terms_pending()
        || state.terms_subject == super::super::front::terms::Subject::Central
        || state.hub != super::Hub::Challenges
        || !state.echoes.opened
        || state.echoes.page != super::Page::Collection
        || state.echoes.collection.pending
        || now < state.echoes.collection.next_read
    {
        return;
    }
    state.echoes.collection.pending = true;
    state.echoes.collection.next_read = now + 30.0;
    let inbox = Arc::clone(&state.echoes.collection.inbox);
    if !state.echoes.session.as_ref().is_some_and(|handle| {
        handle.submit(move |account| {
            let reply = account.tower_award_progress();
            if let Ok(mut inbox) = inbox.lock() {
                *inbox = Some(reply);
            }
        })
    }) {
        state
            .echoes
            .collection
            .receive(Err(baylee_client_core::echoes::Error::Busy));
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;
