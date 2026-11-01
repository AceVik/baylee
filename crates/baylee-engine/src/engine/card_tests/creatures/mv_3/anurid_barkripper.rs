//! `cards/creatures/mv_3/anurid_barkripper.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn anurid_barkripper_is_two_two_below_threshold_and_four_four_at_seven_graveyard_cards() {
    let barkripper = card_index("2cdd85c8-fa60-44ca-bc96-d38f029d2184");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), swamp()])
        .hand(0, &[barkripper, dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 6);
    for f in all_on_battlefield(&engine, p0, forest()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: f })
            .unwrap();
    }
    let card = in_hand(&engine, p0, barkripper).unwrap();
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let frog = on_battlefield(&engine, p0, barkripper).expect("Anurid Barkripper on battlefield");
    assert_eq!(pt(&engine, frog), (2, 2));

    // Cast Dark Ritual as the 7th card entering the graveyard.
    cast_from_hand(&mut engine, p0, dark_ritual());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, frog), (4, 4));
}
