//! Tiered Hot-RAM / Cold-SSD session pool and $O(N_{\text{hot}})$ urgency-aware LRU eviction scheduler.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::time::Instant;

use anyhow::{bail, ensure};

use crate::config::{MuxConfig4, SessionUrgency4};
use crate::metrics::MuxMetrics4;
use crate::snapshot::SnapshottableState;
use crate::swap_store::{KeyedSwapStore4, SwapHandle4};

enum SessionResidency4<S: SnapshottableState> {
    Hot { snapshot: Box<S>, ram_bytes: usize },
    CheckedOut { prev_ram_bytes: usize },
    Cold(SwapHandle4<S::ColdMeta>),
}

struct SessionRecord4<S: SnapshottableState> {
    residency: SessionResidency4<S>,
    urgency: SessionUrgency4,
    last_touch_tick: u64,
    swap_generation: u64,
    state_revision: u64,
    access_count: u64,
}

/// Manages hundreds or thousands of concurrent game sessions across Hot RAM and Cold SSD.
pub struct TieredSessionPool4<S: SnapshottableState> {
    config: MuxConfig4,
    swap_store: KeyedSwapStore4,
    sessions: BTreeMap<u64, SessionRecord4<S>>,
    hot_ids: BTreeSet<u64>,
    cold_count: usize,
    hot_ram_bytes: usize,
    cold_ssd_bytes: u64,
    clock_tick: u64,
}

impl<S: SnapshottableState> TieredSessionPool4<S> {
    /// Creates a new tiered session pool backed by `swap_store`.
    ///
    /// # Errors
    /// If `config` fails validation or the swap store cannot be initialized.
    pub fn new(config: MuxConfig4) -> anyhow::Result<Self> {
        config.validate()?;
        let swap_store = KeyedSwapStore4::new(
            config.swap_dir.clone(),
            config.d_model,
            config.max_swap_file_bytes,
            config.max_uncompressed_bytes,
            config.durable_swap_fsync,
        )?;
        Ok(Self {
            config,
            swap_store,
            sessions: BTreeMap::new(),
            hot_ids: BTreeSet::new(),
            cold_count: 0,
            hot_ram_bytes: 0,
            cold_ssd_bytes: 0,
            clock_tick: 0,
        })
    }

    const fn next_tick(&mut self) -> u64 {
        self.clock_tick = self.clock_tick.wrapping_add(1);
        self.clock_tick
    }

    /// Number of sessions currently resident in Hot RAM.
    #[must_use]
    pub fn hot_count(&self) -> usize {
        self.hot_ids.len()
    }

    /// Number of sessions currently swapped to Cold SSD.
    #[must_use]
    pub const fn cold_count(&self) -> usize {
        self.cold_count
    }

    /// Approximate RAM footprint of all Hot sessions (bytes).
    #[must_use]
    pub const fn hot_ram_bytes(&self) -> usize {
        self.hot_ram_bytes
    }

    /// Total on-disk bytes of all active Cold SSD swap files.
    #[must_use]
    pub const fn cold_ssd_bytes(&self) -> u64 {
        self.cold_ssd_bytes
    }

    /// Whether `session_id` is currently resident in Hot RAM.
    #[must_use]
    pub fn is_hot(&self, session_id: u64) -> bool {
        self.hot_ids.contains(&session_id)
    }

    /// Returns the filesystem path of a cold session's swap file (if currently swapped to SSD).
    #[must_use]
    pub fn cold_swap_path(&self, session_id: u64) -> Option<PathBuf> {
        match &self.sessions.get(&session_id)?.residency {
            SessionResidency4::Cold(h) => Some(self.swap_store.swap_path(h)),
            SessionResidency4::Hot { .. } | SessionResidency4::CheckedOut { .. } => None,
        }
    }

    /// Rotates the active 256-bit swap master key without interrupting cold sessions.
    ///
    /// # Errors
    /// If CSPRNG key generation fails or too many unpruned key epochs accumulate.
    pub fn rotate_swap_key(&mut self, metrics: &mut MuxMetrics4) -> anyhow::Result<u64> {
        let min_cold_epoch = self.min_active_cold_epoch();
        let epoch = self.swap_store.rotate_master_key(min_cold_epoch)?;
        metrics.key_rotations += 1;
        Ok(epoch)
    }

    fn min_active_cold_epoch(&self) -> Option<u64> {
        self.sessions
            .values()
            .filter_map(|rec| match &rec.residency {
                SessionResidency4::Cold(h) => Some(h.key_epoch),
                SessionResidency4::Hot { .. } | SessionResidency4::CheckedOut { .. } => None,
            })
            .min()
    }

