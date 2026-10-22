//! `cards/enchantments/mv_2/legacy_s_allure.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn legacy_s_allure() -> CardIndex {
    card_index("106ca7bd-6dad-43fe-af58-8667ebddb2eb")
}

/// Legacy's Allure (`106ca7bd-6dad-43fe-af58-8667ebddb2eb`) — {U}{U} Enchantment (Partial):
/// "Sacrifice this enchantment: Gain control of target creature..."
#[test]
fn legacy_s_allure_sacrifices_itself_to_gain_control_of_target_creature() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[legacy_s_allure()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, legacy_s_allure());
    pass_until(&mut engine, stack_is_empty);

    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    activate(&mut engine, P0, legacy_s_allure(), 1);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, legacy_s_allure()).is_some());
    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_some());
}
