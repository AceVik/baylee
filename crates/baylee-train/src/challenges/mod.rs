//! Bounded offline challenge generation with fresh execution and semantic certificates.
//! No candidate leaves this module without three independently replayed winning routes.

mod presets;
mod publication;
mod semantic;
mod solve;
#[cfg(test)]
mod tests;
pub mod tower;

use std::collections::BTreeSet;

use anyhow::{Context, Result, ensure};
use baylee_challenges::Package;
use sha2::{Digest, Sha256};

pub use central_api::challenges::{Kind, SolutionFamily as Family};

/// One retained proof, including a record for operator audit, never for public publication.
pub struct Proof {
    /// Exact full record, including the private preset.
    pub record: Vec<u8>,
    /// Hash of the record above.
    pub record_digest: [u8; 32],
    /// Canonical contribution signature, excluding order, elapsed turns and object copies.
    pub strategy_digest: [u8; 32],
    /// Actual lethal damage source, read from the executed journal.
    pub family: Family,
    /// Winning game turn.
    pub turn: u32,
}

/// Admitted output: immutable execution data, public deck list and retained evidence.
pub struct Certified {
    /// Calendar complexity class.
    pub kind: Kind,
    /// Frozen package for the official executor.
    pub package: Package,
    /// Canonical player deck list without draw order.
    pub deck: String,
    /// Three distinct proofs spanning two terminal damage families.
    pub proofs: Vec<Proof>,
}

/// Input/turn caps are deterministic. Wall time only cancels a candidate; never alters a move.
#[derive(Clone, Copy)]
pub struct Limits {
    /// Maximum accepted human answers per probe.
    pub answers: u32,
    /// Maximum game turn per probe.
    pub turns: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            answers: 2_000,
            turns: 60,
        }
    }
}

/// Generate one candidate and prove its diversity. A rejected candidate is never a fallback.
///
/// # Errors
/// Unusable cards/decks, caps, invalid actions, replay divergence or insufficient diversity.
pub fn generate(kind: Kind, seed: u64, limits: Limits) -> Result<Certified> {
    Generator::from_source(&crate::working::repo_root())?.generate(kind, seed, limits)
}

/// Reuse one working-card evidence snapshot across every candidate in a stock pass.
pub struct Generator {
    working: crate::working::Working,
}

impl Generator {
    /// Load evidence from an explicitly selected matching source checkout.
    /// # Errors
    /// Missing, unreadable or inconsistent working-card evidence.
    pub fn from_source(root: &std::path::Path) -> Result<Self> {
        Ok(Self {
            working: crate::working::Working::scan(root).context("working card evidence")?,
        })
    }

    /// Generate and certify one candidate using the retained evidence snapshot.
    /// # Errors
    /// As [`generate`], including insufficient semantic diversity.
    pub fn generate(&self, kind: Kind, seed: u64, limits: Limits) -> Result<Certified> {
        prove(kind, seed, limits, &self.working)
    }
}

fn prove(
    kind: Kind,
    seed: u64,
    limits: Limits,
    working: &crate::working::Working,
) -> Result<Certified> {
    ensure!(
        limits.answers > 0 && limits.turns > 0,
        "positive probe limits required"
    );
    let (preset, deck) = presets::build(kind, seed, working)?;
    certify(
        kind,
        preset,
        deck,
        limits,
        if kind == Kind::Weekly { 9 } else { 1 },
    )
}

fn certify(
    kind: Kind,
    preset: baylee_core::preset::GamePreset,
    deck: String,
    limits: Limits,
    minimum_turn: u32,
) -> Result<Certified> {
    ensure!(
        limits.answers > 0 && limits.turns > 0,
        "positive probe limits required"
    );
    let mut proofs = Vec::new();
    let mut families = BTreeSet::new();
    let mut wins = 0;
    let mut unclassified = 0;
    let mut stats = solve::Statistics::default();
    for solver in solve::strategies(kind) {
        let Some(run) = solve::run(&preset, solver, limits, &mut stats)? else {
            continue;
        };
        wins += 1;
        ensure!(
            run.session.state().turn.number >= minimum_turn,
            "candidate has a proved win before its minimum game turn"
        );
        let Ok((family, strategy_digest)) = semantic::classify(&run.session, &run.trace) else {
            unclassified += 1;
            continue;
        };
        let Some(slot) = proof_slot(&proofs, family, strategy_digest) else {
            continue;
        };
        // A wholly new host runs the same submitted human moves against its own
        // freshly initialized opponent. This is stronger than recorded-action replay.
        solve::verify_fresh(&preset, &run)?;
        let replay = baylee_gamehost::record::replay_exact(&run.record)
            .map_err(|error| anyhow::anyhow!("exact replay failed: {error:?}"))?;
        ensure!(
            replay.ended && replay.engine.snapshot_hash() == run.session.snapshot_hash(),
            "terminal replay mismatch"
        );
        let proof = Proof {
            record_digest: Sha256::digest(&run.record).into(),
            record: run.record,
            strategy_digest,
            family,
            turn: run.session.state().turn.number,
        };
        if slot == proofs.len() {
            proofs.push(proof);
        } else {
            proofs[slot] = proof;
        }
        families = proofs.iter().map(|proof| proof.family).collect();
        if proofs.len() >= 3 && families.len() >= 2 {
            break;
        }
        // Central admits at most twelve proofs. Keep room for the missing family.
        if proofs.len() == 11 && families.len() == 1 {
            proofs.pop();
        }
    }
    ensure!(
        proofs.len() >= 3 && families.len() >= 2,
        "candidate has only {} distinct routes in {} families: {families:?} ({wins} wins, {unclassified} unclassified; {stats:?})",
        proofs.len(),
        families.len()
    );
    let target_turn = proofs
        .iter()
        .map(|proof| proof.turn)
        .min()
        .context("winning turn")?;
    let package = Package {
        schema: 1,
        executor: baylee_build::short().into(),
        preset,
        target_turn,
    };
    package.validate(baylee_build::short())?;
    Ok(Certified {
        kind,
        package,
        deck,
        proofs,
    })
}

// A contributor set is one route even when its final damage source changes.
// Retain the alternate witness only when it adds a family without erasing another.
fn proof_slot(proofs: &[Proof], family: Family, signature: [u8; 32]) -> Option<usize> {
    let Some(index) = proofs
        .iter()
        .position(|proof| proof.strategy_digest == signature)
    else {
        return Some(proofs.len());
    };
    let previous = proofs[index].family;
    (!proofs.iter().any(|proof| proof.family == family)
        && proofs
            .iter()
            .filter(|proof| proof.family == previous)
            .count()
            > 1)
    .then_some(index)
}
