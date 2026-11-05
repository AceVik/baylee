//! The deals a net lost where the house, in the same seat, won: the games
//! that say most about what the net does wrong.

use std::path::Path;

use anyhow::{Context as _, ensure};
use serde_json::Value;

/// Which deals of a summary to take.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub(super) enum Which {
    /// The net lost where the house won.
    Lost,
    /// The net won where the house lost.
    Won,
    /// Every deal.
    All,
}

/// A deal of a summary.
pub(super) struct Deal {
    pub number: u64,
    pub seat: u64,
    pub decks: Vec<String>,
}

/// A deal's result for one player: won, lost, or neither.
fn result(utility: Option<f64>) -> Option<bool> {
    match utility {
        Some(u) if u > 0.75 => Some(true),
        Some(u) if u < 0.25 => Some(false),
        _ => None,
    }
}

/// The deals of eval4's `summary` that `which` selects.
pub(super) fn deals(summary: &Path, which: Which) -> anyhow::Result<Vec<Deal>> {
    let value: Value = serde_json::from_slice(
        &std::fs::read(summary).with_context(|| format!("reading {}", summary.display()))?,
    )?;
    ensure!(
        which == Which::All || value["baseline"].is_string(),
        "{} has no house baseline to compare the net with",
        summary.display()
    );
    let mut deals = Vec::new();
    for deal in value["deal_details"]
        .as_array()
        .context("the summary has no deal_details")?
    {
        let capped = deal["net_capped"].as_bool() == Some(true)
            || deal["base_capped"].as_bool() == Some(true);
        let net = result(deal["net_utility"].as_f64());
        let house = result(deal["base_utility"].as_f64());
        let taken = match which {
            Which::Lost => !capped && (net, house) == (Some(false), Some(true)),
            Which::Won => !capped && (net, house) == (Some(true), Some(false)),
            Which::All => true,
        };
        if taken {
            deals.push(Deal {
                number: deal["deal"].as_u64().context("a deal without its number")?,
                seat: deal["focal_seat"].as_u64().unwrap_or(0),
                decks: deal["decks"]
                    .as_array()
                    .map(|decks| {
                        decks
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_owned)
                            .collect()
                    })
                    .unwrap_or_default(),
            });
        }
    }
    Ok(deals)
}

/// Prints the deals `which` selects, with their seat and decks.
pub(super) fn list(summary: &Path, which: Which) -> anyhow::Result<()> {
    let deals = deals(summary, which)?;
    for deal in &deals {
        println!(
            "deal {:>5}  net seat P{}  {}",
            deal.number,
            deal.seat + 1,
            deal.decks.join("  vs  ")
        );
    }
    println!("{} deals ({which:?})", deals.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::result;

    #[test]
    fn a_draw_is_neither_won_nor_lost() {
        assert_eq!(result(Some(1.0)), Some(true));
        assert_eq!(result(Some(0.0)), Some(false));
        assert_eq!(result(Some(0.5)), None);
        assert_eq!(result(None), None);
    }
}
