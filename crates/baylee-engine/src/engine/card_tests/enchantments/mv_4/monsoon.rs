//! `cards/enchantments/mv_4/monsoon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn monsoon() -> CardIndex {
    card_index("a86e7de8-8448-406c-b162-22c62bdc3813")
}

/// Monsoon (a86e7de8-8448-406c-b162-22c62bdc3813) — {2}{R}{G} Enchantment:
/// "At the beginning of each player's end step, tap all untapped Islands that player controls and this enchantment deals X damage to the player, where X is the number of Islands tapped this way."
#[test]
fn monsoon_taps_active_players_untapped_islands_at_each_end_step_and_deals_that_much_damage() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[monsoon(), island(), island(), island(), forest()])
        .battlefield(1, &[island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let p0_islands: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.controller == P0 && o.card.is_some_and(|c| c.index == island()))
        })
        .collect();
    assert_eq!(p0_islands.len(), 3);
    let p0_forest = on_battlefield(&engine, P0, forest()).unwrap();
    let p1_island = on_battlefield(&engine, P1, island()).unwrap();

    // P0 taps 1 of their 3 Islands in main phase so only 2 Islands remain untapped at P0's end step.
    tap_mana_where(&mut engine, P0, |id| id == p0_islands[0]);
    assert!(entered_tapped(&engine, p0_islands[0]));

    // Advance to P0's end step and let Monsoon's trigger resolve.
    walk_to_step_trigger(&mut engine, P0, Step::End);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        entered_tapped(&engine, p0_islands[1]) && entered_tapped(&engine, p0_islands[2]),
        "Monsoon taps P0's 2 untapped Islands"
    );
    assert!(
        !entered_tapped(&engine, p0_forest),
        "P0's non-Island land (Forest) is left untapped"
    );
    assert!(
        !entered_tapped(&engine, p1_island),
        "non-active player P1's Island is left untapped on P0's end step"
    );
    assert_eq!(
        life_of(&engine, P0),
        18,
        "Monsoon deals 2 damage to P0 for the 2 untapped Islands tapped this way (not 3 for all Islands and not 4 for the Forest)"
    );
    assert_eq!(life_of(&engine, P1), 20);

    // Advance to P1's end step: Monsoon triggers for P1 and taps P1's 1 untapped Island for 1 damage.
    walk_to_step_trigger(&mut engine, P1, Step::End);
    pass_until(&mut engine, stack_is_empty);

    assert!(entered_tapped(&engine, p1_island));
    assert_eq!(life_of(&engine, P1), 19);
}
