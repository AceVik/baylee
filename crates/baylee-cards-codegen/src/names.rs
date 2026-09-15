//! The name table: which card a printed English name resolves to, decided
//! when the code is generated.
//!
//! Resolving a name is what a *deck list* needs — the acceptance file, a
//! paste into the deck builder, a preset — and it was a linear scan over
//! the whole pool with a string compare per card, run once per row of every
//! deck the gateway stores. This places the names into a perfect hash
//! instead: [`baylee_core::phf`] holds the arithmetic, this builds the
//! table, and `baylee_cards::decks::by_name` reads it in O(1).
//!
//! # What it costs, and why the pool and not the corpus
//!
//! The arithmetic is 2 KB of displacements and 4 KB of slots, plus the
//! `(&str, CardIndex)` array — 1365 entries at 24 bytes, about 33 KB — whose
//! spellings are the same literals `baylee-cards` already compiles into
//! every `FaceDef`.
//!
//! That is the whole reason the table may sit in `baylee-cards`, which the
//! **engine links**, while a table over the 33 694-name corpus may not: those
//! names are not in that binary today, and putting them there is precisely
//! what the separate `baylee-cards-index` crate exists to prevent.
//!
//! # A name twice is refused here
//!
//! Two cards with one name have no answer this function could give, and the
//! perfect hash cannot be built over them at all — identical keys hash
//! identically and no displacement separates them, so the placement would
//! simply exhaust every seed and report nothing useful. It is checked first
//! and named instead: [`CodegenError::DuplicateCardName`].

use crate::error::CodegenError;
use baylee_core::phf;
use std::collections::HashMap;
use std::fmt::Write as _;

/// Where the seed search starts. Any value works; this one is fixed so the
/// rendered table is byte-reproducible.
const FIRST_SEED: u64 = 0x5178_c1b7_2722_0a95;

/// How many seeds to try before giving up.
const SEEDS: u64 = 64;

/// How far a bucket may be displaced before the seed is judged a bad one.
const MAX_DISPLACEMENT: u32 = 1_000_000;

/// Empty-slot marker in the slot table. The pool would have to reach 65 535
/// cards for this to collide with a real position.
pub const EMPTY: u16 = u16::MAX;

/// A placed key set.
#[derive(Debug, Clone)]
pub struct Placement {
    /// The seed that worked.
    pub seed: u64,
    /// One displacement per bucket.
    pub displacements: Vec<u32>,
    /// Slot → position in the key list, or [`EMPTY`].
    pub slots: Vec<u16>,
}

/// Places every key in a perfect hash.
///
/// Buckets are filled **largest first**, which is the whole trick: the
/// crowded buckets get their pick of a nearly empty table and the many
/// one-key buckets at the end fit almost anywhere. Ties are broken by bucket
/// index, so the order is total and the output is the same on every machine.
///
/// # Errors
/// [`CodegenError::DuplicateCardName`] if two keys have the same spelling,
/// [`CodegenError::NoPerfectHash`] if no seed placed them.
pub fn place(keys: &[&str]) -> Result<Placement, CodegenError> {
    let mut seen: HashMap<&str, usize> = HashMap::with_capacity(keys.len());
    for (i, key) in keys.iter().enumerate() {
        if let Some(&first) = seen.get(key) {
            return Err(CodegenError::DuplicateCardName {
                name: (*key).to_string(),
                first,
                again: i,
            });
        }
        seen.insert(key, i);
    }
    // A position has to fit in a u16 and stay clear of the empty marker.
    if keys.len() >= EMPTY as usize {
        return Err(CodegenError::PoolTooLarge {
            cards: keys.len(),
            limit: EMPTY as usize - 1,
        });
    }
    // Both powers of two: the slot index is taken with a mask, and four keys
    // to a bucket is the ratio CHD is usually run at.
    let slots = keys.len().next_power_of_two().max(2);
    let buckets = (slots / 4).max(1);

    for attempt in 0..SEEDS {
        let seed = FIRST_SEED.wrapping_add(attempt);
        if let Some(p) = try_seed(keys, seed, slots, buckets) {
            return Ok(p);
        }
    }
    Err(CodegenError::NoPerfectHash {
        keys: keys.len(),
        seeds: SEEDS,
    })
}

