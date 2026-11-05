//! Carry out the social machine's effects through the host services.

#[allow(clippy::wildcard_imports)] // the host's own vocabulary
use super::*;

impl SocialHost {
    /// Carries out the machine's effects.
    pub(crate) fn carry_out(&mut self, effects: Vec<Effect>) {
        #[cfg(not(target_arch = "wasm32"))]
        let snapshot = self.vault.snapshot_owner();
        #[cfg(not(target_arch = "wasm32"))]
        let _batch = snapshot.as_ref().and_then(|sync| sync.begin_batch());
        for effect in effects {
            self.carry_out_one(effect);
        }
    }

    fn carry_out_one(&mut self, effect: Effect) {
        let Some(session) = self.session.clone() else {
            return;
        };
        match effect {
            Effect::Http {
                tag,
                method,
                path,
                body,
            } => self.request_http(&session, tag, method, &path, body.as_ref()),
            Effect::Up(up) => self.send_up(up),
            Effect::Dial => {
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(bridge) = self.central.as_mut() {
                    bridge.resume();
                    if bridge.game_registered() {
                        self.socket.want();
                    }
                    return;
                }
                self.socket.want();
            }
            Effect::Hang => {
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(bridge) = self.central.as_mut() {
                    bridge.pause();
                }
                self.socket.hang();
            }
            Effect::StoreSecrets(value) => {
                if let (Some(store), Some(machine)) = (self.store.as_ref(), self.machine.as_ref())
                    && let Err(why) = store.store(&machine.secret_slot(), value.expose())
                {
                    warn!("the chat's keys could not be kept: {why}");
                }
            }
            Effect::ForgetSecrets => {
                if let (Some(store), Some(machine)) = (self.store.take(), self.machine.as_ref()) {
                    let slot = machine.secret_slot();
                    #[cfg(not(target_arch = "wasm32"))]
                    std::thread::spawn(move || {
                        let _ = store.forget(&slot);
                    });
                    #[cfg(target_arch = "wasm32")]
                    let _ = store.forget(&slot);
                }
            }
            Effect::SaveState(bytes) => {
                #[cfg(not(target_arch = "wasm32"))]
                if self.vault.is_central() {
                    // Central's shutdown drain must see the latest conversation map even
                    // when macOS quits by dropping the World without an AppExit message.
                    if let Some(disk) = self.disk.as_mut() {
                        if disk.write(social::disk::STATE_FILE, &bytes).is_err() {
                            self.vault.error = Some(vault_access::Error::Io);
                        }
                        self.save = None;
                    }
                    return;
                }
                self.save = Some((SAVE_DEBOUNCE, bytes));
            }
            effect @ (Effect::AppendHistory { .. }
            | Effect::RewriteHistory { .. }
            | Effect::DeleteAllLocal) => {
                if matches!(effect, Effect::DeleteAllLocal) {
                    self.save = None;
                }
                if let Some(disk) = self.disk.as_mut()
                    && let Some(Err(why)) = social::disk::apply(&effect, &mut **disk)
                {
                    warn!("the chat's history could not be written: {why}");
                }
            }
            Effect::Toast(toast) => {
                self.toasts.push_back(toast);
                while self.toasts.len() > 6 {
                    self.toasts.pop_front();
                }
            }
            Effect::Withdraw(about) => {
                self.toasts
                    .retain(|toast| toast.about.as_ref() != Some(&about));
                self.withdrawn.push(about);
            }
            Effect::Sound(cue) => self.sounds.push(cue),
            Effect::OpenConversation(conv) => self.asks.push(UiAsk::Open(conv)),
            Effect::JoinTable { game, invite } => self.asks.push(UiAsk::Join { game, invite }),
            Effect::WatchTable { game } => self.asks.push(UiAsk::Watch { game }),
            Effect::OpenDrawer => self.asks.push(UiAsk::Drawer),
            Effect::Copy(text) => self.copy = Some(text),
        }
    }

    fn request_http(
        &mut self,
        session: &Session,
        tag: social::HttpTag,
        method: social::Method,
        path: &str,
        body: Option<&serde_json::Value>,
    ) {
        #[cfg(not(target_arch = "wasm32"))]
        if central::request(self, &tag, method, path, body) {
            return;
        }
        transport::request(
            &self.mailbox,
            &session.gateway,
            &session.token,
            tag,
            method,
            path,
            body,
        );
    }

    fn send_up(&mut self, up: baylee_protocol::social::Up) {
        #[cfg(not(target_arch = "wasm32"))]
        if central::up(self, &up) {
            return;
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let baylee_protocol::social::Up::Ack { upto } = &up
            && self.vault.defer_ack(*upto)
        {
            return;
        }
        if let Ok(text) = serde_json::to_string(&up)
            && !self.socket.send(text)
            && let Some(machine) = self.machine.as_mut()
        {
            // No socket: the machine keeps the send pending and
            // goes again once the link is back.
            #[cfg(not(target_arch = "wasm32"))]
            if self.central.is_some() {
                return;
            }
            let _ = machine.on_link(SocialLink::Down, now_ms());
        }
    }
}
