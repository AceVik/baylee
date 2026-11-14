//! Bounded human-library search without changing the floor's opponent or RNG.

use super::{Certified, Limits};
use anyhow::{Result, bail};
use baylee_core::preset::GamePreset;
use central_api::challenges::Kind;

pub(super) const DEALS: u8 = 8;

pub(super) fn certify(
    preset: &GamePreset,
    text: &str,
    counts: [u8; 3],
    limits: Limits,
) -> Result<Certified> {
    // First certify the canonical route before spending a deal-search budget on
    // its neighbours. A bad opponent candidate is rejected early, without
    // replaying the whole floor's graph for every possible deal.
    let deals = if counts[0] == 0 && counts[1] == 0 {
        1
    } else {
        DEALS
    };
    let mut rejected = String::new();
    for deal in 0..deals {
        let mut candidate = preset.clone();
        arrange(&mut candidate, counts, deal);
        match super::super::certify(Kind::Weekly, candidate, text.to_string(), limits, 5) {
            Ok(certified) => return Ok(certified),
            Err(error) => rejected = error.to_string(),
        }
    }
    bail!("{deals} certified-deal candidates exhausted ({rejected})")
}

pub(super) fn arrange(preset: &mut GamePreset, counts: [u8; 3], deal: u8) {
    if deal == 0 {
        return;
    }
    let domain = [b"tower human library/".as_slice(), &counts, &[deal]].concat();
    let seed = baylee_challenges::seed(&domain, preset.seed);
    // The ordinary engine still shuffles. Pre-permuting only this fixed-size
    // library changes neither the opponent's initial cards nor the number of
    // engine RNG draws. The winning arrangement becomes an immutable preset.
    baylee_engine::rng::GameRng::new(seed).shuffle(&mut preset.seats[0].deck);
}
