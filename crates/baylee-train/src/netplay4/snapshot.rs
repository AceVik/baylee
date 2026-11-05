//! What a player changes while it plays one game, taken whole so a
//! lookahead can play on from a decision and be undone.
use super::{BTreeMap, Backend, CoverageMatrix4, FallbackDiagnostic4, NetPlayer4, ObjectId, Rng};
use crate::features4::SeatKnowledge4;
use baylee_core::ids::PlayerId;

/// Everything per game a [`NetPlayer4`] changes as it observes and answers:
/// recurrent memory, seat knowledge, loop guards, announced sources, the
/// latest observed values, the sampling stream, the last fallback, the
/// coverage counts and the rule references left out. Configuration
/// (depth, temperature, policy mode, model) and the inference backend are
/// not in it.
#[derive(Clone, Debug)]
pub struct PlayerSnapshot4 {
    teams: Vec<Option<u8>>,
    h_states: BTreeMap<u8, Vec<f32>>,
    knowledge: BTreeMap<u8, SeatKnowledge4>,
    seen_hashes: BTreeMap<(u8, u64), u8>,
    last_announced_source: BTreeMap<u8, (ObjectId, Option<u32>)>,
    observed_values: BTreeMap<u8, f32>,
    rng: Rng,
    last_fallback: Option<FallbackDiagnostic4>,
    coverage: CoverageMatrix4,
    rule_refs_left_out: u64,
}

impl NetPlayer4 {
    /// This player's per-game state as it stands.
    ///
    /// Every field is named, none elided: a field added to the player does
    /// not compile until it is taken here or named configuration.
    #[must_use]
    pub fn snapshot(&self) -> PlayerSnapshot4 {
        let Self {
            // Configuration and the backend, not per game.
            backend: _,
            model_dir: _,
            rule_roots: _,
            encoder_version: _,
            model_identity: _,
            d_model: _,
            thinking_steps: _,
            temperature: _,
            policy_mode: _,
            rng,
            teams,
            h_states,
            knowledge,
            seen_hashes,
            last_announced_source,
            observed_values,
            coverage,
            last_fallback,
            rule_refs_left_out,
        } = self;
        PlayerSnapshot4 {
            teams: teams.clone(),
            h_states: h_states.clone(),
            knowledge: knowledge.clone(),
            seen_hashes: seen_hashes.clone(),
            last_announced_source: last_announced_source.clone(),
            observed_values: observed_values.clone(),
            rng: rng.clone(),
            last_fallback: last_fallback.clone(),
            coverage: coverage.clone(),
            rule_refs_left_out: *rule_refs_left_out,
        }
    }

    /// Puts back the per-game state `snapshot` took: whatever was observed,
    /// answered and sampled since is undone. Every field of the snapshot is
    /// named, so one added to it does not compile until it is put back.
    pub fn restore(&mut self, snapshot: &PlayerSnapshot4) {
        let PlayerSnapshot4 {
            teams,
            h_states,
            knowledge,
            seen_hashes,
            last_announced_source,
            observed_values,
            rng,
            last_fallback,
            coverage,
            rule_refs_left_out,
        } = snapshot;
        self.teams.clone_from(teams);
        self.h_states.clone_from(h_states);
        self.knowledge.clone_from(knowledge);
        self.seen_hashes.clone_from(seen_hashes);
        self.last_announced_source.clone_from(last_announced_source);
        self.observed_values.clone_from(observed_values);
        self.rng.clone_from(rng);
        self.last_fallback.clone_from(last_fallback);
        self.coverage.clone_from(coverage);
        self.rule_refs_left_out = *rule_refs_left_out;
    }

    /// The value head's estimate from `seat`'s latest observation that
    /// changed its knowledge (a `K = 0` step of [`Self::observe`]): the
    /// probability that `seat`'s side wins, under the model's critic
    /// semantics. `None` before the seat has observed anything this game.
    #[must_use]
    pub fn observed_value(&self, seat: PlayerId) -> Option<f32> {
        self.observed_values.get(&seat.get()).copied()
    }

