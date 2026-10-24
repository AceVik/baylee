use super::{BTreeMap, CardIndex, Digest, Sha256};
use std::cmp::Ordering;
use std::fmt::Write as _;

pub(super) fn compute_multiset(
    main: &[CardIndex],
    commanders: &[CardIndex],
) -> BTreeMap<CardIndex, u32> {
    let mut m = BTreeMap::new();
    for &c in main.iter().chain(commanders) {
        *m.entry(c).or_insert(0) += 1;
    }
    m
}

pub(super) fn compute_deck_hash(
    multiset: &BTreeMap<CardIndex, u32>,
    commanders: &[CardIndex],
) -> String {
    let mut hasher = Sha256::new();
    for (&cid, &cnt) in multiset {
        hasher.update(cid.get().to_le_bytes());
        hasher.update(cnt.to_le_bytes());
    }
    hasher.update(0xFF_u32.to_le_bytes());
    let mut cmds: Vec<u32> = commanders.iter().map(|c| c.get()).collect();
    cmds.sort_unstable();
    for c in cmds {
        hasher.update(c.to_le_bytes());
    }
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(32);
    for b in &digest[..16] {
        let _ = write!(hex, "{b:02x}");
    }
    hex
}

#[allow(
    clippy::cast_precision_loss,
    reason = "Approximate similarity ratios of bounded deck card counts"
)]
pub(super) fn multiset_jaccard(a: &BTreeMap<CardIndex, u32>, b: &BTreeMap<CardIndex, u32>) -> f64 {
    let mut inter = 0_u64;
    let mut union = 0_u64;
    let mut it_a = a.iter().peekable();
    let mut it_b = b.iter().peekable();
    while it_a.peek().is_some() || it_b.peek().is_some() {
        match (it_a.peek(), it_b.peek()) {
            (Some(&(&ka, &va)), Some(&(&kb, &vb))) => match ka.cmp(&kb) {
                Ordering::Equal => {
                    inter += u64::from(va.min(vb));
                    union += u64::from(va.max(vb));
                    it_a.next();
                    it_b.next();
                }
                Ordering::Less => {
                    union += u64::from(va);
                    it_a.next();
                }
                Ordering::Greater => {
                    union += u64::from(vb);
                    it_b.next();
                }
            },
            (Some(&(_, &va)), None) => {
                union += u64::from(va);
                it_a.next();
            }
            (None, Some(&(_, &vb))) => {
                union += u64::from(vb);
                it_b.next();
            }
            (None, None) => break,
        }
    }
    if union == 0 {
        0.0
    } else {
        inter as f64 / union as f64
    }
}

pub(super) fn find(p: &mut [usize], mut x: usize) -> usize {
    while p[x] != x {
        p[x] = p[p[x]];
        x = p[x];
    }
    x
}
