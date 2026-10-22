//! `cards/enchantments/auras/mv_3/ritual_of_steel.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn ritual_of_steel() -> CardIndex {
    card_index("0af91b63-a874-4184-8f8f-3871429fd1ad")
}

/// Ritual of Steel (`{2}{W}`, `Coverage::Generated`): enchants a creature,
/// gives it +0/+2, and draws a card at the beginning of the next turn's upkeep.
#[test]
fn ritual_of_steel_pumps_toughness_and_draws_on_next_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains(), grizzly_bears()])
        .hand(0, &[ritual_of_steel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, p0, ritual_of_steel());
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bear), (2, 4));
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    pass_until(&mut engine, |e| {
        e.state().turn.active == p1
            && e.state().turn.step == crate::turn::Step::Draw
            && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
}
