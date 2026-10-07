//! `cards/creatures/mv_2/gilded_drake.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Gilded Drake prints flying and one trigger: "When this creature enters,
/// exchange control of this creature and up to one target creature an
/// opponent controls. If you don't or can't make an exchange, sacrifice this
/// creature." Both branches are played, because a test that only swapped
/// would pass on a card that never had the sacrifice clause: the exchange
/// moves two permanents between boards without either one changing zone, and
/// declining the `up to one` target — the trigger's `min` is zero — costs the
/// Drake its life instead.
#[test]
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
fn gilded_drake_exchanges_control_or_sacrifices_itself_when_the_exchange_is_declined() {
    // Branch one: the exchange really happens.
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(90210, island())
        .battlefield(0, &[island(), island()])
        .battlefield(1, &[quiet_creature()])
        .hand(0, &[gilded_drake()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let their_elf = on_battlefield(&engine, p1, quiet_creature()).expect("their Elf is out");
    cast_from_hand(&mut engine, p0, gilded_drake());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing else")
    };
    assert_eq!(player, p0, "the Drake's controller names the exchange");
    assert_eq!(
        (min, max),
        (0, 1),
        "\"up to one target\" is min zero: the exchange may be declined"
    );
    let drake = on_battlefield(&engine, p0, gilded_drake()).expect("the Drake entered");
    assert_eq!(
        options,
        vec![their_elf],
        "the creature an opponent controls, and only it"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_elf],
            },
        )
        .expect("the Elf the trigger offered is a legal exchange");
    pass_until(&mut engine, stack_is_empty);

    // Neither card changed zones: the Drake is the same object under the
    // opponent's control and the Elf is the same object under ours, which is
    // the whole of what "exchange control" says.
    assert_eq!(
        on_battlefield(&engine, p1, gilded_drake()),
        Some(drake),
        "the Drake is the opponent's now, and the very same object"
    );
    assert!(
        on_battlefield(&engine, p0, gilded_drake()).is_none(),
        "and no longer ours, without ever having left the battlefield"
    );
    assert_eq!(
        on_battlefield(&engine, p0, quiet_creature()),
        Some(their_elf),
        "the creature it was exchanged for is under our control"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_creature()).is_none(),
        "so its old controller no longer holds it: one permanent per side"
    );

    // Branch two: declining. A separate board, because the first one's Drake
    // belongs to the opponent by now and the clause under test is the one the
    // card pays for with its own life.
    let mut declined = Duel::new(90210, island())
        .battlefield(0, &[island(), island()])
        .battlefield(1, &[quiet_creature()])
        .hand(0, &[gilded_drake()])
        .start();
    keep_mulligans(&mut declined);
    assert!(
        walk_to_own_main(&mut declined, p0),
        "p0 reaches its own main at the second table too"
    );

    cast_from_hand(&mut declined, p0, gilded_drake());
    pass_until(&mut declined, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { min, options, .. } = declined.pending().clone() else {
        unreachable!("pass_until stops on nothing else")
    };
    assert_eq!(min, 0, "\"up to one target\": nothing is a legal answer");
    assert!(
        !options.is_empty(),
        "and the menu is a real one, so the decline below is a choice rather \
         than an empty offer: {options:?}"
    );

    declined
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .expect("declining the exchange is a legal answer");
    pass_until(&mut declined, stack_is_empty);

    assert!(
        on_battlefield(&declined, p0, gilded_drake()).is_none(),
        "\"If you don't … make an exchange, sacrifice this creature\" — no \
         trade means no Drake"
    );
    assert!(
        in_graveyard(&declined, p0, gilded_drake()).is_some(),
        "and a sacrificed creature goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&declined, p1, quiet_creature()).is_some(),
        "the creature the exchange never named is still where it was"
    );
}