    /// Registers a new session in Hot RAM, evicting cold/idle sessions to SSD if needed.
    ///
    /// # Errors
    /// If `session_id` already exists, total session quota is reached, or eviction fails.
    pub fn insert_session(
        &mut self,
        session_id: u64,
        mut snapshot: S,
        urgency: SessionUrgency4,
        metrics: &mut MuxMetrics4,
    ) -> anyhow::Result<()> {
        ensure!(
            !self.sessions.contains_key(&session_id),
            "session {session_id} is already registered"
        );
        ensure!(
            self.sessions.len() < self.config.max_total_sessions,
            "total session quota ({}) reached",
            self.config.max_total_sessions
        );
        snapshot.pack_fp16_in_ram();
        snapshot.validate(self.config.d_model)?;
        let ram_bytes = snapshot.approx_ram_bytes();
        self.evict_until_fits(&[session_id], ram_bytes, metrics)?;
        let tick = self.next_tick();
        self.sessions.insert(
            session_id,
            SessionRecord4 {
                residency: SessionResidency4::Hot {
                    snapshot: Box::new(snapshot),
                    ram_bytes,
                },
                urgency,
                last_touch_tick: tick,
                swap_generation: 0,
                state_revision: 1,
                access_count: 1,
            },
        );
        self.hot_ids.insert(session_id);
        self.hot_ram_bytes += ram_bytes;
        metrics.sessions_created += 1;
        Ok(())
    }

    /// Updates a session's scheduling urgency (`Idle`, `Soon`, `Immediate`).
    ///
    /// # Errors
    /// If `session_id` is not registered.
    pub fn set_urgency(&mut self, session_id: u64, urgency: SessionUrgency4) -> anyhow::Result<()> {
        let rec = self
            .sessions
            .get_mut(&session_id)
            .ok_or_else(|| anyhow::anyhow!("unknown session {session_id}"))?;
        rec.urgency = urgency;
        Ok(())
    }

    /// Proactively pages a cold SSD session into Hot RAM before its turn arrives.
    ///
    /// # Errors
    /// If `session_id` is unknown or SSD page-in / authentication fails.
    pub fn prefetch(&mut self, session_id: u64, metrics: &mut MuxMetrics4) -> anyhow::Result<bool> {
        self.prefetch_pinned(session_id, &[], metrics)
    }

    /// Proactively pages a batch of upcoming cold sessions into Hot RAM while protecting
    /// all sessions in `session_ids` from evicting each other.
    ///
    /// # Errors
    /// If any session ID is unknown or SSD page-in / authentication fails.
    pub fn prefetch_batch(
        &mut self,
        session_ids: &[u64],
        metrics: &mut MuxMetrics4,
    ) -> anyhow::Result<usize> {
        let mut paged_in = 0;
        for &sid in session_ids {
            if self.prefetch_pinned(sid, session_ids, metrics)? {
                paged_in += 1;
            }
        }
        Ok(paged_in)
    }

    fn prefetch_pinned(
        &mut self,
        session_id: u64,
        pinned: &[u64],
        metrics: &mut MuxMetrics4,
    ) -> anyhow::Result<bool> {
        if self.is_hot(session_id) {
            let tick = self.next_tick();
            if let Some(rec) = self.sessions.get_mut(&session_id) {
                rec.last_touch_tick = tick;
                if rec.urgency == SessionUrgency4::Idle {
                    rec.urgency = SessionUrgency4::Soon;
                }
            }
            metrics.prefetch_already_hot += 1;
            return Ok(false);
        }
        self.page_in_session(session_id, pinned, true, metrics)?;
        Ok(true)
    }

    /// Borrows a session's state directly from Hot RAM without cloning
    /// (paging in from SSD if cold) and returns `(snapshot_ref, state_revision)`.
    ///
    /// # Errors
    /// If `session_id` is unknown or SSD page-in / authentication fails.
    pub fn acquire_hot_ref(
        &mut self,
        session_id: u64,
        metrics: &mut MuxMetrics4,
    ) -> anyhow::Result<(&S, u64)> {
        if self.is_hot(session_id) {
            metrics.hot_ram_hits += 1;
        } else {
            self.page_in_session(session_id, &[session_id], false, metrics)?;
        }
        let tick = self.next_tick();
        let rec = self
            .sessions
            .get_mut(&session_id)
            .ok_or_else(|| anyhow::anyhow!("unknown session {session_id}"))?;
        rec.last_touch_tick = tick;
        rec.access_count += 1;
        let rev = rec.state_revision;
        match &rec.residency {
            SessionResidency4::Hot { snapshot, .. } => Ok((snapshot.as_ref(), rev)),
            SessionResidency4::CheckedOut { .. } => {
                bail!("internal error: session {session_id} already checked out")
            }
            SessionResidency4::Cold(_) => bail!("internal error: session {session_id} still cold"),
        }
    }

