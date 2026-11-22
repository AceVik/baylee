use crate::league::{DIAMOND, GOLD, Gate, NAMES, PLATINUM, REVISED, Record};
use crate::sim::{DAYS, Scenario};

#[derive(Clone, Copy)]
struct Layout {
    below_gold: u8,
    from_gold: u8,
}
const THREE: Layout = Layout {
    below_gold: 3,
    from_gold: 3,
};
const FOUR: Layout = Layout {
    below_gold: 4,
    from_gold: 4,
};
const SELECTED: (Layout, i32) = (
    Layout {
        below_gold: 3,
        from_gold: 4,
    },
    15,
);
const VARIANTS: [(Layout, i32); 5] = [(THREE, 0), (FOUR, 0), (THREE, 15), (FOUR, 15), SELECTED];
const ROMAN: [&str; 4] = ["I", "II", "III", "IV"];
const BANDS: [&str; 3] = ["Holz-Silber", "Gold-Platin", "Diamant"];

#[cfg(test)]
#[path = "rank_art_tests.rs"]
mod art_tests;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct Rank {
    tier: usize,
    step: usize,
}

struct Scale {
    layout: Layout,
    buffer: i32,
    steps: [Vec<Gate>; NAMES.len()],
}

impl Scale {
    fn new(layout: Layout, buffer: i32) -> Self {
        let steps = std::array::from_fn(|tier| {
            if !(1..=DIAMOND).contains(&tier) {
                return Vec::new();
            }
            let (elo, games) = REVISED.gate(tier);
            let (next_elo, next_games) = REVISED.gate(tier + 1);
            let divisions = if tier < GOLD {
                layout.below_gold
            } else {
                layout.from_gold
            };
            (0..divisions)
                .map(|step| {
                    let elo_span = u32::try_from(next_elo - elo).expect("thresholds increase");
                    let offset = (elo_span * u32::from(step)).div_ceil(u32::from(divisions));
                    (
                        elo + i32::try_from(offset).expect("within the next gate"),
                        games
                            + ((next_games - games) * u16::from(step))
                                .div_ceil(u16::from(divisions)),
                    )
                })
                .collect()
        });
        Self {
            layout,
            buffer,
            steps,
        }
    }
    fn publish(&self, tier: usize, record: Record, previous: Rank) -> Rank {
        let gates = &self.steps[tier];
        let mut step = gates
            .iter()
            .rposition(|&(elo, games)| record.elo >= elo && record.games >= games)
            .unwrap_or(0);
        if tier == previous.tier && step < previous.step {
            // Buffer only the downward crossing of an already held division.
            step = gates[..=previous.step]
                .iter()
                .rposition(|&(elo, games)| record.elo >= elo - self.buffer && record.games >= games)
                .unwrap_or(0);
        }
        Rank { tier, step }
    }
}

pub fn print_ladder() {
    let scale = Scale::new(SELECTED.0, SELECTED.1);
    println!("rank,min_elo,min_games,additional_rule");
    println!("Papier,,0,");
    for (tier, name) in NAMES.iter().enumerate().skip(1) {
        if tier <= DIAMOND {
            for (i, (elo, games)) in scale.steps[tier].iter().enumerate() {
                let numeral = ROMAN[scale.steps[tier].len() - 1 - i];
                println!("{name} {numeral},{elo},{games},");
            }
        } else {
            let (elo, games) = REVISED.gate(tier);
            let extra = if tier == crate::league::CHAMPION {
                "age>=184d active<=14d global top 100"
            } else {
                "age>=184d active<=14d"
            };
            println!("{name},{elo},{games},{extra}");
        }
    }
}

#[derive(Default)]
struct Counts {
    days: u64,
    up: u64,
    down: u64,
    reversals: u64,
}

pub struct Comparison {
    scale: Scale,
    previous: Vec<Rank>,
    two_days_ago: Vec<Rank>,
    counts: [Counts; 3],
}

