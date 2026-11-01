//! What a player changes while it plays one game, taken whole so a
//! lookahead can play on from a decision and be undone.
use super::{BTreeMap, CoverageMatrix4, FallbackDiagnostic4, NetPlayer4, ObjectId, Rng};
use crate::features4::SeatKnowledge4;
use baylee_core::ids::PlayerId;

/// Everything per game a [`NetPlayer4`] changes as it observes and answers:
/// recurrent memory, seat knowledge, loop guards, announced sources, the
/// latest observed values, the sampling stream, the last fallback and the
/// coverage counts. Configuration (depth, temperature, policy mode, model)
/// and the inference backend are not in it.
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
}

impl NetPlayer4 {
    /// This player's per-game state as it stands.
    #[must_use]
    pub fn snapshot(&self) -> PlayerSnapshot4 {
        PlayerSnapshot4 {
            teams: self.teams.clone(),
            h_states: self.h_states.clone(),
            knowledge: self.knowledge.clone(),
            seen_hashes: self.seen_hashes.clone(),
            last_announced_source: self.last_announced_source.clone(),
            observed_values: self.observed_values.clone(),
            rng: self.rng.clone(),
            last_fallback: self.last_fallback.clone(),
            coverage: self.coverage.clone(),
        }
    }

    /// Puts back the per-game state `snapshot` took: whatever was observed,
    /// answered and sampled since is undone.
    pub fn restore(&mut self, snapshot: &PlayerSnapshot4) {
        self.teams.clone_from(&snapshot.teams);
        self.h_states.clone_from(&snapshot.h_states);
        self.knowledge.clone_from(&snapshot.knowledge);
        self.seen_hashes.clone_from(&snapshot.seen_hashes);
        self.last_announced_source
            .clone_from(&snapshot.last_announced_source);
        self.observed_values.clone_from(&snapshot.observed_values);
        self.rng.clone_from(&snapshot.rng);
        self.last_fallback.clone_from(&snapshot.last_fallback);
        self.coverage.clone_from(&snapshot.coverage);
    }

    /// The value head's estimate from `seat`'s latest observation that
    /// changed its knowledge (a `K = 0` step of [`Self::observe`]): the
    /// probability that `seat`'s side wins, under the model's critic
    /// semantics. `None` before the seat has observed anything this game.
    #[must_use]
    pub fn observed_value(&self, seat: PlayerId) -> Option<f32> {
        self.observed_values.get(&seat.get()).copied()
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
        assert_ne!(fingerprint(&net), printed);
        assert_eq!(net.observed_value(seat), Some(0.75));

        net.restore(&before);
        assert_eq!(fingerprint(&net), printed);
        assert_eq!(net.observed_value(seat), Some(0.25));
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