    /// Moves a session's `Box<S>` out of Hot RAM in $O(1)$ without cloning
    /// so a worker slot can `std::mem::swap` against it in-place.
    ///
    /// # Errors
    /// If `session_id` is unknown or SSD page-in / authentication fails.
    pub fn checkout_hot_box(
        &mut self,
        session_id: u64,
        metrics: &mut MuxMetrics4,
    ) -> anyhow::Result<(Box<S>, u64)> {
        if self.is_hot(session_id) {
            metrics.hot_ram_hits += 1;
        } else {
            self.page_in_session(session_id, &[session_id], false, metrics)?;
        }
        let tick = self.next_tick();
        let rec = self
            .sessions
            .get_mut(&session_id)
            .ok_or_else(|| anyhow::anyhow!("unknown session {session_id}"))?;
        rec.last_touch_tick = tick;
        rec.access_count += 1;
        let rev = rec.state_revision;
        let prev_ram_bytes = match &rec.residency {
            SessionResidency4::Hot { ram_bytes, .. } => *ram_bytes,
            SessionResidency4::CheckedOut { .. } => {
                bail!("session {session_id} is already checked out")
            }
            SessionResidency4::Cold(_) => bail!("internal error: session {session_id} still cold"),
        };
        let old = std::mem::replace(
            &mut rec.residency,
            SessionResidency4::CheckedOut { prev_ram_bytes },
        );
        let SessionResidency4::Hot { snapshot, .. } = old else {
            unreachable!();
        };
        Ok((snapshot, rev))
    }

    /// Ensures all `session_ids` are resident in Hot RAM simultaneously and moves their
    /// `Box<S>` pointers out in $O(1)$ without cloning for parallel wave execution.
    ///
    /// All or nothing: every session is checked before any is moved out, so a
    /// wave that fails here leaves no session checked out.
    ///
    /// # Errors
    /// If `session_ids.len() > max_hot_sessions`, a session is unknown, named twice or
    /// already checked out, or SSD page-in fails.
    pub fn checkout_wave_boxes(
        &mut self,
        session_ids: &[u64],
        metrics: &mut MuxMetrics4,
    ) -> anyhow::Result<Vec<(u64, Box<S>, u64)>> {
        ensure!(
            session_ids.len() <= self.config.max_hot_sessions,
            "wave size ({}) exceeds max_hot_sessions ({})",
            session_ids.len(),
            self.config.max_hot_sessions
        );
        let mut named = BTreeSet::new();
        for &sid in session_ids {
            ensure!(named.insert(sid), "session {sid} is named twice in a wave");
            match self.sessions.get(&sid).map(|rec| &rec.residency) {
                None => bail!("unknown session {sid}"),
                Some(SessionResidency4::CheckedOut { .. }) => {
                    bail!("session {sid} is already checked out")
                }
                Some(SessionResidency4::Hot { .. } | SessionResidency4::Cold(_)) => {}
            }
        }
        for &sid in session_ids {
            if self.is_hot(sid) {
                metrics.hot_ram_hits += 1;
            } else {
                self.page_in_session(sid, session_ids, false, metrics)?;
            }
        }
        let mut out = Vec::with_capacity(session_ids.len());
        for &sid in session_ids {
            let tick = self.next_tick();
            let rec = self
                .sessions
                .get_mut(&sid)
                .ok_or_else(|| anyhow::anyhow!("unknown session {sid}"))?;
            rec.last_touch_tick = tick;
            rec.access_count += 1;
            let rev = rec.state_revision;
            let prev_ram_bytes = match &rec.residency {
                SessionResidency4::Hot { ram_bytes, .. } => *ram_bytes,
                SessionResidency4::CheckedOut { .. } => {
                    bail!("duplicate or checked-out session {sid} in wave")
                }
                SessionResidency4::Cold(_) => bail!("internal error: wave session {sid} not hot"),
            };
            let old = std::mem::replace(
                &mut rec.residency,
                SessionResidency4::CheckedOut { prev_ram_bytes },
            );
            let SessionResidency4::Hot { snapshot, .. } = old else {
                unreachable!();
            };
            out.push((sid, snapshot, rev));
        }
        Ok(out)
    }

