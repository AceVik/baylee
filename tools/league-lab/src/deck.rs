use std::collections::BTreeMap;

/// Sorted card identities, one entry per copy, across main and side together.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Pool(Vec<u32>);

impl Pool {
    pub fn registered(main: &[u32], side: &[u32]) -> Self {
        let mut cards: Vec<_> = main.iter().chain(side).copied().collect();
        assert!(
            !cards.is_empty(),
            "only a legal registered list can be ranked"
        );
        cards.sort_unstable();
        Self(cards)
    }

    pub fn distance(&self, other: &Self) -> f64 {
        let (mut i, mut j, mut shared) = (0, 0, 0);
        while i < self.0.len() && j < other.0.len() {
            match self.0[i].cmp(&other.0[j]) {
                std::cmp::Ordering::Less => i += 1,
                std::cmp::Ordering::Greater => j += 1,
                std::cmp::Ordering::Equal => {
                    i += 1;
                    j += 1;
                    shared += 1;
                }
            }
        }
        let size = self.0.len().max(other.0.len());
        (size - shared) as f64 / size as f64
    }
}

pub const FLOOR: i32 = 150;
pub fn start(rating: i32, share: f64, power: f64) -> i32 {
    let fraction = 0.2 * share + 0.8 * share.powf(power);
    rating - (f64::from(rating - FLOOR) * fraction).round() as i32
}

pub fn after_match(rating: i32, opponent_deck: i32, pilot_gap: i32, won: bool) -> i32 {
    let expected = 1.0 / (1.0 + 10_f64.powf(f64::from(opponent_deck - rating - pilot_gap) / 400.0));
    let delta = (24.0 * (f64::from(u8::from(won)) - expected)).round() as i32;
    (rating + delta).max(FLOOR)
}

#[derive(Clone, Copy)]
struct Version {
    rating: i32,
    season_games: u32,
}

/// One shared registry per format and match mode. Reference ratings are frozen
/// for an entire season. The caller validates deck legality before admission.
pub struct Registry {
    versions: BTreeMap<Pool, Version>,
    references: BTreeMap<Pool, i32>,
    power: f64,
}

impl Registry {
    pub fn new(power: f64) -> Self {
        Self {
            versions: BTreeMap::new(),
            references: BTreeMap::new(),
            power,
        }
    }

    pub fn register(&mut self, key: &Pool) -> i32 {
        if let Some(known) = self.versions.get(key) {
            return known.rating;
        }
        let closest = self
            .references
            .iter()
            .map(|(pool, &snapshot)| {
                let share = pool.distance(key);
                // A small edit cannot resurrect points the reference has since lost.
                let rating = snapshot.min(self.versions[pool].rating);
                (share, start(rating, share, self.power))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        let rating = closest.map_or(FLOOR, |(_, rating)| rating);
        self.versions.insert(
            key.clone(),
            Version {
                rating,
                season_games: 0,
            },
        );
        rating
    }

    /// A single fixture seat updates against a rated opponent snapshot and
    /// the difference between the pilots' Elo before the match.
    pub fn record_result(
        &mut self,
        key: &Pool,
        opponent_deck: i32,
        pilot_gap: i32,
        won: bool,
    ) -> i32 {
        let rating = self.register(key);
        let version = self.versions.get_mut(key).expect("registered deck");
        version.rating = after_match(rating, opponent_deck, pilot_gap, won);
        version.season_games += 1;
        version.rating
    }

    pub fn close_season(&mut self) {
        for (pool, version) in &mut self.versions {
            if version.season_games >= 40 {
                self.references.insert(pool.clone(), version.rating);
            }
            version.season_games = 0;
        }
    }

    /// Rehydrate an already established reference and its persisted rating.
    pub fn established(key: &Pool, rating: i32, power: f64) -> Self {
        let mut registry = Self::new(power);
        registry.references.insert(key.clone(), rating);
        registry.versions.insert(
            key.clone(),
            Version {
                rating,
                season_games: 0,
            },
        );
        registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn new_list_cannot_recover_a_references_recent_losses() {
        let a = Pool::registered(&[1, 2, 3, 4], &[]);
        let b = Pool::registered(&[1, 2, 3, 5], &[]);
        let mut book = Registry::established(&a, 1680, 2.0);
        let latest = book.record_result(&a, 1500, 0, false);
        assert!(latest < 1680);
        assert_eq!(book.register(&b), start(latest, 0.25, 2.0));
    }
    #[test]
    fn identity_copies_sizes_and_zones() {
        let old = Pool::registered(&[1, 1, 2], &[3]);
        let sideboarded = Pool::registered(&[3, 2, 1], &[1]);
        assert_eq!(old, sideboarded);
        assert_eq!(old.distance(&Pool::registered(&[1, 2], &[3, 4])), 0.25);
        assert_eq!(old.distance(&Pool::registered(&[1, 2], &[3])), 0.25);
        assert_eq!(start(1680, 0.0, 2.0), 1680);
        assert_eq!(start(1680, 1.0, 2.0), FLOOR);
    }
    #[test]
    fn intervening_games_and_season_boundary_do_not_launder_edits() {
        let a = Pool::registered(&[1, 2, 3, 4], &[]);
        let b = Pool::registered(&[1, 2, 3, 5], &[]);
        let c = Pool::registered(&[1, 2, 5, 6], &[]);
        let mut book = Registry::established(&a, 1680, 2.0);
        let early = book.register(&b);
        let earned = book.record_result(&b, 1680, 0, true);
        assert!(earned > early);
        book.close_season();
        assert_eq!(book.register(&c), start(1680, 0.5, 2.0));
        assert_eq!(book.register(&a), 1680);
        assert_eq!(book.register(&b), earned);
        assert_eq!(book.register(&Pool::registered(&[5, 2], &[1, 3])), earned);
    }
    #[test]
    fn new_anchor_requires_a_season_cut_and_forty_games_on_one_pool() {
        let a = Pool::registered(&[1, 2, 3, 4], &[]);
        let b = Pool::registered(&[1, 2, 3, 5], &[]);
        let c = Pool::registered(&[1, 2, 5, 6], &[]);
        let mut book = Registry::established(&a, 1680, 2.0);
        let mut earned = 0;
        for _ in 0..40 {
            earned = book.record_result(&b, 1680, 0, true);
        }
        book.close_season();
        assert_eq!(book.register(&c), start(earned, 0.25, 2.0));
        let mut empty = Registry::new(2.0);
        assert_eq!(empty.register(&a), FLOOR);
    }
}
