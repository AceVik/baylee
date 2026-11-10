//! `cards/creatures/mv_7/havenwood_wurm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn havenwood_wurm() -> CardIndex {
    card_index("a4ddff34-d0ae-481f-870b-b911b249250f")
}

/// Archweaver, a seven-mana creature without flash, the control in the next
/// test.
fn archweaver() -> CardIndex {
    card_index("2ceb76e2-1692-48fc-ab20-9587d1374172")
}

/// Havenwood Wurm — a 5/6 trampler with flash for {6}{G}.
#[test]
fn havenwood_wurm_costs_seven_and_is_a_five_six_trampler() {
    let mut engine = table7(&lots(forest(), 7), &[havenwood_wurm()], &[]);
    let wurm = cast_and_resolve(&mut engine, havenwood_wurm());
    assert_body(
        &engine,
        wurm,
        (5, 6),
        KeywordSet::FLASH.union(KeywordSet::TRAMPLE),
    );
}

/// Flash: during the opponent's upkeep, with seven mana in the pool, the Wurm
/// is offered and an Archweaver (same cost, no flash) is not; the Wurm
/// resolves onto the battlefield.
#[test]
fn havenwood_wurm_can_be_cast_on_their_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &lots(forest(), 7))
        .hand(0, &[havenwood_wurm(), archweaver()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    tap_all_mana(&mut engine, p0);
    assert!(castable7(&engine, havenwood_wurm()), "flash: any time");
    assert!(
        !castable7(&engine, archweaver()),
        "the control without flash is not castable on their turn"
    );
    cast_with_floating(&mut engine, p0, havenwood_wurm());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, havenwood_wurm()).is_some());
}
