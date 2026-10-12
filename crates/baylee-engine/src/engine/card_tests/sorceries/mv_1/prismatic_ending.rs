//! `cards/sorceries/mv_1/prismatic_ending.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::choice::NumberPrompt;

/// The same {2}{W} paid with three Plains is one color. The target was
/// legal — the condition is not part of the targeting — so the spell
/// resolves and the three-drop stays.
#[test]
fn prismatic_ending_of_one_color_leaves_a_three_drop() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let engine = prismatic_ending_at(&[plains(), plains(), plains()], 2, prismatic_three_drop());
    assert!(on_battlefield(&engine, p1, prismatic_three_drop()).is_some());
    assert!(in_graveyard(&engine, p0, prismatic_ending()).is_some());
}

/// The exact bound, with nothing left to tap. Seat 0 floats one {W} off its
/// only Plains and casts `{X}{W}` at seat 1's Llanowar Elves (mana value 1).
/// No source remains to make more, so the pool is all that pays: one {W}
/// leaves nothing for X, and the question offers X = 0 and no more. The
/// question that offered the pool's size, 1, let X = 1 through and sent the
/// caster back to priority at payment with the spell uncast. At X = 0 the one
/// color spent is enough for a mana value of 1, and the Elves are exiled.
///
/// A guard: it passes both before and after the change that left X open
/// while a source remains (the test below), because no source remains here.
#[test]
fn prismatic_ending_off_one_floating_white_is_announced_at_zero_and_exiles_a_one_drop() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[prismatic_ending()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let target = on_battlefield(&engine, p1, llanowar_elves()).expect("seat 1's Elves");
    let only_plains = all_on_battlefield(&engine, p0, plains())[0];
    tap_mana_where(&mut engine, p0, |id| id == only_plains);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one {{W}} floating, no Plains left to tap"
    );

    cast_with_floating(&mut engine, p0, prismatic_ending());
    let Pending::ChooseNumber {
        min,
        max,
        reason: NumberPrompt::X,
        ..
    } = engine.pending().clone()
    else {
        panic!("X is announced, got {:?}", engine.pending())
    };
    assert_eq!(
        (min, max),
        (0, 0),
        "the {{W}} is the fixed part, X gets none"
    );
    assert!(
        engine.apply(p0, PlayerAction::ChooseNumber(1)).is_err(),
        "X = 1 is more than one floating {{W}} pays for"
    );

    engine
        .apply(p0, PlayerAction::ChooseNumber(0))
        .expect("the one X offered");
    let _ = aim_at(&mut engine, p0, target);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, prismatic_ending()).is_some(),
        "the spell was cast and resolved"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "one color spent reaches a mana value of 1"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p1))
            .iter()
            .any(|&id| engine
                .state()
                .object(id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == llanowar_elves()))),
        "exiled, not destroyed"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}

/// The report, as the rules have it (CR 601.2b, 601.2g). Seat 0 floats one {W}
/// off the first of three Plains and casts `{X}{W}` at seat 1's Llanowar
/// Elves (mana value 1) with two Plains still untapped. The question is open:
/// the caster may announce more than floats, because the untapped Plains can
/// still make the mana. X = 2 is announced, the Elves named, and the payment
/// window opens for the `{2}{W}` the pool cannot cover. Two Plains tapped in
/// it and a pass cast the spell with X = 2 and an empty pool; spent on white
/// alone, the one color reaches a mana value of 1 and the Elves are exiled.
/// The version that capped X at the one floating {W} offered 0 and gave the
/// caster no way to announce 2.
#[test]
fn prismatic_ending_announces_x_beyond_the_floating_white_and_pays_it_in_the_window() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[prismatic_ending()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let target = on_battlefield(&engine, p1, llanowar_elves()).expect("seat 1's Elves");
    let plains_ids = all_on_battlefield(&engine, p0, plains());
    tap_mana_where(&mut engine, p0, |id| id == plains_ids[0]);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one {{W}} floating, two Plains still untapped"
    );

    cast_with_floating(&mut engine, p0, prismatic_ending());
    let Pending::ChooseNumber {
        min,
        max,
        reason: NumberPrompt::X,
        ..
    } = engine.pending().clone()
    else {
        panic!("X is announced, got {:?}", engine.pending())
    };
    assert_eq!(
        (min, max),
        (0, u32::MAX),
        "two untapped Plains can still make the mana for X"
    );
    engine
        .apply(p0, PlayerAction::ChooseNumber(2))
        .expect("an X above what floats is an answer");
    let _ = aim_at(&mut engine, p0, target);
    assert!(
        matches!(engine.payment_window(), Some((seat, _)) if seat == p0),
        "the payment window opens for the {{2}}{{W}} the pool cannot cover: {:?}",
        engine.pending()
    );
    assert!(
        on_stack(&engine, prismatic_ending()).is_none(),
        "not cast yet"
    );

    let paying = [plains_ids[1], plains_ids[2]];
    assert_eq!(
        tap_mana_where(&mut engine, p0, |id| paying.contains(&id)),
        2,
        "both remaining Plains make their mana in the window"
    );
    engine
        .apply(p0, PlayerAction::PassPriority)
        .expect("three white pay {2}{W}");
    let spell = on_stack(&engine, prismatic_ending()).expect("the spell was cast");
    assert_eq!(engine.state().object(spell).map(|o| o.x_value), Some(2));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, prismatic_ending()).is_some(),
        "the spell resolved"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "one color spent reaches a mana value of 1"
    );
}
