//! `cards/instants/mv_2/mana_leak.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mana Leak: "Counter target spell unless its controller pays {3}."
/// Seat 0 casts Llanowar Elves; Seat 1 responds by casting Mana Leak.
/// When asked to pay {3} for the tax, Seat 0 declines, causing the creature spell to be countered.
#[test]
fn mana_leak_counters_spell_when_tax_is_declined() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(55, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[llanowar_elves()])
        .battlefield(1, &[island(), island()])
        .hand(1, &[mana_leak()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    let elves = in_hand(&engine, p0, llanowar_elves()).expect("Llanowar Elves in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: elves })
        .unwrap();

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    let leak = in_hand(&engine, p1, mana_leak()).expect("Mana Leak in hand");
    engine
        .apply(p1, PlayerAction::CastSpell { card: leak })
        .unwrap();

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Mana Leak, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(options, vec![elves]);
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayTax { mana },
        ..
    } = engine.pending().clone()
    else {
        panic!("expected PayTax prompt, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert_eq!(mana, 3);

    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "countered creature never arrives"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "countered creature is in graveyard"
    );
    assert!(
        in_graveyard(&engine, p1, mana_leak()).is_some(),
        "resolved Mana Leak is in graveyard"
    );
}
