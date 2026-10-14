//! Transposition-invariant, directed-interval checks for both manuscript books.

use std::{collections::BTreeSet, path::Path};

use anyhow::{Context, Result, ensure};
use regex::Regex;
use serde::Deserialize;

const RUN: usize = 6;
const PHRASES: usize = 75;

#[derive(Deserialize)]
struct Tune {
    name: String,
    intervals: Vec<i16>,
    #[serde(default)]
    alt_intervals: Vec<Vec<i16>>,
}

fn melodies(text: &str) -> Result<Vec<(String, Vec<i16>)>> {
    let phrases = Regex::new(r"(?s)const ([A-Z_0-9]+): Phrase = &\[(.*?)\n\];")?;
    let notes = Regex::new(r"\((\d+), \d+\)")?;
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for phrase in phrases.captures_iter(text) {
        let name = phrase[1].to_owned();
        ensure!(seen.insert(name.clone()), "duplicate phrase {name}");
        let pitches = notes
            .captures_iter(&phrase[2])
            .map(|note| note[1].parse::<i16>().context("MIDI pitch"))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .filter(|&pitch| pitch != 0)
            .collect();
        out.push((name, pitches));
    }
    ensure!(
        out.len() == PHRASES,
        "expected {PHRASES} complete phrases, found {}",
        out.len()
    );
    Ok(out)
}

fn intervals(pitches: &[i16]) -> Vec<i16> {
    pitches.windows(2).map(|pair| pair[1] - pair[0]).collect()
}

// A single dynamic-programming row replaces repeatedly scanning every suffix.
fn longest_common_run(a: &[i16], b: &[i16]) -> usize {
    let mut previous = vec![0; b.len() + 1];
    let mut best = 0;
    for &interval in a {
        for j in (0..b.len()).rev() {
            previous[j + 1] = if interval == b[j] { previous[j] + 1 } else { 0 };
            best = best.max(previous[j + 1]);
        }
    }
    best
}

pub(super) fn run(root: &Path) -> Result<()> {
    let score = root.join("crates/baylee-client-core/src/music/score");
    let text = std::fs::read_to_string(score.join("manuscript.rs"))?
        + &std::fs::read_to_string(score.join("styles/manuscript.rs"))?;
    let tunes: Vec<Tune> =
        serde_json::from_slice(&std::fs::read(root.join("art/music/avoid.json"))?)?;
    let settings: Vec<_> = tunes
        .iter()
        .flat_map(|tune| {
            std::iter::once(&tune.intervals)
                .chain(&tune.alt_intervals)
                .map(move |setting| (&tune.name, setting))
        })
        .filter(|(_, setting)| !setting.is_empty())
        .collect();
    ensure!(
        !settings.is_empty(),
        "avoid.json contains no interval signatures"
    );
    let mut bad = 0;
    for (name, pitches) in melodies(&text)? {
        let ours = intervals(&pitches);
        let (run, tune) = settings
            .iter()
            .map(|(name, theirs)| (longest_common_run(&ours, theirs), *name))
            .max()
            .context("no comparison tunes")?;
        let flag = if run >= RUN {
            bad += 1;
            "MATCH"
        } else {
            "ok"
        };
        println!("{name:16} longest shared run {run} ({tune}) {flag}");
    }
    let (name, theirs) = settings[0];
    ensure!(
        longest_common_run(&theirs[..RUN.min(theirs.len())], theirs) >= RUN.min(theirs.len()),
        "injected opening was not caught"
    );
    println!("an injected opening of {name}: caught");
    ensure!(
        bad == 0,
        "{bad} melodies share at least {RUN} directed intervals with a listed tune"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directed_intervals_ignore_transposition_but_not_reversal() {
        let notes = [60, 64, 62, 67, 69, 66, 71];
        let transposed: Vec<_> = notes.iter().map(|pitch| pitch + 7).collect();
        let signature = intervals(&notes);
        assert_eq!(longest_common_run(&signature, &intervals(&transposed)), RUN);
        assert!(longest_common_run(&signature, &[-4, 2, -5, -2, 3, -5]) < RUN);
    }

    #[test]
    fn runs_are_contiguous_and_do_not_reuse_a_note() {
        assert_eq!(longest_common_run(&[1, 2, 3, 4], &[1, 2, 9, 3, 4]), 2);
        assert_eq!(longest_common_run(&[1], &[1, 1, 1]), 1);
        assert_eq!(longest_common_run(&[], &[1]), 0);
    }

    #[test]
    fn both_repository_books_have_75_unique_complete_phrases() {
        let text = concat!(
            include_str!("../../../baylee-client-core/src/music/score/manuscript.rs"),
            "\n",
            include_str!("../../../baylee-client-core/src/music/score/styles/manuscript.rs")
        );
        let phrases = melodies(text).unwrap();
        assert_eq!(phrases.len(), PHRASES);
        assert!(
            phrases
                .iter()
                .all(|(_, notes)| !notes.is_empty() && !notes.contains(&0))
        );
    }
}
