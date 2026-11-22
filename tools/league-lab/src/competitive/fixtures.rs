use rand::RngExt;
use rand_chacha::ChaCha8Rng;
use rand_distr::StandardNormal;

use super::queue::Ticket;
use crate::sim::DAYS;
#[derive(Clone, Copy)]
pub(super) struct Case {
    pub name: &'static str,
    pub n: usize,
    pub span: u32,
    pub seeds: u64,
}
pub(super) const CASES: [Case; 5] = [
    Case {
        name: "duel",
        n: 2,
        span: 75,
        seeds: 4,
    },
    Case {
        name: "night",
        n: 24,
        span: 1200,
        seeds: 6,
    },
    Case {
        name: "town",
        n: 160,
        span: 1800,
        seeds: 6,
    },
    Case {
        name: "city",
        n: 1600,
        span: 1800,
        seeds: 6,
    },
    Case {
        name: "peak",
        n: 5000,
        span: 1800,
        seeds: 3,
    },
];
pub(super) const DAY: u32 = 86_400;
pub(super) struct Pilot {
    pub skill: i32,
    pub deck: i32,
    slots: u8,
    pub format: usize,
    join: u16,
    leave: u16,
}
pub(super) fn normal(rng: &mut ChaCha8Rng, sd: f64) -> i32 {
    let z: f64 = rng.sample(StandardNormal);
    (z * sd).round() as i32
}
pub(super) fn pilots(case: Case, rng: &mut ChaCha8Rng) -> Vec<Pilot> {
    (0..case.n)
        .map(|id| {
            if case.n == 2 {
                return Pilot {
                    skill: [1150, 1850][id],
                    deck: 0,
                    slots: 10,
                    format: 0,
                    join: 0,
                    leave: DAYS,
                };
            }
            Pilot {
                skill: 1500 + normal(rng, 210.0),
                deck: 0,
                slots: match rng.random_range(0..100) {
                    0..35 => 2,
                    35..70 => 6,
                    _ => 10,
                },
                format: usize::from(rng.random_ratio(1, 4)),
                join: if rng.random_ratio(1, 8) {
                    rng.random_range(14..60)
                } else {
                    0
                },
                leave: if rng.random_ratio(1, 10) {
                    rng.random_range(240..330)
                } else {
                    DAYS
                },
            }
        })
        .collect()
}
/// Key the outcome draw by fixture and unordered IDs, independent of candidate
/// search order. Common opponents in the same slot see the same luck.
fn mix(mut x: u64) -> u64 {
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}
pub(super) fn uniform(seed: u64, slot: u32, ids: [usize; 2]) -> f64 {
    let a = ids[0].min(ids[1]) as u64;
    let b = ids[0].max(ids[1]) as u64;
    let bits = mix(mix(mix(seed) ^ u64::from(slot)) ^ mix((a << 32) | b)) >> 11;
    bits as f64 / 9_007_199_254_740_992.0
}
pub(super) fn arrivals(
    tickets: &mut Vec<Ticket>,
    pilots: &[Pilot],
    case: Case,
    day: u16,
    slot: u8,
    rng: &mut ChaCha8Rng,
) {
    tickets.clear();
    for (id, p) in pilots.iter().enumerate() {
        if p.join > day || p.leave <= day || slot >= p.slots {
            continue;
        }
        let at =
            u32::from(day) * DAY + u32::from(slot) * 7200 + 5 * rng.random_range(0..=case.span / 5);
        tickets.push(Ticket {
            id,
            at,
            deadline: at + 5 * rng.random_range(18..=48),
        });
    }
    tickets.sort_unstable_by_key(|t| (t.at, t.id));
}
