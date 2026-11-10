//! What a player changes while it plays one game, taken whole so a
//! lookahead can play on from a decision and be undone.
use super::{
    BTreeMap, Backend, ChoiceGuard4, CoverageMatrix4, FallbackDiagnostic4, NetPlayer4, ObjectId,
    RECURRENT_SLOTS, Rng, TacticRules4,
};
use crate::features4::{MAX_SEATS, SeatKnowledge4};
use anyhow::{Context as _, bail, ensure};
use baylee_core::ids::PlayerId;
use baylee_driver::fp16::{
    extend_u16_le_bytes, f16_to_f32, f16_to_f32_checked, f32_to_f16, pack_f32_slice_to_f16,
    unpack_f16_slice_to_f32, unpack_u16_le_bytes_into,
};
use serde::{Deserialize, Serialize};
use zeroize::Zeroize as _;

const MAX_SEEN_HASHES_WIRE: usize = 16_384;
const WIRE_MAGIC_V3: &[u8; 6] = b"BMUX4\x03";
const WIRE_MAGIC_COLD_V4: &[u8; 6] = b"BMUX4\x04";

/// Compact non-tensor session metadata retained across cold tensor swaps without JSON overhead.
#[derive(Clone, Debug)]
pub struct ColdMeta4 {
    teams: Vec<Option<u8>>,
    knowledge: BTreeMap<u8, SeatKnowledge4>,
    last_announced_source: BTreeMap<u8, (ObjectId, Option<u32>)>,
    last_fallback: Option<FallbackDiagnostic4>,
    coverage: CoverageMatrix4,
    rule_refs_left_out: u64,
    choice_guard: ChoiceGuard4,
    tactic_rules: TacticRules4,
}

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
    /// Compact in-RAM IEEE-754 `FP16` recurrent memory (`9 * d_model` `u16` words per seat)
    /// used by `npu_mux4` to cut Hot RAM footprint in half (`23 KB` vs `46 KB` per 2-seat game)
    /// and page to/from SSD with zero float conversion overhead.
    h_states_fp16: BTreeMap<u8, Vec<u16>>,
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
struct SnapshotMetaWire4 {
    teams: Vec<Option<u8>>,
    knowledge: BTreeMap<u8, SeatKnowledge4>,
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
    /// Wipes sensitive in-RAM latent state buffers (`h_states`, `h_states_fp16`, `observed_values`, `seen_hashes`)
    /// before a snapshot is dropped after SSD eviction or session closure.
    pub fn zeroize_sensitive(&mut self) {
        for h in self.h_states.values_mut() {
            h.zeroize();
        }
        for h in self.h_states_fp16.values_mut() {
            h.zeroize();
        }
        for v in self.observed_values.values_mut() {
            v.zeroize();
        }
        for c in self.seen_hashes.values_mut() {
            c.zeroize();
        }
        self.seen_hashes.clear();
    }

    /// Quantizes `h_states` (`f32`) into `h_states_fp16` (`u16` IEEE-754 `FP16`) and releases
    /// the `f32` heap vectors, cutting the in-RAM recurrent tensor footprint by 50%.
    pub fn pack_fp16_in_ram(&mut self) {
        for (seat, h32) in std::mem::take(&mut self.h_states) {
            let dst = self.h_states_fp16.entry(seat).or_default();
            pack_f32_slice_to_f16(&h32, dst);
        }
    }

    /// Reads the latest observed value for `seat` directly from the snapshot without a worker checkout.
    #[must_use]
    pub fn observed_value(&self, seat: PlayerId) -> Option<f32> {
        self.observed_values.get(&seat.get()).copied()
    }

    /// Approximate heap + struct memory footprint in RAM (bytes).
    #[must_use]
    pub fn approx_ram_bytes(&self) -> usize {
        let h_f32_bytes: usize = self
            .h_states
            .values()
            .map(|v| v.len() * std::mem::size_of::<f32>())
            .sum();
        let h_f16_bytes: usize = self
            .h_states_fp16
            .values()
            .map(|v| v.len() * std::mem::size_of::<u16>())
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
        std::mem::size_of::<Self>() + h_f32_bytes + h_f16_bytes + kn_bytes + hash_bytes
    }

