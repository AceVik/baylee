//! `cards/enchantments/mv_4/oath_of_lim_dul.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn oath_of_lim_dul() -> CardIndex {
    card_index("64e163c2-2378-4e4c-aefe-5e0d2fd19988")
}

fn hand_size(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(seat)).len()
}

/// Oath of Lim-Dûl (64e163c2-2378-4e4c-aefe-5e0d2fd19988) — {3}{B} Enchantment:
/// "{B}{B}: Draw a card." (Coverage::Partial — life-loss trigger not supported.)
#[test]
fn oath_of_lim_dul_draws_a_card_for_two_black_mana() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[oath_of_lim_dul(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let hand_before = hand_size(&engine, P0);
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, oath_of_lim_dul(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        hand_size(&engine, P0),
        hand_before + 1,
        "activating {{B}}{{B}} on Oath of Lim-Dûl draws a card"
    );
}
