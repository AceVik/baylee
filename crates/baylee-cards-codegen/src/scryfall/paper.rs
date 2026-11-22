//! Paper availability aggregated over Scryfall printings by Oracle ID.

use super::{CodegenError, USER_AGENT, bulk_uri};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Duration;

const PAPER_IDS_FILE: &str = ".paper-oracle-ids";
const FRESH_FOR: Duration = Duration::from_hours(24);
const FEED: &str = "default_cards";

/// Returns the Oracle IDs with at least one printing whose `games` lists
/// `paper`. `default_cards` contains English printings, so the default
/// printing of an Oracle card need not itself be paper.
///
/// The cache stores just the IDs, for a day. Only tools proposing or
/// creating new pool admissions need this snapshot.
///
/// # Errors
/// If the snapshot needs a refresh and the bulk feed or cache cannot be read
/// or the new cache cannot be written.
pub fn paper_oracle_ids(
    agent: &ureq::Agent,
    cache_dir: &Path,
) -> Result<BTreeSet<String>, CodegenError> {
    let path = cache_dir.join(PAPER_IDS_FILE);
    if fs::metadata(&path)
        .and_then(|metadata| metadata.modified())
        .is_ok_and(|modified| modified.elapsed().is_ok_and(|age| age < FRESH_FOR))
    {
        let cached = fs::read_to_string(&path).map_err(CodegenError::io(&path))?;
        if !cached.is_empty() {
            return Ok(cached.lines().map(str::to_owned).collect());
        }
    }

    let url = bulk_uri(agent, FEED).map_err(|message| CodegenError::Http {
        url: "https://api.scryfall.com/bulk-data".to_owned(),
        message,
    })?;
    eprintln!("scryfall: refreshing paper availability from {FEED}");
    let response = agent
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .call()
        .map_err(|error| CodegenError::Http {
            url: url.clone(),
            message: error.to_string(),
        })?;
    let decoder = flate2::read::GzDecoder::new(response.into_body().into_reader());
    let paper = read_paper_ids(BufReader::new(decoder))
        .map_err(|message| CodegenError::Http { url, message })?;

    fs::create_dir_all(cache_dir).map_err(CodegenError::io(cache_dir))?;
    let tmp = path.with_extension(format!("{}.part", std::process::id()));
    let text = paper
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&tmp, text).map_err(CodegenError::io(&tmp))?;
    fs::rename(&tmp, &path).map_err(CodegenError::io(&path))?;
    Ok(paper)
}

fn read_paper_ids(reader: impl BufRead) -> Result<BTreeSet<String>, String> {
    #[derive(Deserialize)]
    struct Printing<'a> {
        oracle_id: Option<&'a str>,
        games: Vec<&'a str>,
    }

    let mut paper = BTreeSet::new();
    for line in reader.lines() {
        let line = line.map_err(|error| error.to_string())?;
        let line = line.trim().trim_end_matches(',');
        if line.is_empty() || line == "[" || line == "]" {
            continue;
        }
        let printing: Printing<'_> =
            serde_json::from_str(line).map_err(|error| error.to_string())?;
        if let Some(id) = printing
            .oracle_id
            .filter(|_| printing.games.contains(&"paper"))
        {
            paper.insert(id.to_owned());
        }
    }
    if paper.is_empty() {
        return Err("bulk feed contains no paper printings with an Oracle ID".to_owned());
    }
    Ok(paper)
}

#[cfg(test)]
mod tests {
    use super::read_paper_ids;

    #[test]
    fn a_paper_printing_counts_even_when_other_printings_are_digital() {
        let feed = br#"
{"oracle_id":"mixed","games":["arena"]}
{"oracle_id":"digital","games":["mtgo","arena"]}
{"oracle_id":"mixed","games":["paper","mtgo"]}
{"oracle_id":"paper-only","games":["paper"]}
{"games":["paper"]}
"#;
        let paper = read_paper_ids(feed.as_slice()).unwrap();
        assert_eq!(
            paper.into_iter().collect::<Vec<_>>(),
            ["mixed", "paper-only"]
        );
    }
}