    /// A second player on this one's batched session, configured as it is
    /// (temperature, policy mode, thinking steps) and in `snapshot`'s
    /// per-game state: a lookahead's rollout player, whose steps join the
    /// same batches. `None` for a player with sessions of its own, which a
    /// second player could not share.
    #[must_use]
    pub fn twin_on_shared_net(&self, snapshot: &PlayerSnapshot4) -> Option<Self> {
        let Backend::Shared(net) = &self.backend else {
            return None;
        };
        let mut twin = Self::shared(net);
        twin.temperature = self.temperature;
        twin.policy_mode = self.policy_mode;
        twin.thinking_steps = self.thinking_steps;
        twin.restore(snapshot);
        Some(twin)
    }

    /// Whether this player's steps run on a batched session that a twin
    /// can share ([`Self::twin_on_shared_net`]).
    #[must_use]
    pub const fn is_on_shared_net(&self) -> bool {
        matches!(self.backend, Backend::Shared(_))
    }

    /// Restarts only the sampling stream from `seed`, keeping the game:
    /// one rollout's own stream, never the deal's.
    pub fn reseed_sampling(&mut self, seed: u64) {
        self.rng = Rng::new(seed ^ 0x9e37_79b9_7f4a_7c15);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features4::{PendingKind4, SourceKind4};
    use crate::netplay4::FallbackReason4;

    /// A player with no graphs: enough to hold per-game state.
    fn player() -> NetPlayer4 {
        NetPlayer4::without_graphs(7, 4)
    }

    fn fingerprint(net: &NetPlayer4) -> String {
        format!("{:?}", net.snapshot())
    }

    #[test]
    fn restore_undoes_everything_a_game_changes() {
        let deck = crate::housedeck::HouseDeck::named("victory").unwrap();
        let preset = crate::selfplay::table_for(
            5,
            &[&deck, &deck],
            &[baylee_core::preset::AIProfile::STEADY; 2],
            &[],
        );
        let mut net = player();
        net.init_game(&preset);
        let seat = PlayerId::new(0);
        net.observed_values.insert(0, 0.25);
        let before = net.snapshot();
        let printed = fingerprint(&net);

        net.h_states.get_mut(&0).unwrap()[0] = 1.0;
        net.knowledge.get_mut(&0).unwrap().cursor += 7;
        net.seen_hashes.insert((0, 42), 3);
        net.last_announced_source
            .insert(0, (ObjectId::new(9, 0), Some(1)));
        net.observed_values.insert(0, 0.75);
        net.rng.next();
        net.last_fallback = Some(FallbackDiagnostic4::new(
            FallbackReason4::RepeatedView,
            PendingKind4::Priority,
        ));
        net.coverage
            .record(PendingKind4::Priority, SourceKind4::Net);
        net.teams.push(Some(3));
        net.rule_refs_left_out += 2;
        assert_ne!(fingerprint(&net), printed);
        assert_eq!(net.observed_value(seat), Some(0.75));

        net.restore(&before);
        assert_eq!(fingerprint(&net), printed);
        assert_eq!(net.observed_value(seat), Some(0.25));
    }

    #[test]
    fn a_player_with_sessions_of_its_own_lends_no_twin() {
        let net = player();
        assert!(!net.is_on_shared_net());
        assert!(net.twin_on_shared_net(&net.snapshot()).is_none());
    }

    #[test]
    fn reseeding_sampling_keeps_the_game() {
        let mut net = player();
        net.observed_values.insert(1, 0.5);
        net.reseed_sampling(11);
        let first = net.rng.clone().next();
        net.reseed_sampling(11);
        assert_eq!(net.rng.clone().next(), first);
        net.reseed_sampling(12);
        assert_ne!(net.rng.clone().next(), first);
        assert_eq!(net.observed_value(PlayerId::new(1)), Some(0.5));
    }
}
