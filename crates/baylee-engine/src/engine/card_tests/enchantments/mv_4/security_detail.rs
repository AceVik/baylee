//! `cards/enchantments/mv_4/security_detail.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn security_detail() -> CardIndex {
    card_index("2a26dee4-b4b6-489d-99e5-6dbd8d19419d")
}

fn offered(engine: &Engine<RegistryLookup>, detail: ObjectId) -> bool {
    priority_offer(engine).abilities.contains(&(detail, 0))
}

/// "{W}{W}: Create a 1/1 white Soldier creature token. Activate only if you
/// control no creatures and only once each turn.": offered with no creature,
/// makes a 1/1 white Soldier, and is not offered again that turn, not even
/// after the Soldier is gone.
#[test]
fn security_detail_makes_one_soldier_when_i_control_no_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[security_detail()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let detail = on_battlefield(&engine, p0, security_detail()).expect("seated");

    assert!(!offered(&engine, detail), "no {{W}}{{W}} yet");
    float(&mut engine, p0, ManaColor::White, 2);
    assert!(offered(&engine, detail), "WW and no creature: offered");
    activate(&mut engine, p0, security_detail(), 0);
    pass_until(&mut engine, stack_is_empty);
    let soldiers = tokens_of(&engine, p0);
    assert_eq!(soldiers.len(), 1, "one Soldier");
    assert_eq!(pt(&engine, soldiers[0]), (1, 1));
    assert!(
        engine
            .state()
            .object(soldiers[0])
            .expect("token")
            .characteristics()
            .colors
            .contains(Color::White)
    );

    float(&mut engine, p0, ManaColor::White, 2);
    assert!(!offered(&engine, detail), "I control a creature now");
    crate::sba::destroy(engine.dev_state_mut(p0).expect("harness"), soldiers[0]);
    resync(&mut engine);
    assert!(tokens_of(&engine, p0).is_empty(), "the Soldier is gone");
    assert!(
        !offered(&engine, detail),
        "no creature again, but only once each turn"
    );
}

/// With a creature already out, the ability is withheld even with {W}{W}.
#[test]
fn security_detail_is_withheld_while_i_control_a_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[security_detail(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let detail = on_battlefield(&engine, p0, security_detail()).expect("seated");
    float(&mut engine, p0, ManaColor::White, 2);
    assert!(!offered(&engine, detail), "I control Bears");
}
