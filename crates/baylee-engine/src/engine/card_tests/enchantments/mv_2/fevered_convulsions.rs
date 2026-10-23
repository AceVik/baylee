//! `cards/enchantments/mv_2/fevered_convulsions.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn fevered_convulsions() -> CardIndex {
    card_index("83f35293-39e9-4c78-9c9f-25a1fc905676")
}

/// Fevered Convulsions (`83f35293-39e9-4c78-9c9f-25a1fc905676`) — {B}{B} Enchantment:
/// "{2}{B}{B}: Put a -1/-1 counter on target creature."
#[test]
fn fevered_convulsions_puts_minus_one_minus_one_counter_on_target_creature() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[fevered_convulsions(), swamp(), swamp(), swamp(), swamp()],
        )
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, fevered_convulsions(), 0);
    let source = on_battlefield(&engine, P0, fevered_convulsions()).unwrap();
    let land = on_battlefield(&engine, P0, swamp()).unwrap();
    let menu = aim_at(&mut engine, P0, bears);
    assert!(!menu.contains(&source));
    assert!(!menu.contains(&land));
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .counters
            .get(CounterKind::M1M1),
        1
    );
    assert_eq!(pt(&engine, bears), (1, 1));
}
