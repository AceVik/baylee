//! `cards/creatures/mv_2/mogg_toady.rs`, played.

use super::super::super::creature_count_combat as count_combat;
#[allow(clippy::wildcard_imports)]
use super::*;

fn mogg_toady() -> CardIndex {
    card_index("77c7ba06-d247-421d-9e0c-54c3000c1b78")
}

#[test]
fn mogg_toady_attacks_only_a_player_it_outnumbers() {
    count_combat::attack_compares_each_defender(mogg_toady());
}

#[test]
fn mogg_toady_blocks_only_when_it_outnumbers_the_attackers_controller() {
    count_combat::block_compares_actual_attackers_controller(mogg_toady());
}
