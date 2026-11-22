/// One entry per global account across all ranked queues.
pub const NAMES: [&str; 11] = [
    "Papier",
    "Holz",
    "Eisen",
    "Bronze",
    "Silber",
    "Gold",
    "Platin",
    "Diamant",
    "Master",
    "Grandmaster",
    "Champion",
];
pub const SEASON: u16 = 91;
pub const MASTER_AGE: u16 = 184;
pub const CHAMPION_LIMIT: usize = 100;
pub const GOLD: usize = 5;
pub const PLATINUM: usize = 6;
pub const DIAMOND: usize = 7;
pub const MASTER: usize = 8;
pub const GRANDMASTER: usize = 9;
pub const CHAMPION: usize = 10;
pub type Gate = (i32, u16);

#[derive(Clone, Copy)]
pub struct Policy {
    pub name: &'static str,
    // Papier has no gate. The index is the public tier ID minus one.
    pub gates: &'static [Option<Gate>; CHAMPION],
}
pub const LEGACY: Policy = Policy {
    name: "legacy",
    gates: &[
        Some((1150, 10)),
        Some((1275, 30)),
        Some((1400, 60)),
        Some((1500, 120)),
        Some((1625, 260)),
        None,
        Some((1775, 600)),
        Some((2200, 1840)),
        Some((2350, 2500)),
        Some((2490, 3000)),
    ],
};
pub const REVISED: Policy = Policy {
    name: "platinum",
    gates: &[
        Some((1200, 20)),
        Some((1320, 60)),
        Some((1440, 120)),
        Some((1560, 240)),
        Some((1680, 440)),
        Some((1840, 800)),
        Some((2000, 1240)),
        Some((2240, 2000)),
        Some((2350, 2500)),
        Some((2490, 3000)),
    ],
};
pub const POLICIES: [Policy; 2] = [LEGACY, REVISED];

impl Policy {
    pub fn gate(self, tier: usize) -> Gate {
        self.gates[tier - 1].expect("tier exists in this policy")
    }
}

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
    fn meets(self, (elo, games): Gate) -> bool {
        self.elo >= elo && self.games >= games
    }
    fn tier(self, policy: Policy, day: u16) -> usize {
        if self.meets(policy.gate(MASTER))
            && day.saturating_sub(self.first) >= MASTER_AGE
            && day - self.last <= 14
        {
            return if self.meets(policy.gate(GRANDMASTER)) {
                GRANDMASTER
            } else {
                MASTER
            };
        }
        policy.gates[..DIAMOND]
            .iter()
            .rposition(|gate| gate.is_some_and(|g| self.meets(g)))
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
        if tier == GRANDMASTER && record.meets(policy.gate(CHAMPION)) {
            candidates.push((std::cmp::Reverse(record.elo), id));
        }
    }
    let eligible = candidates.len();
    if eligible > CHAMPION_LIMIT {
        candidates.select_nth_unstable(CHAMPION_LIMIT);
        candidates.truncate(CHAMPION_LIMIT);
    }
    for (_, id) in candidates {
        tiers[id] = CHAMPION;
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
    fn unbeaten_ten_matches_per_day_reach_master_on_day_200() {
        let mut player = Record::default();
        for day in 0..200 {
            for _ in 0..10 {
                player.played(day, 12);
            }
            assert_eq!(player.tier(REVISED, day) >= MASTER, day == 199);
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
        assert_eq!(r.tier(REVISED, 253), DIAMOND);
        r.last = 254;
        assert_eq!(r.tier(REVISED, 254), GRANDMASTER);
        r.games = 1999;
        assert_eq!(r.tier(REVISED, 254), DIAMOND);
        r.games = 2000;
        assert_eq!(r.tier(REVISED, 254), MASTER);
        assert_eq!(r.tier(REVISED, 269), DIAMOND);
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
        let table = standings(&r, REVISED, 350);
        assert_eq!(table.candidates, 140);
        assert!(table.tiers[..100].iter().all(|&t| t == CHAMPION));
        assert!(table.tiers[100..].iter().all(|&t| t == GRANDMASTER));
        r[139].elo += 1;
        let table = standings(&r, REVISED, 350);
        assert_eq!((table.tiers[139], table.tiers[99]), (CHAMPION, GRANDMASTER));
        assert_eq!(standings(&r[..3], REVISED, 350).tiers, vec![CHAMPION; 3]);
        assert!(
            standings(&r, REVISED, 365)
                .tiers
                .iter()
                .all(|&t| t < MASTER)
        );
    }
    #[test]
    fn each_gate_requires_elo_and_experience_and_platinum_has_its_own_slot() {
        for tier in 1..=DIAMOND {
            let (elo, games) = REVISED.gate(tier);
            let mut r = Record {
                elo,
                games,
                first: 0,
                last: 300,
            };
            assert_eq!(r.tier(REVISED, 300), tier);
            r.games -= 1;
            assert_eq!(r.tier(REVISED, 300), tier - 1);
            r.elo -= 1;
            r.games += 1;
            assert_eq!(r.tier(REVISED, 300), tier - 1);
        }
        assert_eq!(NAMES[DIAMOND - 1], "Platin");
        assert!(LEGACY.gates[DIAMOND - 2].is_none());
    }
}
