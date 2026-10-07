//! `cards/sorceries/mv_6/zof_consumption.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Zof Consumption` // `Zof Bloodbog` (`Coverage::Implemented`): "Each opponent loses 4 life and
/// you gain 4 life. // This land enters tapped. {T}: Add {B}."
///
/// Under `Coverage::Implemented`, casting `Zof Consumption` causes each opponent to lose 4 life
/// via `Effect::LoseLife` and the caster to gain 4 life via `Effect::gain_life`. The spell has no
/// targets and resolves directly, leaving the card in the caster's graveyard.
#[test]
fn zof_consumption_drains_opponent_for_four_life_and_caster_gains_four() {
    // Only the caster is named: both life totals are read off the seat list
    // by index below, which is what the card is about.
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(984, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[swamp()])
        .hand(0, &[zof_consumption()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert_eq!(
        engine.state().players[0].life,
        20,
        "caster starts at 20 life"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "opponent starts at 20 life"
    );

    tap_all_mana(&mut engine, p0);
    cast_front_face(&mut engine, p0, zof_consumption());

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        16,
        "opponent lost 4 life from Zof Consumption"
    );
    assert_eq!(
        engine.state().players[0].life,
        24,
        "caster gained 4 life from Zof Consumption"
    );
    assert!(
        in_graveyard(&engine, p0, zof_consumption()).is_some(),
        "the sorcery card is in the graveyard after resolving"
    );
}
