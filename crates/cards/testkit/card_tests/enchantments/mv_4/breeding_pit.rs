//! `cards/enchantments/mv_4/breeding_pit.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn breeding_pit() -> CardIndex {
    card_index("ddc83b35-cf7c-495b-bd30-e409b622e299")
}

/// "At the beginning of your upkeep, sacrifice this enchantment unless you
/// pay {B}{B}": the price is {B}{B}, paying keeps the Pit, and at my end step
/// it makes a 0/1 black Thrull.
#[test]
fn breeding_pit_paid_stays_and_makes_a_thrull_at_the_end_step() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[breeding_pit(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);

    let (asked, cost) = walk_to_price(&mut engine);
    assert_eq!(asked, p0, "the Pit's controller is asked");
    assert_eq!(cost, baylee_core::mana::ManaCost::parse("{B}{B}"));
    pay_the_price_with(&mut engine, p0, |_| true);
    assert!(
        on_battlefield(&engine, p0, breeding_pit()).is_some(),
        "paid, so the Pit stays"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "no Thrull before the end step"
    );

    walk_to_step_trigger(&mut engine, p0, Step::End);
    pass_until(&mut engine, stack_is_empty);
    let thrulls = tokens_of(&engine, p0);
    assert_eq!(thrulls.len(), 1, "one Thrull");
    assert_eq!(pt(&engine, thrulls[0]), (0, 1), "a 0/1");
    assert!(
        engine
            .state()
            .object(thrulls[0])
            .expect("token")
            .characteristics()
            .colors
            .contains(Color::Black),
        "a black Thrull"
    );
}

/// Declined, the Pit is sacrificed and makes no Thrull.
#[test]
fn breeding_pit_unpaid_is_sacrificed() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[breeding_pit(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);

    walk_to_price(&mut engine);
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, breeding_pit()).is_some(),
        "unpaid: sacrificed"
    );
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert!(tokens_of(&engine, p0).is_empty(), "no Pit, no Thrull");
}
