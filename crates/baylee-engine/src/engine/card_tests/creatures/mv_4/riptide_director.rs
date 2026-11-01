//! `cards/creatures/mv_4/riptide_director.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Riptide Director (`f7ae67c7-9005-4806-8130-5895dcd9879a`) — {2}{U}{U} Creature — Human Wizard 2/3:
/// "{2}{U}{U}, {T}: Draw a card for each Wizard you control."
#[test]
fn riptide_director_draws_a_card_for_each_wizard_you_control() {
    let director_card = card_index("f7ae67c7-9005-4806-8130-5895dcd9879a");
    let p0 = PlayerId::new(0);

    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                director_card,
                index::COVERT_OPERATIVE,
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_all_mana_but(&mut engine, p0, None);
    activate(&mut engine, p0, director_card, 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 2
    );
}
