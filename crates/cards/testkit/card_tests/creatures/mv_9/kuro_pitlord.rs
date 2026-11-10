//! `cards/creatures/mv_9/kuro_pitlord.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn kuro() -> CardIndex {
    card_index("c80a7d22-076a-4fd7-bcf4-4769143647f0")
}

/// Kuro is a 9/9 for {6}{B}{B}{B}.
#[test]
fn kuro_is_cast_for_nine_as_a_nine_nine() {
    let mut engine = at_main(&[swamp(); 9], &[kuro()]);
    let demon = cast_and_settle(&mut engine, kuro());
    assert_eq!(pt(&engine, demon), (9, 9));
}

fn upkeep(pay: bool) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest()).battlefield(0, &[kuro()]).start();
    keep_mulligans(&mut engine);
    give_mana(&mut engine, p0, ManaColor::Black, 4);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(pay)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    engine
}

/// "At the beginning of your upkeep, sacrifice Kuro unless you pay
/// {B}{B}{B}{B}": paying keeps him and spends all four.
#[test]
fn kuro_stays_when_four_black_are_paid() {
    let engine = upkeep(true);
    let p0 = PlayerId::new(0);
    assert!(on_battlefield(&engine, p0, kuro()).is_some());
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        0
    );
}

/// Declining sacrifices him and spends nothing.
#[test]
fn kuro_is_sacrificed_when_the_upkeep_is_declined() {
    let engine = upkeep(false);
    let p0 = PlayerId::new(0);
    assert!(on_battlefield(&engine, p0, kuro()).is_none());
    assert!(in_graveyard(&engine, p0, kuro()).is_some());
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        4
    );
}

/// "Pay 1 life: Target creature gets -1/-1 until end of turn": two payments
/// shrink the opponent's Bears to 1/1 and then kill it.
#[test]
fn kuro_pays_life_to_shrink_a_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[kuro()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    give_mana(&mut engine, p0, ManaColor::Black, 4);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain)
            && e.state().turn.active == p0
            && stack_is_empty(e)
    });
    let bear = on_battlefield(&engine, p1, grizzly_bears()).expect("their bear");
    let life = life_of(&engine, 0);

    activate(&mut engine, p0, kuro(), 1);
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bear), (1, 1));
    assert_eq!(life_of(&engine, 0), life - 1);

    activate(&mut engine, p0, kuro(), 1);
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p1, grizzly_bears()).is_none(),
        "0/0 dies"
    );
    assert_eq!(life_of(&engine, 0), life - 2);
}
