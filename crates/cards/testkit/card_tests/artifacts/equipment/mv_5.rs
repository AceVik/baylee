//! The tests of the cards under `cards/artifacts/equipment/mv_5/`, one module per card.

#[allow(unused_imports)] // the rig the Equipment of `mv_2` share
use super::mv_2::{
    Rig, colors_of, equipped_gets, equipped_stays_without, goblin_piker, jasmine_boreal, rig,
    rig_vs, steel_wall, subtypes_of,
};
#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

mod pariah_s_shield;
