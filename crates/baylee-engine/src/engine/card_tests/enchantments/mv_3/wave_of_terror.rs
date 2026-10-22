//! `cards/enchantments/mv_3/wave_of_terror.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::counters::AGE;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

fn wave_of_terror() -> CardIndex {
    card_index("2346be19-7344-4f11-be2c-a22750cb7e1d")
}

/// `Wave of Terror` ({2}{B}, Enchantment, Partial) has cumulative upkeep `{1}`
/// and at the beginning of your draw step destroys each creature with mana
/// value equal to the number of age counters on Wave of Terror.
#[test]
fn wave_of_terror_destroys_creatures_matching_age_counter_count() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), HILL_GIANT])
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[wave_of_terror()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, wave_of_terror());
    pass_until(&mut engine, stack_is_empty);

    let wave = on_battlefield(&engine, p0, wave_of_terror()).expect("Wave of Terror");
    engine
        .dev_state_mut(p0)
        .unwrap()
        .object_mut(wave)
        .unwrap()
        .counters
        .set(AGE, 1);
    engine.refresh_offer();

    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::Upkeep
            && e.state().turn.active == p0
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    tap_all_mana(&mut engine, p0);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == p0),
    );
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain) && stack_is_empty(e)
    });

    assert!(on_battlefield(&engine, p0, wave_of_terror()).is_some());
    assert!(in_graveyard(&engine, p1, GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&engine, p0, HILL_GIANT).is_some());
}