    /// Returns a checked-out `Box<S>` back to Hot RAM in $O(1)$ without heap allocation.
    ///
    /// # Errors
    /// If `session_id` is unknown or eviction fails.
    pub fn commit_hot_box(
        &mut self,
        session_id: u64,
        snapshot: Box<S>,
        urgency: SessionUrgency4,
        metrics: &mut MuxMetrics4,
    ) -> anyhow::Result<u64> {
        let new_bytes = snapshot.approx_ram_bytes();
        let tick = self.next_tick();
        let rec = self
            .sessions
            .get_mut(&session_id)
            .ok_or_else(|| anyhow::anyhow!("unknown session {session_id}"))?;
        match &rec.residency {
            SessionResidency4::CheckedOut { prev_ram_bytes } => {
                self.hot_ram_bytes = self.hot_ram_bytes.saturating_sub(*prev_ram_bytes) + new_bytes;
            }
            SessionResidency4::Hot { ram_bytes, .. } => {
                self.hot_ram_bytes = self.hot_ram_bytes.saturating_sub(*ram_bytes) + new_bytes;
            }
            SessionResidency4::Cold(_) => {
                self.cold_count = self.cold_count.saturating_sub(1);
                self.hot_ids.insert(session_id);
                self.hot_ram_bytes += new_bytes;
            }
        }
        rec.state_revision = rec.state_revision.wrapping_add(1);
        let new_rev = rec.state_revision;
        rec.residency = SessionResidency4::Hot {
            snapshot,
            ram_bytes: new_bytes,
        };
        rec.urgency = urgency;
        rec.last_touch_tick = tick;
        self.evict_until_fits(&[session_id], 0, metrics)?;
        Ok(new_rev)
    }

    /// Explicitly evicts a specific Hot session to Cold SSD, zeroizing its RAM buffer if configured.
    ///
    /// # Errors
    /// If `session_id` is unknown or SSD encryption/write fails.
    #[allow(clippy::cast_possible_truncation)]
    pub fn evict_session(
        &mut self,
        session_id: u64,
        metrics: &mut MuxMetrics4,
    ) -> anyhow::Result<()> {
        let zeroize = self.config.zeroize_on_evict;
        let rec = self
            .sessions
            .get_mut(&session_id)
            .ok_or_else(|| anyhow::anyhow!("unknown session {session_id}"))?;
        let SessionResidency4::Hot {
            snapshot,
            ram_bytes,
        } = &mut rec.residency
        else {
            return Ok(());
        };
        let next_gen = rec
            .swap_generation
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("swap generation overflow for session {session_id}"))?;
        let t0 = Instant::now();
        let handle = self
            .swap_store
            .page_out(session_id, next_gen, &mut **snapshot)?;
        metrics.record_page_out_us(t0.elapsed().as_micros() as u64);
        metrics.ssd_page_outs += 1;
        metrics.ssd_bytes_written += handle.disk_bytes;

