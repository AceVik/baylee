//! Restrict the prepared order to effects that can name one object.
//!
//! Ranks refer to the complete, already dependency-sorted plan. Merging the
//! global and per-object ranks preserves that order, including timestamp ties.

use crate::effects::{EffectFilter, EffectTable};
use baylee_core::ids::ObjectId;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
pub(super) struct EffectIndex {
    global: Vec<u32>,
    objects: BTreeMap<ObjectId, Vec<u32>>,
}

impl EffectIndex {
    pub(super) fn build(effects: &EffectTable, ordered: &[u32]) -> Option<Self> {
        // Small plans are cheaper to walk than to index. Trigger storms have
        // hundreds of ObjectIs effects, each touching exactly one permanent.
        if effects.len() < 32 {
            return None;
        }
        let mut index = Self::default();
        for (rank, &at) in ordered.iter().enumerate() {
            let ranks = match effects.as_slice()[at as usize].filter {
                EffectFilter::ObjectIs(id, _) => index.objects.entry(id).or_default(),
                EffectFilter::Dsl(_)
                | EffectFilter::DslAbout(..)
                | EffectFilter::DslControlledBy(..) => &mut index.global,
            };
            ranks.push(u32::try_from(rank).expect("effect plan fits u32"));
        }
        (!index.objects.is_empty()).then_some(index)
    }

    pub(super) fn may_affect(&self, id: ObjectId) -> bool {
        !self.global.is_empty() || self.objects.contains_key(&id)
    }

    pub(super) fn ranks(&self, id: ObjectId, start: u32, end: u32) -> (&[u32], &[u32]) {
        let local = self.objects.get(&id).map_or(&[][..], Vec::as_slice);
        (within(&self.global, start, end), within(local, start, end))
    }
}

fn within(ranks: &[u32], start: u32, end: u32) -> &[u32] {
    let begin = ranks.partition_point(|&rank| rank < start);
    let len = ranks[begin..].partition_point(|&rank| rank < end);
    &ranks[begin..begin + len]
}

/// A stable merge of two disjoint subsequences of the complete order.
pub(super) enum Candidates<'a> {
    All(std::slice::Iter<'a, u32>),
    Indexed {
        ordered: &'a [u32],
        global: &'a [u32],
        local: &'a [u32],
    },
}

impl Iterator for Candidates<'_> {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::All(iter) => iter.next().copied(),
            Self::Indexed {
                ordered,
                global,
                local,
            } => {
                let take_global = match (global.first(), local.first()) {
                    (Some(g), Some(l)) => g < l,
                    (Some(_), None) => true,
                    (None, Some(_)) => false,
                    (None, None) => return None,
                };
                let ranks = if take_global { global } else { local };
                let (&rank, rest) = ranks.split_first()?;
                *ranks = rest;
                Some(ordered[rank as usize])
            }
        }
    }
}
