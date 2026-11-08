//! What a player changes while it plays one game, taken whole so a
//! lookahead can play on from a decision and be undone.
use super::{
    BTreeMap, Backend, ChoiceGuard4, CoverageMatrix4, FallbackDiagnostic4, NetPlayer4, ObjectId,
    RECURRENT_SLOTS, Rng, TacticRules4,
};
use crate::features4::{MAX_SEATS, SeatKnowledge4};
use anyhow::{Context as _, bail, ensure};
use baylee_core::ids::PlayerId;
use serde::{Deserialize, Serialize};

const MAX_SEEN_HASHES_WIRE: usize = 16_384;

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
    choice_guard: ChoiceGuard4,
    tactic_rules: TacticRules4,
}

#[derive(Serialize, Deserialize)]
struct SnapshotWire4 {
    teams: Vec<Option<u8>>,
    h_states: BTreeMap<u8, Vec<f32>>,
    knowledge: BTreeMap<u8, SeatKnowledge4>,
    seen_hashes: Vec<((u8, u64), u8)>,
    last_announced_source: BTreeMap<u8, (ObjectId, Option<u32>)>,
    observed_values: BTreeMap<u8, f32>,
    rng_state: u64,
    last_fallback: Option<FallbackDiagnostic4>,
    coverage: CoverageMatrix4,
    rule_refs_left_out: u64,
    choice_guard: ChoiceGuard4,
    tactic_rules: TacticRules4,
}

impl PlayerSnapshot4 {
    /// Approximate heap + struct memory footprint in RAM (bytes).
    #[must_use]
    pub fn approx_ram_bytes(&self) -> usize {
        let h_bytes: usize = self
            .h_states
            .values()
            .map(|v| v.len() * std::mem::size_of::<f32>())
            .sum();
        let kn_bytes: usize = self
            .knowledge
            .values()
            .map(|k| {
                std::mem::size_of::<SeatKnowledge4>()
                    + k.own_deck_cards.len() * 8
                    + k.known_cards.len() * 32
            })
            .sum();
        let hash_bytes = self.seen_hashes.len() * 24;
        std::mem::size_of::<Self>() + h_bytes + kn_bytes + hash_bytes
    }

    /// Validates all tensor dimensions, seat bounds, and finite floating-point
    /// invariants against `d_model`, then serializes to canonical JSON bytes.
    ///
    /// # Errors
    /// Returns an error if any dimension or floating-point value is invalid.
    pub fn to_validated_wire(&self, d_model: usize) -> anyhow::Result<Vec<u8>> {
        self.validate(d_model)?;
        let wire = SnapshotWire4 {
            teams: self.teams.clone(),
            h_states: self.h_states.clone(),
            knowledge: self.knowledge.clone(),
            seen_hashes: self.seen_hashes.iter().map(|(&k, &v)| (k, v)).collect(),
            last_announced_source: self.last_announced_source.clone(),
            observed_values: self.observed_values.clone(),
            rng_state: self.rng.state(),
            last_fallback: self.last_fallback.clone(),
            coverage: self.coverage.clone(),
            rule_refs_left_out: self.rule_refs_left_out,
            choice_guard: self.choice_guard,
            tactic_rules: self.tactic_rules,
        };
        serde_json::to_vec(&wire).context("serializing PlayerSnapshot4 wire payload")
    }

    /// Deserializes and strictly validates a [`PlayerSnapshot4`] against `d_model`.
    ///
    /// # Errors
    /// Rejects malformed JSON, out-of-range seats, mismatched `h_state` lengths,
    /// `NaN`/`Inf` floats, or excessive map entries.
    pub fn from_validated_wire(bytes: &[u8], d_model: usize) -> anyhow::Result<Self> {
        let wire: SnapshotWire4 =
            serde_json::from_slice(bytes).context("deserializing PlayerSnapshot4 wire payload")?;
        ensure!(
            wire.seen_hashes.len() <= MAX_SEEN_HASHES_WIRE,
            "snapshot seen_hashes count {} exceeds limit {MAX_SEEN_HASHES_WIRE}",
            wire.seen_hashes.len()
        );
        let snap = Self {
            teams: wire.teams,
            h_states: wire.h_states,
            knowledge: wire.knowledge,
            seen_hashes: wire.seen_hashes.into_iter().collect(),
            last_announced_source: wire.last_announced_source,
            observed_values: wire.observed_values,
            rng: Rng::new(wire.rng_state),
            last_fallback: wire.last_fallback,
            coverage: wire.coverage,
            rule_refs_left_out: wire.rule_refs_left_out,
            choice_guard: wire.choice_guard,
            tactic_rules: wire.tactic_rules,
        };
        snap.validate(d_model)?;
        Ok(snap)
    }