        let freed_bytes = *ram_bytes;
        if zeroize {
            snapshot.zeroize_sensitive();
        }
        self.hot_ram_bytes = self.hot_ram_bytes.saturating_sub(freed_bytes);
        self.hot_ids.remove(&session_id);
        self.cold_count += 1;
        self.cold_ssd_bytes += handle.disk_bytes;
        rec.swap_generation = next_gen;
        rec.residency = SessionResidency4::Cold(handle);
        Ok(())
    }

    /// Closes a session, zeroizes any Hot RAM state, and wipes any associated SSD swap file.
    pub fn remove_session(&mut self, session_id: u64, metrics: &mut MuxMetrics4) -> bool {
        let Some(mut rec) = self.sessions.remove(&session_id) else {
            return false;
        };
        match &mut rec.residency {
            SessionResidency4::Hot {
                snapshot,
                ram_bytes,
            } => {
                if self.config.zeroize_on_evict {
                    snapshot.zeroize_sensitive();
                }
                self.hot_ids.remove(&session_id);
                self.hot_ram_bytes = self.hot_ram_bytes.saturating_sub(*ram_bytes);
            }
            SessionResidency4::CheckedOut { prev_ram_bytes } => {
                self.hot_ids.remove(&session_id);
                self.hot_ram_bytes = self.hot_ram_bytes.saturating_sub(*prev_ram_bytes);
            }
            SessionResidency4::Cold(handle) => {
                self.cold_count = self.cold_count.saturating_sub(1);
                self.cold_ssd_bytes = self.cold_ssd_bytes.saturating_sub(handle.disk_bytes);
                self.swap_store.remove_swap_file(handle);
            }
        }
        metrics.sessions_closed += 1;
        true
    }

    #[allow(clippy::cast_possible_truncation)]
    fn page_in_session(
        &mut self,
        session_id: u64,
        pinned: &[u64],
        is_prefetch: bool,
        metrics: &mut MuxMetrics4,
    ) -> anyhow::Result<()> {
        let handle = match self.sessions.get_mut(&session_id) {
            Some(rec) => match &rec.residency {
                SessionResidency4::Cold(_) => {
                    let SessionResidency4::Cold(h) = std::mem::replace(
                        &mut rec.residency,
                        SessionResidency4::CheckedOut { prev_ram_bytes: 0 },
                    ) else {
                        unreachable!();
                    };
                    h
                }
                SessionResidency4::Hot { .. } | SessionResidency4::CheckedOut { .. } => {
                    return Ok(());
                }
            },
            None => bail!("unknown session {session_id}"),
        };
        let disk_bytes = handle.disk_bytes;
        let t0 = Instant::now();
        let snapshot: S = match self.swap_store.page_in(session_id, handle.clone()) {
            Ok(s) => s,
            Err(e) => {
                if let Some(rec) = self.sessions.get_mut(&session_id) {
                    rec.residency = SessionResidency4::Cold(handle);
                }
                metrics.security_rejections += 1;
                return Err(e);
            }
        };
        metrics.record_page_in_us(t0.elapsed().as_micros() as u64);
        metrics.ssd_bytes_read += disk_bytes;
        if is_prefetch {
            metrics.prefetch_page_ins += 1;
        } else {
            metrics.cold_ssd_page_ins += 1;
        }
        let ram_bytes = snapshot.approx_ram_bytes();
        self.evict_until_fits(pinned, ram_bytes, metrics)?;
        let tick = self.next_tick();
        if let Some(rec) = self.sessions.get_mut(&session_id) {
            rec.residency = SessionResidency4::Hot {
                snapshot: Box::new(snapshot),
                ram_bytes,
            };
            rec.last_touch_tick = tick;
            if is_prefetch && rec.urgency == SessionUrgency4::Idle {
                rec.urgency = SessionUrgency4::Soon;
            }
        }
        self.cold_count = self.cold_count.saturating_sub(1);
        self.cold_ssd_bytes = self.cold_ssd_bytes.saturating_sub(disk_bytes);
        self.hot_ids.insert(session_id);
        self.hot_ram_bytes += ram_bytes;
        let min_cold_epoch = self.min_active_cold_epoch();
        self.swap_store.prune_retired_keys(min_cold_epoch);
        Ok(())
    }

    fn evict_until_fits(
        &mut self,
        protected_sessions: &[u64],
        incoming_bytes: usize,
        metrics: &mut MuxMetrics4,
    ) -> anyhow::Result<()> {
        while self.hot_ids.len() >= self.config.max_hot_sessions
            || (!self.hot_ids.is_empty()
                && self.hot_ram_bytes + incoming_bytes > self.config.max_hot_ram_bytes)
        {
            let victim = self
                .hot_ids
                .iter()
                .copied()
                .filter(|sid| !protected_sessions.contains(sid))
                .filter_map(|sid| {
                    let rec = self.sessions.get(&sid)?;
                    if !matches!(rec.residency, SessionResidency4::Hot { .. }) {
                        return None;
                    }
                    Some((sid, (rec.urgency, rec.last_touch_tick)))
                })
                .min_by_key(|&(_, key)| key)
                .map(|(sid, _)| sid);
            let Some(victim_id) = victim else {
                break;
            };
            self.evict_session(victim_id, metrics)?;
        }
        Ok(())
    }
}

impl<S: SnapshottableState> Drop for TieredSessionPool4<S> {
    fn drop(&mut self) {
        let zeroize = self.config.zeroize_on_evict;
        for rec in self.sessions.values_mut() {
            match &mut rec.residency {
                SessionResidency4::Hot { snapshot, .. } => {
                    if zeroize {
                        snapshot.zeroize_sensitive();
                    }
                }
                SessionResidency4::CheckedOut { .. } => {}
                SessionResidency4::Cold(handle) => {
                    self.swap_store.remove_swap_file(handle);
                }
            }
        }
    }
}
