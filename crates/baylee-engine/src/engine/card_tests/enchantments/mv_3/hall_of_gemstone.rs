//! `cards/enchantments/mv_3/hall_of_gemstone.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HALL_OF_GEMSTONE;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Hall of Gemstone (18e4551f-9f6c-4421-ad66-a270df6d3463) — {1}{G}{G} World Enchantment:
/// "At the beginning of each player's upkeep, that player chooses a color.
/// Until end of turn, lands tapped for mana produce mana of the chosen color
/// instead of any other color."
#[test]
fn hall_of_gemstone_replaces_colored_land_mana_with_green_until_eot() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .battlefield(1, &[plains()])
        .hand(0, &[HALL_OF_GEMSTONE])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, HALL_OF_GEMSTONE);
    pass_until(&mut engine, stack_is_empty);

    // Advance to P1's main phase (resolving Hall of Gemstone's upkeep trigger).
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.phase == Phase::FirstMain
    });
    tap_all_mana(&mut engine, P1);
    assert_eq!(
        engine.state().players[1]
            .mana_pool
            .available(ManaColor::Green),
        1
    );
    assert_eq!(
        engine.state().players[1]
            .mana_pool
            .available(ManaColor::White),
        0
    );
}