    /// Ultra-fast pure-binary cold serialization (`BMUX4\x04`) for SSD paging into a caller-owned
    /// buffer `out` (zero heap allocation when `out` is a thread-local scratch buffer):
    /// serializes `h_states_fp16` (`2 bytes/elem`), `seen_hashes`, `observed_values`, and `rng_state`
    /// in raw binary with 64-bit word packing, while moving non-tensor metadata out into [`ColdMeta4`]
    /// in $O(1)$ without cloning.
    ///
    /// # Errors
    /// Returns an error if any dimension or floating-point invariant is violated.
    #[allow(clippy::cast_possible_truncation)]
    pub fn to_cold_wire_into(
        &mut self,
        d_model: usize,
        out: &mut Vec<u8>,
    ) -> anyhow::Result<ColdMeta4> {
        if self.h_states_fp16.is_empty() && !self.h_states.is_empty() {
            self.pack_fp16_in_ram();
        }
        self.validate(d_model)?;
        let expected_h_len = RECURRENT_SLOTS * d_model;
        let h_bytes_total = self.h_states_fp16.len() * (1 + expected_h_len * 2);
        let hash_bytes_total = self.seen_hashes.len() * 10;
        let obs_bytes_total = self.observed_values.len() * 5;
        out.clear();
        out.reserve(24 + h_bytes_total + hash_bytes_total + obs_bytes_total);
        out.extend_from_slice(WIRE_MAGIC_COLD_V4);
        out.extend_from_slice(&(d_model as u32).to_le_bytes());
        out.push(self.h_states_fp16.len() as u8);
        for (&seat, h16) in &self.h_states_fp16 {
            out.push(seat);
            extend_u16_le_bytes(h16, out);
        }
        out.extend_from_slice(&(self.seen_hashes.len() as u32).to_le_bytes());
        for (&(seat, hash), &count) in &self.seen_hashes {
            out.push(seat);
            out.extend_from_slice(&hash.to_le_bytes());
            out.push(count);
        }
        out.push(self.observed_values.len() as u8);
        for (&seat, &val) in &self.observed_values {
            out.push(seat);
            out.extend_from_slice(&val.to_le_bytes());
        }
        out.extend_from_slice(&self.rng.state().to_le_bytes());

        Ok(ColdMeta4 {
            teams: std::mem::take(&mut self.teams),
            knowledge: std::mem::take(&mut self.knowledge),
            last_announced_source: std::mem::take(&mut self.last_announced_source),
            last_fallback: self.last_fallback.take(),
            coverage: std::mem::take(&mut self.coverage),
            rule_refs_left_out: self.rule_refs_left_out,
            choice_guard: self.choice_guard,
            tactic_rules: self.tactic_rules,
        })
    }

    /// Allocating wrapper around [`Self::to_cold_wire_into`].
    ///
    /// # Errors
    /// Returns an error if any dimension or floating-point invariant is violated.
    pub fn to_cold_wire_and_meta(
        &mut self,
        d_model: usize,
    ) -> anyhow::Result<(Vec<u8>, ColdMeta4)> {
        let mut out = Vec::new();
        let meta = self.to_cold_wire_into(d_model, &mut out)?;
        Ok((out, meta))
    }

