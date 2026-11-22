/// One entry per global account across all ranked queues.
pub const NAMES: [&str; 10] = [
    "Papier",
    "Holz",
    "Eisen",
    "Bronze",
    "Silber",
    "Gold",
    "Diamant",
    "Master",
    "Grandmaster",
    "Champion",
];
pub const SEASON: u16 = 91;
pub const MASTER_AGE: u16 = 184;
pub const MASTER_GAMES: u16 = 1840;
pub const CHAMPION_LIMIT: usize = 100;
const LOWER: [(i32, u16); 6] = [
    (1150, 10),
    (1275, 30),
    (1400, 60),
    (1500, 120),
    (1625, 260),
    (1775, 600),
];

#[derive(Clone, Copy)]
pub struct Policy {
    pub name: &'static str,
    pub master: i32,
    pub grandmaster: i32,
    pub champion: i32,
}
pub const POLICIES: [Policy; 3] = [
    Policy {
        name: "open",
        master: 2060,
        grandmaster: 2190,
        champion: 2340,
    },
    Policy {
        name: "middle",
        master: 2140,
        grandmaster: 2290,
        champion: 2420,
    },
    Policy {
        name: "strict",
        master: 2200,
        grandmaster: 2350,
        champion: 2490,
    },
];

#[derive(Clone, Copy)]
pub struct Record {
    pub elo: i32,
    pub games: u16,
    pub first: u16,
    pub last: u16,
}
impl Default for Record {
    fn default() -> Self {
        Self {
            elo: 1500,
            games: 0,
            first: u16::MAX,
            last: 0,
        }
    }
}

impl Record {
    pub fn played(&mut self, day: u16, points: i32) {
        self.elo += points;
        self.games += 1;
        self.first = self.first.min(day);
        self.last = day;
    }
    fn tier(self, policy: Policy, day: u16) -> usize {
        if self.games >= MASTER_GAMES
            && day.saturating_sub(self.first) >= MASTER_AGE
            && day - self.last <= 14
            && self.elo >= policy.master
        {
            return if self.games >= 2500 && self.elo >= policy.grandmaster {
                8
            } else {
                7
            };
        }
        LOWER
            .iter()
            .rposition(|&(elo, games)| self.elo >= elo && self.games >= games)
            .map_or(0, |i| i + 1)
    }
}

pub struct Standings {
    pub tiers: Vec<usize>,
    pub candidates: usize,
}
/// The daily publication chooses all titleholders in one global snapshot.
/// Equal integer Elo uses stable account ID as the final ordering key.
pub fn standings(records: &[Record], policy: Policy, day: u16) -> Standings {
    let mut tiers = Vec::with_capacity(records.len());
    let mut candidates = Vec::new();
    for (id, record) in records.iter().enumerate() {
        let tier = record.tier(policy, day);
        tiers.push(tier);
        if tier == 8 && record.games >= 3000 && record.elo >= policy.champion {
            candidates.push((std::cmp::Reverse(record.elo), id));
        }
    }
    let eligible = candidates.len();
    if eligible > CHAMPION_LIMIT {
        candidates.select_nth_unstable(CHAMPION_LIMIT);
        candidates.truncate(CHAMPION_LIMIT);
    }
    for (_, id) in candidates {
        tiers[id] = 9;
    }
    Standings {
        tiers,
        candidates: eligible,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unbeaten_ten_matches_per_day_reach_master_on_day_185() {
        let mut player = Record::default();
        for day in 0..=MASTER_AGE {
            for _ in 0..10 {
                player.played(day, 12);
            }
            assert_eq!(player.tier(POLICIES[2], day) >= 7, day == MASTER_AGE);
        }
    }
    #[test]
    fn perfect_streak_still_needs_time_and_volume() {
        let mut r = Record {
            elo: 3000,
            games: 5000,
            first: 70,
            last: 253,
        };
        assert_eq!(r.tier(POLICIES[1], 253), 6);
        r.last = 254;
        assert_eq!(r.tier(POLICIES[1], 254), 8);
        r.games = 1839;
        assert_eq!(r.tier(POLICIES[1], 254), 6);
        r.games = 1840;
        assert_eq!(r.tier(POLICIES[1], 254), 7);
        assert_eq!(r.tier(POLICIES[1], 269), 6);
    }
    #[test]
    fn global_ties_displacement_and_vacancies() {
        let mut r = vec![
            Record {
                elo: 2700,
                games: 3000,
                first: 0,
                last: 350
            };
            140
        ];
        let table = standings(&r, POLICIES[1], 350);
        assert_eq!(table.candidates, 140);
        assert!(table.tiers[..100].iter().all(|&t| t == 9));
        assert!(table.tiers[100..].iter().all(|&t| t == 8));
        r[139].elo += 1;
        let table = standings(&r, POLICIES[1], 350);
        assert_eq!((table.tiers[139], table.tiers[99]), (9, 8));
        assert_eq!(standings(&r[..3], POLICIES[1], 350).tiers, vec![9; 3]);
        assert!(standings(&r, POLICIES[1], 365).tiers.iter().all(|&t| t < 7));
    }
}