    /// Strictly checks seat bounds, `h_state` dimensions (`RECURRENT_SLOTS * d_model`),
    /// and finite float invariants.
    ///
    /// # Errors
    /// When any invariant is violated.
    pub fn validate(&self, d_model: usize) -> anyhow::Result<()> {
        ensure!(d_model > 0 && d_model <= 8192, "invalid d_model {d_model}");
        ensure!(
            self.teams.len() <= MAX_SEATS,
            "snapshot teams length {} exceeds MAX_SEATS ({MAX_SEATS})",
            self.teams.len()
        );
        let expected_h_len = RECURRENT_SLOTS * d_model;
        for (&seat, h) in &self.h_states {
            ensure!(
                usize::from(seat) < MAX_SEATS,
                "snapshot h_state seat {seat} out of bounds"
            );
            ensure!(
                h.len() == expected_h_len,
                "snapshot h_state length {} for seat {seat} != expected {expected_h_len}",
                h.len()
            );
            if h.iter().any(|v| !v.is_finite()) {
                bail!("snapshot h_state for seat {seat} contains non-finite float");
            }
        }
        for (&seat, kn) in &self.knowledge {
            ensure!(
                usize::from(seat) < MAX_SEATS,
                "snapshot knowledge seat {seat} out of bounds"
            );
            ensure!(
                kn.viewer.get() == seat,
                "snapshot knowledge viewer {} != key seat {seat}",
                kn.viewer.get()
            );
            ensure!(
                (1..=MAX_SEATS).contains(&kn.seat_count),
                "snapshot knowledge seat_count {} out of bounds",
                kn.seat_count
            );
            if kn.own_deck_summary.iter().any(|v| !v.is_finite()) {
                bail!("snapshot own_deck_summary for seat {seat} contains non-finite float");
            }
        }
        for (&seat, &val) in &self.observed_values {
            ensure!(
                usize::from(seat) < MAX_SEATS,
                "snapshot observed_values seat {seat} out of bounds"
            );
            ensure!(
                val.is_finite(),
                "snapshot observed_value for seat {seat} is non-finite"
            );
        }
        for &(seat, _) in self.seen_hashes.keys() {
            ensure!(
                usize::from(seat) < MAX_SEATS,
                "snapshot seen_hashes seat {seat} out of bounds"
            );
        }
        for &seat in self.last_announced_source.keys() {
            ensure!(
                usize::from(seat) < MAX_SEATS,
                "snapshot last_announced_source seat {seat} out of bounds"
            );
        }
        Ok(())
    }
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
            choice_guard,
            payable_menu: _,
            tactic_rules,
            last_menu: _,
            tactician: _,
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
            choice_guard: *choice_guard,
            tactic_rules: *tactic_rules,
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
            choice_guard,
            tactic_rules,
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
        self.choice_guard = *choice_guard;
        self.tactic_rules = *tactic_rules;
    }

    /// Resets every seat other than `searching_seat` to empty knowledge and
    /// zero recurrent memory for a simulated rollout world, so a neural
    /// opponent in lookahead reads only its view in the simulated world and
    /// never the real game's submitted deck list or history.
    pub fn reset_other_seats_for_rollout(&mut self, searching_seat: PlayerId) {
        let me = searching_seat.get();
        let seat_count = self.teams.len().clamp(1, MAX_SEATS);
        for (&seat, h) in &mut self.h_states {
            if seat != me {
                h.fill(0.0);
            }
        }
        for (&seat, kn) in &mut self.knowledge {
            if seat != me {
                *kn = SeatKnowledge4::empty(PlayerId::new(seat), seat_count);
            }
        }
        self.seen_hashes.retain(|&(seat, _), _| seat == me);
        self.last_announced_source.retain(|&seat, _| seat == me);
        self.observed_values.retain(|&seat, _| seat == me);
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
