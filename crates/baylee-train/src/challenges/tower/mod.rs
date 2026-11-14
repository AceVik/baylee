//! A complete five-floor graph of independently executed, replayed encounters.

mod deck;
mod encounter;
#[cfg(test)]
mod tests;

use super::{Certified, Generator, Limits};
use anyhow::{Context, Result, ensure};
use central_api::{
    challenges::{Kind, Period, Publication},
    tower::{self as api, Manifest, Theme},
};

/// Retained proof material for every reachable route; never a player-facing response.
pub struct CertifiedTower {
    /// Exact public choices and floor names.
    pub theme: Theme,
    /// Thirty-five certified nodes in canonical coordinate order.
    pub nodes: Vec<Certified>,
}

impl Generator {
    /// Certify one named variant for a focused calibration run, without publishing a graph.
    /// Candidate numbering matches the one-based progress report of [`Self::tower`].
    ///
    /// # Errors
    /// Invalid coordinates, missing card evidence or insufficient independently verified wins.
    pub fn tower_encounter(
        &self,
        period: Period,
        seed: u64,
        candidate: u8,
        node: u8,
        limits: Limits,
    ) -> Result<Certified> {
        ensure!(
            period.kind == Kind::Weekly && (1..=32).contains(&candidate),
            "invalid tower probe bounds"
        );
        period.validate()?;
        let counts = *api::coordinates()
            .get(usize::from(node))
            .context("unknown tower node")?;
        let floor = counts.iter().sum::<u8>() + 1;
        let theme = deck::theme(period);
        let pool = deck::opponent_pool(&self.working)?;
        let domain = [b"tower floor/".as_slice(), &[floor, candidate - 1]].concat();
        let seed = baylee_challenges::seed(&domain, seed);
        let (preset, text) = deck::build(&theme, counts, seed, &pool, &self.working)?;
        encounter::certify(&preset, &text, counts, limits)
    }

    /// Generate all five layers, searching a bounded seed budget independently per floor.
    /// A floor's opponent and shuffle seed remain identical across its upgrade variants.
    ///
    /// # Errors
    /// Invalid period, unsupported cards, exhausted certification budget or replay failure.
    pub fn tower(
        &self,
        period: Period,
        seed: u64,
        candidates: u8,
        limits: Limits,
    ) -> Result<CertifiedTower> {
        self.tower_with_progress(period, seed, candidates, limits, |_, _, _| {})
    }

    /// As [`Self::tower`], reporting only floor, one-based candidate and verified node count.
    /// Neither seeds nor hidden actions are passed to the progress observer.
    ///
    /// # Errors
    /// The same certification and bounded-search failures as [`Self::tower`].
    pub fn tower_with_progress(
        &self,
        period: Period,
        seed: u64,
        candidates: u8,
        limits: Limits,
        mut progress: impl FnMut(u8, u8, usize),
    ) -> Result<CertifiedTower> {
        ensure!(
            period.kind == Kind::Weekly && (1..=32).contains(&candidates),
            "invalid tower generation bounds"
        );
        period.validate()?;
        let theme = deck::theme(period);
        let pool = deck::opponent_pool(&self.working)?;
        let coordinates = api::coordinates();
        let mut nodes = Vec::with_capacity(api::NODES);
        for floor in 1..=api::FLOORS {
            let mut layer = None;
            let mut rejected = String::new();
            for candidate in 0..candidates {
                progress(floor, candidate + 1, 0);
                let domain = [b"tower floor/".as_slice(), &[floor, candidate]].concat();
                let seed = baylee_challenges::seed(&domain, seed);
                let mut verified = Vec::new();
                for &counts in coordinates
                    .iter()
                    .filter(|counts| counts.iter().sum::<u8>() == floor - 1)
                {
                    let result = deck::build(&theme, counts, seed, &pool, &self.working).and_then(
                        |(preset, text)| encounter::certify(&preset, &text, counts, limits),
                    );
                    let node = match result {
                        Ok(node) => node,
                        Err(error) => {
                            rejected = format!("node {counts:?}: {error}");
                            break;
                        }
                    };
                    verified.push(node);
                    progress(floor, candidate + 1, verified.len());
                }
                let expected = usize::from(floor) * usize::from(floor + 1) / 2;
                if verified.len() == expected {
                    layer = Some(verified);
                    break;
                }
            }
            nodes.extend(layer.with_context(|| {
                format!("floor {floor} exhausted its certification budget ({rejected})")
            })?);
        }
        ensure!(nodes.len() == api::NODES, "incomplete tower graph");
        Ok(CertifiedTower { theme, nodes })
    }
}

impl CertifiedTower {
    /// Bind the complete graph to its week. Packages are transported individually.
    ///
    /// # Errors
    /// Incomplete graph, invalid evidence or a public/private package mismatch.
    pub fn publications(&self) -> Result<(Manifest, Vec<Publication>)> {
        ensure!(self.nodes.len() == api::NODES, "incomplete tower graph");
        let mut publications = Vec::with_capacity(api::NODES);
        for (counts, certified) in api::coordinates().into_iter().zip(&self.nodes) {
            let floor = counts.iter().sum::<u8>();
            let mut publication = certified.publication(self.theme.period)?;
            publication.description.title = self.theme.floors[usize::from(floor)].clone();
            publication.description.expected_minutes = 12 + u16::from(floor) * 2;
            publication.description.instructions = deck::words(
                "Win this duel with your expedition deck. Your chosen improvements are already included. You start with seven supplied cards from your 60-card deck. A defeat costs one life; a victory opens the next upgrade. Every retry keeps this encounter's shuffle and opponent policy. You may pause and resume.",
                "Gewinne dieses Duell mit deinem Expeditionsdeck. Deine gewählten Verbesserungen sind bereits enthalten. Du beginnst mit sieben vorgegebenen Karten aus deinem 60-Karten-Deck. Eine Niederlage kostet ein Leben; ein Sieg eröffnet die nächste Verbesserung. Jeder Versuch behält Mischung und Gegnerstrategie dieser Begegnung. Du kannst pausieren und fortsetzen.",
            );
            publication.digest = publication.calculated_digest()?;
            publication.validate()?;
            publications.push(publication);
        }
        let manifest = Manifest {
            theme: self.theme.clone(),
            nodes: publications.iter().map(|p| p.digest).collect(),
        };
        manifest.validate()?;
        Ok((manifest, publications))
    }
}
