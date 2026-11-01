//! `cards/enchantments/mv_5/black_market.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "21338b71-37f5-4121-9b84-565025ebcd17"
fn black_market() -> CardIndex {
    card_index("21338b71-37f5-4121-9b84-565025ebcd17")
}

/// Black Market ({3}{B}{B} Enchantment):
/// "Whenever a creature dies, put a charge counter on this enchantment."
/// "At the beginning of your first main phase, add {B} for each charge counter on this enchantment."
#[test]
fn black_market_accumulates_charge_counters_on_creature_deaths_and_adds_black_mana_in_first_main() {
    let mut engine = lk::game(
        SEED,
        &[black_market(), index::GRIZZLY_BEARS],
        &[index::GRAY_OGRE],
        &[index::LIGHTNING_BOLT, index::TERROR],
        &[],
    );
    let market = lk::obj(&engine, P0, black_market());
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let ogre = lk::obj(&engine, P1, index::GRAY_OGRE);

    lk::float(
        &mut engine,
        P0,
        &[
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 1),
            (ManaColor::Black, 1),
        ],
    );
    lk::cast_at(&mut engine, P0, index::LIGHTNING_BOLT, &[bears], &[]);
    lk::cast_at(&mut engine, P0, index::TERROR, &[ogre], &[]);

    assert_eq!(
        engine
            .state()
            .object(market)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        2,
        "two creature deaths put two charge counters on Black Market"
    );

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(
        lk::pool_of(&engine, P1, ManaColor::Black),
        0,
        "Black Market does not trigger on the opponent's first main phase"
    );

    // Advance to P0's first main phase and let the triggered ability resolve.
    reach_their_main_phase(&mut engine, P0);
    lk::settle(&mut engine);

    assert_eq!(
        lk::pool_of(&engine, P0, ManaColor::Black),
        2,
        "Black Market added {{B}}{{B}} at the beginning of P0's first main phase"
    );
}