/// One attempt. `None` means a bucket could not be placed anywhere.
fn try_seed(keys: &[&str], seed: u64, slots: usize, buckets: usize) -> Option<Placement> {
    let mut in_bucket: Vec<Vec<usize>> = vec![Vec::new(); buckets];
    let mut parts: Vec<(u32, u32)> = Vec::with_capacity(keys.len());
    for (i, key) in keys.iter().enumerate() {
        let (bucket, base, step) = phf::parts(key.as_bytes(), seed, buckets);
        in_bucket[bucket].push(i);
        parts.push((base, step));
    }

    // Largest bucket first, then by bucket index — a total order, so the
    // table does not depend on how the hash map happened to iterate.
    let mut order: Vec<usize> = (0..buckets).collect();
    order.sort_by_key(|&b| (std::cmp::Reverse(in_bucket[b].len()), b));

    let mask = slots - 1;
    let mut placed = vec![EMPTY; slots];
    let mut displacements = vec![0u32; buckets];
    let mut scratch: Vec<usize> = Vec::new();

    for b in order {
        if in_bucket[b].is_empty() {
            continue;
        }
        let mut found = false;
        for d in 0..=MAX_DISPLACEMENT {
            scratch.clear();
            let fits = in_bucket[b].iter().all(|&i| {
                let (base, step) = parts[i];
                let slot = (base.wrapping_add(d.wrapping_mul(step)) as usize) & mask;
                // Free in the table, and not already claimed by a sibling
                // this same displacement sent to the same place.
                if placed[slot] != EMPTY || scratch.contains(&slot) {
                    return false;
                }
                scratch.push(slot);
                true
            });
            if fits {
                for (&i, &slot) in in_bucket[b].iter().zip(scratch.iter()) {
                    placed[slot] = u16::try_from(i).expect("the pool is far below 65 535 cards");
                }
                displacements[b] = d;
                found = true;
                break;
            }
        }
        if !found {
            return None;
        }
    }
    Some(Placement {
        seed,
        displacements,
        slots: placed,
    })
}

/// Header of the generated file. Its own paragraph, because the file is the
/// first thing anyone finds when they wonder where a name lookup went.
const HEADER: &str = "\
// GENERATED by `cargo xtask codegen` — do not edit by hand.
//
// Which card a printed English name resolves to, placed into a perfect hash
// when this file was written. `baylee_core::phf` is the arithmetic that put
// the names here and the arithmetic that finds them again — one definition,
// linked by the generator and by the reader, because two copies of a hash
// function drift and a drifted one answers `None` for every card.
//
// Source: the compiled pool, `baylee_cards::all()`.
#![allow(missing_docs, clippy::all, clippy::pedantic)]

use baylee_core::ids::CardIndex;
use baylee_core::phf;

";

/// Renders `crates/baylee-cards/src/generated_names.rs`.
///
/// The entries are sorted by name here rather than trusted to arrive in an
/// order, so the file is the same whatever order the pool was walked in.
///
/// # Errors
/// Whatever [`place`] refuses: a name twice, or a key set it could not
/// place.
pub fn render(entries: &[(&str, u32)]) -> Result<String, CodegenError> {
    let mut sorted: Vec<(&str, u32)> = entries.to_vec();
    sorted.sort_unstable();
    let keys: Vec<&str> = sorted.iter().map(|(name, _)| *name).collect();
    let placed = place(&keys)?;

    let mut out = String::with_capacity(sorted.len() * 48 + placed.slots.len() * 8);
    out.push_str(HEADER);

    let _ = write!(
        out,
        "/// The perfect hash over `NAMES`.\n\
         pub static TABLE: phf::Table = phf::Table {{\n    \
             displacements: &DISPLACEMENTS,\n    \
             slots: {},\n    \
             seed: {:#018x},\n\
         }};\n\n",
        placed.slots.len(),
        placed.seed
    );

    let _ = write!(
        out,
        "/// One displacement per bucket, in bucket order.\n\
         #[rustfmt::skip]\n\
         static DISPLACEMENTS: [u32; {}] = [\n",
        placed.displacements.len()
    );
    push_numbers(&mut out, &placed.displacements);
    out.push_str("];\n\n");

    let _ = write!(
        out,
        "/// The slot no name landed in.\n\
         pub const EMPTY: u16 = u16::MAX;\n\n\
         /// Slot → position in `NAMES`. [`EMPTY`] is a slot no name landed\n\
         /// in — every string hashes to *some* slot, so a name the pool does\n\
         /// not have arrives here too and this is where most of them stop.\n\
         #[rustfmt::skip]\n\
         pub static SLOTS: [u16; {}] = [\n",
        placed.slots.len()
    );
    push_numbers(&mut out, &placed.slots);
    out.push_str("];\n\n");

    let _ = write!(
        out,
        "/// Every name in the pool and the card it names, in name order.\n\
         ///\n\
         /// The spelling is kept beside the answer because a perfect hash is\n\
         /// perfect only over the keys it was built from: any other string\n\
         /// lands in some slot as well, and comparing what is written there\n\
         /// is the whole difference between `None` and the wrong card.\n\
         #[rustfmt::skip]\n\
         pub static NAMES: [(&str, CardIndex); {}] = [\n",
        sorted.len()
    );
    for (name, index) in &sorted {
        let _ = writeln!(out, "    ({name:?}, CardIndex::new({index})),");
    }
    out.push_str("];\n");
    Ok(out)
}

