//! Fixed deck workload construction.
use super::{Game, SEEDS, variant};

/// The Tower executor's fixed set: its own starters and relics, which no house
/// deck is (`variant::tower_games`).
#[cfg(feature = "tower")]
pub(super) fn games(four: bool) -> anyhow::Result<Vec<Game>> {
    variant::tower_games(four, &SEEDS)
}

/// The fixed set; with `four`, each table's two decks twice around it.
#[cfg(not(feature = "tower"))]
pub(super) fn games(four: bool) -> anyhow::Result<Vec<Game>> {
    use anyhow::Context as _;
    use baylee_cards::decks::{LoadedDeck, preset_for_all};
    use baylee_core::preset::{AIProfile, SeatController};
    let mut games = Vec::new();
    for (a, b, profile_a, profile_b) in super::TABLES {
        let decks = [deck(a)?, deck(b)?];
        let profiles = [profile_a, profile_b]
            .map(|name| AIProfile::named(name).with_context(|| format!("profile {name}")));
        let [profile_a, profile_b] = profiles;
        let profiles = [profile_a?, profile_b?];
        for seed in SEEDS {
            let chairs: &[&LoadedDeck] = if four {
                &[&decks[0], &decks[1], &decks[0], &decks[1]]
            } else {
                &[&decks[0], &decks[1]]
            };
            let mut preset = preset_for_all(seed, chairs);
            for (seat, profile) in preset.seats.iter_mut().zip(profiles.iter().cycle()) {
                seat.controller = SeatController::Ai(*profile);
                seat.team = None;
            }
            variant::prepare(&mut preset);
            games.push(Game {
                id: format!("{a}-{b}-{seed}"),
                preset,
            });
        }
    }
    Ok(games)
}

/// The house deck `key`, from the checkout this was built in.
#[cfg(not(feature = "tower"))]
fn deck(key: &str) -> anyhow::Result<baylee_cards::decks::LoadedDeck> {
    use anyhow::Context as _;
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/decks")
        .join(format!("{key}.txt"));
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    baylee_cards::decks::house_deck(&text)
        .map_err(anyhow::Error::msg)
        .with_context(|| format!("loading {}", path.display()))
}
