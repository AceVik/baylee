//! `cards/enchantments/mv_4/pyrohemia.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn pyrohemia() -> CardIndex {
    card_index("9ac57a10-3402-4656-9079-f713884cde35")
}

/// "{R}: This enchantment deals 1 damage to each creature and each player":
/// a 2/2 survives with one damage marked, both players lose 1, Pyrohemia
/// stays.
#[test]
fn pyrohemia_pings_each_creature_and_each_player_for_red() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[pyrohemia(), grizzly_bears()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    float(&mut engine, p0, ManaColor::Red, 1);

    activate(&mut engine, p0, pyrohemia(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p0), 19, "I take 1");
    assert_eq!(life_of(&engine, p1), 19, "so does the opponent");
    for seat in [p0, p1] {
        let bears = on_battlefield(&engine, seat, grizzly_bears()).expect("2/2 survives 1");
        assert_eq!(
            engine.state().object(bears).expect("object").damage,
            1,
            "one damage marked on each creature, whoever controls it"
        );
    }
    assert!(on_battlefield(&engine, p0, pyrohemia()).is_some());
}

/// "At the beginning of the end step, if no creatures are on the
/// battlefield, sacrifice this enchantment": with a creature out it stays.
#[test]
fn pyrohemia_stays_at_the_end_step_while_a_creature_is_out() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[pyrohemia()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert!(
        stack_is_empty(&engine),
        "the intervening if is false: no trigger"
    );
    assert!(on_battlefield(&engine, p0, pyrohemia()).is_some());
}

/// With no creature anywhere, the end step trigger sacrifices it.
#[test]
fn pyrohemia_is_sacrificed_at_the_end_step_with_no_creatures() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[pyrohemia()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert!(!stack_is_empty(&engine), "the trigger waits on the stack");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, pyrohemia()).is_some(),
        "no creatures: sacrificed"
    );
}