impl Comparison {
    pub fn selected(n: usize) -> Self {
        Self::new(SELECTED, n)
    }
    pub fn movement(&self) -> (u64, u64, u64) {
        self.counts
            .iter()
            .fold((0, 0, 0), |(days, changes, back), c| {
                (days + c.days, changes + c.up + c.down, back + c.reversals)
            })
    }
    pub fn variants(n: usize) -> [Self; VARIANTS.len()] {
        VARIANTS.map(|layout| Self::new(layout, n))
    }
    fn new((divisions, buffer): (Layout, i32), n: usize) -> Self {
        Self {
            scale: Scale::new(divisions, buffer),
            previous: vec![Rank::default(); n],
            two_days_ago: vec![Rank::default(); n],
            counts: std::array::from_fn(|_| Counts::default()),
        }
    }
    pub fn update(&mut self, tiers: &[usize], records: &[Record], day: u16) {
        for (id, (&tier, &record)) in tiers.iter().zip(records).enumerate() {
            let previous = self.previous[id];
            let today = self.scale.publish(tier, record, previous);
            if day >= DAYS - 28
                && record.last == day
                && (1..=DIAMOND).contains(&tier)
                && tier == previous.tier
            {
                let group = if tier < GOLD {
                    0
                } else if tier <= PLATINUM {
                    1
                } else {
                    2
                };
                let count = &mut self.counts[group];
                count.days += 1;
                count.up += u64::from(today.step > previous.step);
                count.down += u64::from(today.step < previous.step);
                count.reversals += u64::from(today != previous && today == self.two_days_ago[id]);
            }
            self.two_days_ago[id] = previous;
            self.previous[id] = today;
        }
    }
    pub fn rows(self, cfg: Scenario) -> Vec<String> {
        self.counts
            .into_iter()
            .zip(BANDS)
            .map(|(c, band)| {
                format!(
                    "{},{},{},{},{},{},{band},{},{},{},{}",
                    cfg.label,
                    cfg.seed,
                    cfg.population,
                    self.scale.layout.below_gold,
                    self.scale.layout.from_gold,
                    self.scale.buffer,
                    c.days,
                    c.up,
                    c.down,
                    c.reversals
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn division_gates_require_both_elo_and_games() {
        for (divisions, buffer) in VARIANTS {
            let scale = Scale::new(divisions, buffer);
            for tier in 1..=DIAMOND {
                for (step, &(elo, games)) in scale.steps[tier].iter().enumerate().skip(1) {
                    let mut record = Record {
                        elo,
                        games,
                        ..Record::default()
                    };
                    assert_eq!(scale.publish(tier, record, Rank::default()).step, step);
                    record.elo -= 1;
                    assert_eq!(scale.publish(tier, record, Rank::default()).step, step - 1);
                    record.elo += 1;
                    record.games -= 1;
                    assert_eq!(scale.publish(tier, record, Rank::default()).step, step - 1);
                }
            }
        }
    }
    #[test]
    fn buffer_only_preserves_an_earned_division_within_its_league() {
        let scale = Scale::new(SELECTED.0, SELECTED.1);
        let mut r = Record {
            elo: 1720,
            games: 1000,
            ..Record::default()
        };
        let held = scale.publish(GOLD, r, Rank::default());
        assert_eq!(held.step, 1);
        r.elo = 1705;
        assert_eq!(scale.publish(GOLD, r, held).step, 1);
        assert_eq!(scale.publish(GOLD, r, Rank::default()).step, 0);
        r.elo -= 1;
        assert_eq!(scale.publish(GOLD, r, held).step, 0);
        r.elo = 1679;
        let demoted = scale.publish(GOLD - 1, r, held);
        assert_eq!((demoted.tier, demoted.step), (GOLD - 1, 2));
        assert_eq!(scale.publish(0, r, demoted).step, 0);
    }
}
