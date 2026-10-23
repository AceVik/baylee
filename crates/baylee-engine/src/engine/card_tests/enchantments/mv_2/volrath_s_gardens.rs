//! `cards/enchantments/mv_2/volrath_s_gardens.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn volrath_s_gardens() -> CardIndex {
    card_index("ff99337e-410c-43af-9475-7995d690bcef")
}

/// Volrath's Gardens (`ff99337e-410c-43af-9475-7995d690bcef`) — {1}{G} Enchantment (Implemented):
/// "{2}, Tap an untapped creature you control: You gain 2 life. Activate only as a sorcery."
#[test]
fn volrath_s_gardens_taps_creature_to_gain_two_life_at_sorcery_speed() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                volrath_s_gardens(),
                index::GRIZZLY_BEARS,
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let gardens = on_battlefield(&engine, P0, volrath_s_gardens()).unwrap();
    let bear = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    tap_all_mana(&mut engine, P0);
    kit::activate_object(&mut engine, P0, gardens, 0);
    kit::settle(&mut engine, P0, |_, p| kit::choose_cards(p, &[bear]));

    assert!(entered_tapped(&engine, bear));
    assert_eq!(engine.state().players[0].life, 22);
}
