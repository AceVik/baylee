//! Separate account and gateway trust, including in-flight fetches and confirmations.
use super::{Conv, Down, Effect, Entropy, HttpTag, Intent, KeyCache, Social};

pub(super) fn game(conv: &Conv) -> bool {
    matches!(conv, Conv::Table { .. } | Conv::Team { .. } | Conv::Global)
}
impl Social {
    /// Composer commands use the same trust context as their open conversation.
    pub fn key(
        &mut self,
        key: super::ComposerKey,
        now_ms: u64,
        rng: &mut dyn Entropy,
    ) -> (super::ComposerOutcome, Vec<Effect>) {
        self.select_key_scope(self.active.as_ref().is_some_and(game));
        let out = self.key_in_scope(key, now_ms, rng);
        self.select_key_scope(false);
        out
    }

    pub(super) fn select_key_scope(&mut self, game: bool) {
        if !self.account_transport || self.game_keys_active == game {
            return;
        }
        let other = self.state.game_keys.get_or_insert_with(KeyCache::default);
        std::mem::swap(&mut self.state.keys, other);
        std::mem::swap(
            &mut self.state.own_devices,
            &mut self.state.game_own_devices,
        );
        std::mem::swap(&mut self.fetching, &mut self.game_fetching);
        std::mem::swap(&mut self.refetched, &mut self.game_refetched);
        std::mem::swap(&mut self.stale, &mut self.game_stale);
        self.game_keys_active = game;
    }
    pub(super) fn display_keys(&self) -> &KeyCache {
        self.keys_for_scope(self.active.as_ref().is_some_and(game))
    }
    pub(super) fn keys_for_scope(&self, game: bool) -> &KeyCache {
        let game = self.account_transport && game;
        if game == self.game_keys_active {
            &self.state.keys
        } else {
            self.state.game_keys.as_ref().unwrap_or(&self.state.keys)
        }
    }
    /// A directory answer updates only the service it was requested from.
    pub fn on_http(
        &mut self,
        tag: HttpTag,
        status: u16,
        body: &[u8],
        now_ms: u64,
        rng: &mut dyn Entropy,
    ) -> Vec<Effect> {
        self.select_key_scope(matches!(tag, HttpTag::GameKeys(_)));
        let out = self.on_http_in_scope(tag, status, body, now_ms, rng);
        self.select_key_scope(false);
        out
    }
    /// Gateway deliveries use gateway pins; Central sends' receipts retain their
    /// own namespace. A transport must authenticate which service supplied a frame.
    pub fn on_frame(&mut self, frame: Down, now_ms: u64, rng: &mut dyn Entropy) -> Vec<Effect> {
        let is_game = match &frame {
            Down::Msg { .. } | Down::KeysChanged { .. } => true,
            Down::Sent { req, .. } | Down::Refused { req: Some(req), .. } => self
                .outbox
                .iter()
                .find(|pending| pending.req == *req)
                .is_some_and(|pending| game(&pending.key.conv)),
            _ => false,
        };
        self.select_key_scope(is_game);
        let out = self.on_frame_in_scope(frame, now_ms, rng);
        self.select_key_scope(false);
        out
    }
    /// Verification applies to the open conversation's exact displayed key set.
    pub fn act(&mut self, intent: Intent, now_ms: u64, rng: &mut dyn Entropy) -> Vec<Effect> {
        let is_game = match &intent {
            Intent::OpenConv(conv) | Intent::Panel(Some(conv)) => game(conv),
            Intent::Whisper { .. } => false,
            _ => self.active.as_ref().is_some_and(game),
        };
        self.select_key_scope(is_game);
        let out = self.act_in_scope(intent, now_ms, rng);
        self.select_key_scope(false);
        out
    }
}

#[cfg(test)]
mod tests;