    /// Reconstructs a hot [`PlayerSnapshot4`] (keeping `h_states_fp16` in compact `FP16` RAM form)
    /// from its `BMUX4\x04` binary cold tensor payload and [`ColdMeta4`].
    ///
    /// # Errors
    /// Rejects malformed headers, out-of-range seats, or non-finite `FP16`/`f32` values.
    #[allow(clippy::too_many_lines)]
    pub fn from_cold_wire_and_meta(
        bytes: &[u8],
        d_model: usize,
        meta: ColdMeta4,
    ) -> anyhow::Result<Self> {
        ensure!(d_model > 0 && d_model <= 8192, "invalid d_model {d_model}");
        ensure!(
            bytes.starts_with(WIRE_MAGIC_COLD_V4),
            "invalid or unsupported PlayerSnapshot4 cold wire magic"
        );
        ensure!(
            bytes.len() >= 15,
            "truncated PlayerSnapshot4 cold wire header"
        );
        let wire_d_model = u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]) as usize;
        ensure!(
            wire_d_model == d_model,
            "snapshot wire d_model ({wire_d_model}) != expected ({d_model})"
        );
        let h_count = usize::from(bytes[10]);
        ensure!(
            h_count <= MAX_SEATS,
            "snapshot wire h_count ({h_count}) exceeds MAX_SEATS ({MAX_SEATS})"
        );
        let expected_h_len = RECURRENT_SLOTS * d_model;
        let per_seat_bytes = 1 + expected_h_len * 2;
        let mut cursor = 11;
        ensure!(
            bytes.len() >= cursor + h_count * per_seat_bytes + 4,
            "truncated PlayerSnapshot4 cold h_states_fp16 section"
        );
        let mut h_states_fp16 = BTreeMap::new();
        for _ in 0..h_count {
            let seat = bytes[cursor];
            ensure!(
                usize::from(seat) < MAX_SEATS,
                "snapshot seat {seat} out of bounds"
            );
            cursor += 1;
            let mut vec = Vec::with_capacity(expected_h_len);
            let slice_end = cursor + expected_h_len * 2;
            unpack_u16_le_bytes_into(&bytes[cursor..slice_end], &mut vec)?;
            cursor = slice_end;
            if h_states_fp16.insert(seat, vec).is_some() {
                bail!("duplicate seat {seat} in PlayerSnapshot4 h_states_fp16");
            }
        }
        let hash_count = u32::from_le_bytes([
            bytes[cursor],
            bytes[cursor + 1],
            bytes[cursor + 2],
            bytes[cursor + 3],
        ]) as usize;
        cursor += 4;
        ensure!(
            hash_count <= MAX_SEEN_HASHES_WIRE,
            "snapshot seen_hashes count {hash_count} exceeds limit {MAX_SEEN_HASHES_WIRE}"
        );
        ensure!(
            bytes.len() > cursor + hash_count * 10,
            "truncated PlayerSnapshot4 cold seen_hashes section"
        );
        let mut seen_hashes = BTreeMap::new();
        for _ in 0..hash_count {
            let seat = bytes[cursor];
            ensure!(
                usize::from(seat) < MAX_SEATS,
                "snapshot hash seat {seat} out of bounds"
            );
            let hash = u64::from_le_bytes([
                bytes[cursor + 1],
                bytes[cursor + 2],
                bytes[cursor + 3],
                bytes[cursor + 4],
                bytes[cursor + 5],
                bytes[cursor + 6],
                bytes[cursor + 7],
                bytes[cursor + 8],
            ]);
            let count = bytes[cursor + 9];
            cursor += 10;
            seen_hashes.insert((seat, hash), count);
        }
        let obs_count = usize::from(bytes[cursor]);
        cursor += 1;
        ensure!(
            obs_count <= MAX_SEATS && bytes.len() == cursor + obs_count * 5 + 8,
            "invalid or truncated PlayerSnapshot4 cold tail"
        );
        let mut observed_values = BTreeMap::new();
        for _ in 0..obs_count {
            let seat = bytes[cursor];
            let val = f32::from_le_bytes([
                bytes[cursor + 1],
                bytes[cursor + 2],
                bytes[cursor + 3],
                bytes[cursor + 4],
            ]);
            ensure!(
                usize::from(seat) < MAX_SEATS && val.is_finite(),
                "invalid observed_value"
            );
            cursor += 5;
            observed_values.insert(seat, val);
        }
        let rng_state = u64::from_le_bytes([
            bytes[cursor],
            bytes[cursor + 1],
            bytes[cursor + 2],
            bytes[cursor + 3],
            bytes[cursor + 4],
            bytes[cursor + 5],
            bytes[cursor + 6],
            bytes[cursor + 7],
        ]);
        Ok(Self {
            teams: meta.teams,
            h_states: BTreeMap::new(),
            h_states_fp16,
            knowledge: meta.knowledge,
            seen_hashes,
            last_announced_source: meta.last_announced_source,
            observed_values,
            rng: Rng::new(rng_state),
            last_fallback: meta.last_fallback,
            coverage: meta.coverage,
            rule_refs_left_out: meta.rule_refs_left_out,
            choice_guard: meta.choice_guard,
            tactic_rules: meta.tactic_rules,
        })
    }

    /// Validates all tensor dimensions, seat bounds, and finite floating-point
    /// invariants against `d_model`, then serializes to the compact `BMUX4\x03`
    /// binary + metadata wire format (writing `h_states` in `FP16` and `seen_hashes` as
    /// raw little-endian binary records).
    ///
    /// # Errors
    /// Returns an error if any dimension or floating-point value is invalid.
    #[allow(clippy::cast_possible_truncation)]
    pub fn to_validated_wire(&self, d_model: usize) -> anyhow::Result<Vec<u8>> {
        self.validate(d_model)?;
        let expected_h_len = RECURRENT_SLOTS * d_model;
        let meta = SnapshotMetaWire4 {
            teams: self.teams.clone(),
            knowledge: self.knowledge.clone(),
            last_announced_source: self.last_announced_source.clone(),
            observed_values: self.observed_values.clone(),
            rng_state: self.rng.state(),
            last_fallback: self.last_fallback.clone(),
            coverage: self.coverage.clone(),
            rule_refs_left_out: self.rule_refs_left_out,
            choice_guard: self.choice_guard,
            tactic_rules: self.tactic_rules,
        };
        let meta_bytes =
            serde_json::to_vec(&meta).context("serializing PlayerSnapshot4 metadata")?;

        let seat_h_count = if self.h_states_fp16.is_empty() {
            self.h_states.len()
        } else {
            self.h_states_fp16.len()
        };
        let h_bytes_total = seat_h_count * (1 + expected_h_len * 2);
        let hash_bytes_total = self.seen_hashes.len() * 10;
        let mut out = Vec::with_capacity(16 + h_bytes_total + hash_bytes_total + meta_bytes.len());
        out.extend_from_slice(WIRE_MAGIC_V3);
        out.extend_from_slice(&(d_model as u32).to_le_bytes());
        out.push(seat_h_count as u8);
        if self.h_states_fp16.is_empty() {
            for (&seat, h) in &self.h_states {
                out.push(seat);
                for &val in h {
                    out.extend_from_slice(&f32_to_f16(val).to_le_bytes());
                }
            }
        } else {
            for (&seat, h16) in &self.h_states_fp16 {
                out.push(seat);
                extend_u16_le_bytes(h16, &mut out);
            }
        }
        out.extend_from_slice(&(self.seen_hashes.len() as u32).to_le_bytes());
        for (&(seat, hash), &count) in &self.seen_hashes {
            out.push(seat);
            out.extend_from_slice(&hash.to_le_bytes());
            out.push(count);
        }
        out.extend_from_slice(&meta_bytes);
        Ok(out)
    }

    /// Deserializes and strictly validates a [`PlayerSnapshot4`] against `d_model`.
    ///
    /// # Errors
    /// Rejects malformed wire headers, out-of-range seats, mismatched `h_state` lengths,
    /// `NaN`/`Inf` floats, or excessive map entries.
    pub fn from_validated_wire(bytes: &[u8], d_model: usize) -> anyhow::Result<Self> {
        ensure!(d_model > 0 && d_model <= 8192, "invalid d_model {d_model}");
        ensure!(
            bytes.starts_with(WIRE_MAGIC_V3),
            "invalid or unsupported PlayerSnapshot4 wire magic"
        );
        ensure!(bytes.len() >= 15, "truncated PlayerSnapshot4 wire header");
        let wire_d_model = u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]) as usize;
        ensure!(
            wire_d_model == d_model,
            "snapshot wire d_model ({wire_d_model}) != expected ({d_model})"
        );
        let h_count = usize::from(bytes[10]);
        ensure!(
            h_count <= MAX_SEATS,
            "snapshot wire h_count ({h_count}) exceeds MAX_SEATS ({MAX_SEATS})"
        );
        let expected_h_len = RECURRENT_SLOTS * d_model;
        let per_seat_bytes = 1 + expected_h_len * 2;
        let mut cursor = 11;
        ensure!(
            bytes.len() >= cursor + h_count * per_seat_bytes + 4,
            "truncated PlayerSnapshot4 h_states section"
        );
        let mut h_states = BTreeMap::new();
        for _ in 0..h_count {
            let seat = bytes[cursor];
            cursor += 1;
            let mut vec = Vec::with_capacity(expected_h_len);
            for _ in 0..expected_h_len {
                let h_bits = u16::from_le_bytes([bytes[cursor], bytes[cursor + 1]]);
                cursor += 2;
                let f = f16_to_f32_checked(h_bits)?;
                vec.push(f);
            }
            if h_states.insert(seat, vec).is_some() {
                bail!("duplicate seat {seat} in PlayerSnapshot4 h_states");
            }
        }
        let hash_count = u32::from_le_bytes([
            bytes[cursor],
            bytes[cursor + 1],
            bytes[cursor + 2],
            bytes[cursor + 3],
        ]) as usize;
        cursor += 4;
        ensure!(
            hash_count <= MAX_SEEN_HASHES_WIRE,
            "snapshot seen_hashes count {hash_count} exceeds limit {MAX_SEEN_HASHES_WIRE}"
        );
        ensure!(
            bytes.len() >= cursor + hash_count * 10,
            "truncated PlayerSnapshot4 seen_hashes section"
        );
        let mut seen_hashes = BTreeMap::new();
        for _ in 0..hash_count {
            let seat = bytes[cursor];
            let hash = u64::from_le_bytes([
                bytes[cursor + 1],
                bytes[cursor + 2],
                bytes[cursor + 3],
                bytes[cursor + 4],
                bytes[cursor + 5],
                bytes[cursor + 6],
                bytes[cursor + 7],
                bytes[cursor + 8],
            ]);
            let count = bytes[cursor + 9];
            cursor += 10;
            seen_hashes.insert((seat, hash), count);
        }
        let meta: SnapshotMetaWire4 = serde_json::from_slice(&bytes[cursor..])
            .context("deserializing PlayerSnapshot4 metadata tail")?;
        let snap = Self {
            teams: meta.teams,
            h_states,
            h_states_fp16: BTreeMap::new(),
            knowledge: meta.knowledge,
            seen_hashes,
            last_announced_source: meta.last_announced_source,
            observed_values: meta.observed_values,
            rng: Rng::new(meta.rng_state),
            last_fallback: meta.last_fallback,
            coverage: meta.coverage,
            rule_refs_left_out: meta.rule_refs_left_out,
            choice_guard: meta.choice_guard,
            tactic_rules: meta.tactic_rules,
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
        for (&seat, h16) in &self.h_states_fp16 {
            ensure!(
                usize::from(seat) < MAX_SEATS,
                "snapshot h_state_fp16 seat {seat} out of bounds"
            );
            ensure!(
                h16.len() == expected_h_len,
                "snapshot h_state_fp16 length {} for seat {seat} != expected {expected_h_len}",
                h16.len()
            );
            if h16.iter().any(|&bits| (bits & 0x7c00) == 0x7c00) {
                bail!("snapshot h_state_fp16 for seat {seat} contains non-finite FP16 value");
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
            dirty_h_seats: _,
            tactician: _,
        } = self;
        PlayerSnapshot4 {
            teams: teams.clone(),
            h_states: h_states.clone(),
            h_states_fp16: BTreeMap::new(),
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

    /// Takes a fresh hot snapshot (`h_states_fp16` in `FP16`) by moving metadata out of `self`
    /// without cloning `SeatKnowledge4` or `BTreeMap`s.
    #[must_use]
    pub fn take_hot_snapshot(&mut self) -> PlayerSnapshot4 {
        let mut h_states_fp16 = BTreeMap::new();
        for (&seat, h32) in &self.h_states {
            let mut h16 = Vec::new();
            pack_f32_slice_to_f16(h32, &mut h16);
            h_states_fp16.insert(seat, h16);
        }
        self.dirty_h_seats = 0;
        PlayerSnapshot4 {
            teams: std::mem::take(&mut self.teams),
            h_states: BTreeMap::new(),
            h_states_fp16,
            knowledge: std::mem::take(&mut self.knowledge),
            seen_hashes: std::mem::take(&mut self.seen_hashes),
            last_announced_source: std::mem::take(&mut self.last_announced_source),
            observed_values: std::mem::take(&mut self.observed_values),
            rng: self.rng.clone(),
            last_fallback: self.last_fallback.take(),
            coverage: std::mem::take(&mut self.coverage),
            rule_refs_left_out: self.rule_refs_left_out,
            choice_guard: self.choice_guard,
            tactic_rules: self.tactic_rules,
        }
    }

    /// Zero-clone context switch into `self` from a Hot [`PlayerSnapshot4`]:
    /// swaps all metadata pointers in $O(1)$ (`std::mem::swap`) and, when `unpack_h_states`
    /// is true (non-affinity switch), unpacks `h_states_fp16` into `self.h_states` in-place
    /// reusing existing vector capacity.
    pub fn swap_in_hot(&mut self, snapshot: &mut PlayerSnapshot4, unpack_h_states: bool) {
        std::mem::swap(&mut self.teams, &mut snapshot.teams);
        std::mem::swap(&mut self.knowledge, &mut snapshot.knowledge);
        std::mem::swap(&mut self.seen_hashes, &mut snapshot.seen_hashes);
        std::mem::swap(
            &mut self.last_announced_source,
            &mut snapshot.last_announced_source,
        );
        std::mem::swap(&mut self.observed_values, &mut snapshot.observed_values);
        std::mem::swap(&mut self.rng, &mut snapshot.rng);
        std::mem::swap(&mut self.last_fallback, &mut snapshot.last_fallback);
        std::mem::swap(&mut self.coverage, &mut snapshot.coverage);
        std::mem::swap(
            &mut self.rule_refs_left_out,
            &mut snapshot.rule_refs_left_out,
        );
        std::mem::swap(&mut self.choice_guard, &mut snapshot.choice_guard);
        std::mem::swap(&mut self.tactic_rules, &mut snapshot.tactic_rules);

        self.dirty_h_seats = 0;
        if unpack_h_states {
            if snapshot.h_states_fp16.is_empty() {
                self.h_states.clone_from(&snapshot.h_states);
                self.dirty_h_seats = 0xff;
            } else {
                // The slot last served another session, maybe of more seats:
                // a seat this session does not have must not ride along into
                // its state at the next `swap_out_hot`.
                self.h_states
                    .retain(|seat, _| snapshot.h_states_fp16.contains_key(seat));
                for (&seat, h16) in &snapshot.h_states_fp16 {
                    let dst = self.h_states.entry(seat).or_default();
                    unpack_f16_slice_to_f32(h16, dst);
                }
            }
        }
    }

    /// Zero-clone context switch back out from `self` into a Hot [`PlayerSnapshot4`]:
    /// swaps all metadata pointers back in $O(1)$ (`std::mem::swap`) and quantizes only
    /// dirtied seats' `self.h_states` into `snapshot.h_states_fp16` in-place reusing capacity.
    pub fn swap_out_hot(&mut self, snapshot: &mut PlayerSnapshot4) {
        std::mem::swap(&mut self.teams, &mut snapshot.teams);
        std::mem::swap(&mut self.knowledge, &mut snapshot.knowledge);
        std::mem::swap(&mut self.seen_hashes, &mut snapshot.seen_hashes);
        std::mem::swap(
            &mut self.last_announced_source,
            &mut snapshot.last_announced_source,
        );
        std::mem::swap(&mut self.observed_values, &mut snapshot.observed_values);
        std::mem::swap(&mut self.rng, &mut snapshot.rng);
        std::mem::swap(&mut self.last_fallback, &mut snapshot.last_fallback);
        std::mem::swap(&mut self.coverage, &mut snapshot.coverage);
        std::mem::swap(
            &mut self.rule_refs_left_out,
            &mut snapshot.rule_refs_left_out,
        );
        std::mem::swap(&mut self.choice_guard, &mut snapshot.choice_guard);
        std::mem::swap(&mut self.tactic_rules, &mut snapshot.tactic_rules);

        let dirty = self.dirty_h_seats;
        self.dirty_h_seats = 0;
        for (&seat, h32) in &self.h_states {
            let bit = 1_u8.wrapping_shl(u32::from(seat));
            let needs_pack = (dirty & bit) != 0
                || snapshot
                    .h_states_fp16
                    .get(&seat)
                    .is_none_or(|v| v.len() != h32.len());
            if needs_pack {
                let dst = snapshot.h_states_fp16.entry(seat).or_default();
                pack_f32_slice_to_f16(h32, dst);
            }
        }
        snapshot.h_states.clear();
    }

    /// Puts back the per-game state `snapshot` took: whatever was observed,
    /// answered and sampled since is undone. Every field of the snapshot is
    /// named, so one added to it does not compile until it is put back.
    pub fn restore(&mut self, snapshot: &PlayerSnapshot4) {
        let PlayerSnapshot4 {
            teams,
            h_states,
            h_states_fp16,
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
        if h_states.is_empty() {
            self.h_states.clear();
            for (&seat, h16) in h_states_fp16 {
                let dst = self.h_states.entry(seat).or_default();
                dst.resize(h16.len(), 0.0);
                for (d, &bits) in dst.iter_mut().zip(h16.iter()) {
                    *d = f16_to_f32(bits);
                }
            }
        } else {
            self.h_states.clone_from(h_states);
        }
        self.dirty_h_seats = 0xff;
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
        self.dirty_h_seats = 0xff;
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

impl baylee_driver::SnapshottableState for PlayerSnapshot4 {
    type ColdMeta = ColdMeta4;

    fn pack_fp16_in_ram(&mut self) {
        Self::pack_fp16_in_ram(self);
    }

    fn validate(&self, d_model: usize) -> anyhow::Result<()> {
        Self::validate(self, d_model)
    }

    fn approx_ram_bytes(&self) -> usize {
        Self::approx_ram_bytes(self)
    }

    fn observed_value(&self, seat: PlayerId) -> Option<f32> {
        Self::observed_value(self, seat)
    }

    fn zeroize_sensitive(&mut self) {
        Self::zeroize_sensitive(self);
    }

    fn to_cold_wire_into(
        &mut self,
        d_model: usize,
        out: &mut Vec<u8>,
    ) -> anyhow::Result<Self::ColdMeta> {
        Self::to_cold_wire_into(self, d_model, out)
    }

    fn from_cold_wire_and_meta(
        bytes: &[u8],
        d_model: usize,
        meta: Self::ColdMeta,
    ) -> anyhow::Result<Self> {
        Self::from_cold_wire_and_meta(bytes, d_model, meta)
    }
}

impl baylee_driver::SnapshottableBrain for NetPlayer4 {
    type State = PlayerSnapshot4;
    type Answer = super::Answer4;

    fn d_model(&self) -> usize {
        Self::d_model(self)
    }

    fn init_fresh_session(
        &mut self,
        preset: &baylee_core::preset::GamePreset,
        policy_seed: u64,
    ) -> Self::State {
        self.reseed(policy_seed);
        self.init_game(preset);
        self.take_hot_snapshot()
    }

    fn swap_in_hot(&mut self, snapshot: &mut Self::State, unpack_h_states: bool) {
        Self::swap_in_hot(self, snapshot, unpack_h_states);
    }

    fn swap_out_hot(&mut self, snapshot: &mut Self::State) {
        Self::swap_out_hot(self, snapshot);
    }

    fn observe(&mut self, view: &baylee_view::PlayerView) -> anyhow::Result<()> {
        Self::observe(self, view)
    }

    fn act_detailed(
        &mut self,
        view: &baylee_view::PlayerView,
        pending: &baylee_engine::choice::Pending,
        teams: &[Option<u8>],
    ) -> Self::Answer {
        Self::act_detailed(self, view, pending, teams)
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

    /// One worker slot serves a four-seat session, then a duel without
    /// affinity: the duel's state keeps its own two seats, and the four-seat
    /// session's other seats do not ride along into it.
    #[test]
    fn a_switch_to_a_smaller_table_carries_no_seat_of_the_last_session() {
        let deck = crate::housedeck::HouseDeck::named("victory").unwrap();
        let steady = baylee_core::preset::AIProfile::STEADY;
        let four = crate::selfplay::table_for(5, &[&deck; 4], &[steady; 4], &[1, 2, 1, 2]);
        let duel = crate::selfplay::table_for(6, &[&deck; 2], &[steady; 2], &[]);
        let mut net = player();
        net.init_game(&four);
        let mut big = net.take_hot_snapshot();
        net.init_game(&duel);
        let mut small = net.take_hot_snapshot();
        let seats = |snapshot: &PlayerSnapshot4| -> Vec<u8> {
            snapshot.h_states_fp16.keys().copied().collect()
        };
        assert_eq!(seats(&big), [0, 1, 2, 3]);
        assert_eq!(seats(&small), [0, 1]);

        net.swap_in_hot(&mut big, true);
        net.h_states.get_mut(&3).unwrap()[0] = 0.5;
        net.dirty_h_seats |= 1 << 3;
        net.swap_out_hot(&mut big);
        net.swap_in_hot(&mut small, true);
        assert_eq!(net.h_states.keys().copied().collect::<Vec<_>>(), [0, 1]);
        net.swap_out_hot(&mut small);
        assert_eq!(seats(&small), [0, 1]);
        assert_eq!(seats(&big), [0, 1, 2, 3]);
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