/// Sixteen to a line: long enough that the file stays short, short enough
/// that a line fits on a screen when someone goes looking.
fn push_numbers<T: std::fmt::Display>(out: &mut String, values: &[T]) {
    for chunk in values.chunks(16) {
        out.push_str("   ");
        for v in chunk {
            let _ = write!(out, " {v},");
        }
        out.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::{EMPTY, place, render};
    use crate::error::CodegenError;

    /// Every key finds its own slot back, and no two share one. That is the
    /// entire promise of the structure, so it is asserted over a set big
    /// enough to have crowded buckets.
    #[test]
    fn every_key_lands_in_a_slot_of_its_own() {
        let owned: Vec<String> = (0..2000).map(|i| format!("Card Number {i}")).collect();
        let keys: Vec<&str> = owned.iter().map(String::as_str).collect();
        let p = place(&keys).expect("placeable");
        let table = baylee_core::phf::Table {
            // Leaked because `Table` holds a `'static` slice and this is a
            // test: the process is about to end either way.
            displacements: Box::leak(p.displacements.clone().into_boxed_slice()),
            slots: p.slots.len(),
            seed: p.seed,
        };
        for (i, key) in keys.iter().enumerate() {
            let at = p.slots[table.slot(key.as_bytes())];
            assert_ne!(at, EMPTY, "{key} landed in an empty slot");
            assert_eq!(usize::from(at), i, "{key} landed on another key's slot");
        }
    }

    #[test]
    fn a_name_claimed_twice_is_refused_by_name() {
        let err = place(&["Forest", "Island", "Forest"]).expect_err("two Forests");
        match err {
            CodegenError::DuplicateCardName { name, first, again } => {
                assert_eq!(name, "Forest");
                assert_eq!((first, again), (0, 2));
            }
            other => panic!("wrong error: {other}"),
        }
    }

    /// The seed search and the bucket order are both fixed, so two runs over
    /// the same names produce the same bytes. Without that a `--check` in CI
    /// would fail on a table nobody changed.
    #[test]
    fn the_same_names_render_the_same_bytes() {
        let entries = [("Birds of Paradise", 7u32), ("Forest", 3), ("Mox Opal", 11)];
        assert_eq!(
            render(&entries).expect("rendered"),
            render(&entries).expect("rendered")
        );
    }

    /// The order the pool is walked in must not reach the file.
    #[test]
    fn the_order_the_names_arrive_in_does_not_matter() {
        let forwards = [("Birds of Paradise", 7u32), ("Forest", 3), ("Mox Opal", 11)];
        let backwards = [("Mox Opal", 11u32), ("Forest", 3), ("Birds of Paradise", 7)];
        assert_eq!(
            render(&forwards).expect("rendered"),
            render(&backwards).expect("rendered")
        );
    }

    #[test]
    fn a_name_with_a_quote_in_it_comes_out_as_valid_rust() {
        let out = render(&[("Ach! Hans, Run!", 1), ("Yawgmoth's Will", 2)]).expect("rendered");
        assert!(out.contains(r#"("Yawgmoth's Will", CardIndex::new(2))"#), "{out}");
    }
}
