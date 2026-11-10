//! `cards/instants/mv_1/power_sink.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Power Sink: "Counter target spell unless its controller pays {X}." X is
/// the X Power Sink itself was cast with — two, not the one a card that
/// ignored its own announced X and printed a fixed tax would still show —
/// and the player asked to pay is the *targeted* spell's controller — p0,
/// who cast the Elves — and not Power Sink's own caster p1. p0 declines, so
/// the Elves are countered.
#[test]
fn power_sink_counters_the_targeted_spell_when_its_controller_declines_to_pay_x() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[llanowar_elves()])
        .battlefield(1, &[island(), island(), island()])
        .hand(1, &[power_sink()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    let elves = in_hand(&engine, p0, llanowar_elves()).expect("Elves in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: elves })
        .unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut engine, p1);
    let menu = cast_power_sink_at(&mut engine, p1, 2, elves);
    assert!(
        menu.contains(&elves),
        "\"counter target spell\" — any spell is a legal target: {menu:?}"
    );

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
        unreachable!("pass_until stopped on the tax question")
    };
    assert_eq!(mana, 2, "X = 2, the X Power Sink was cast with");
    assert_eq!(
        player, p0,
        "\"its controller\" — the targeted spell's controller, not Power Sink's own caster"
    );

    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "countered: the Elf never arrives"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "a countered spell is put into its owner's graveyard (CR 701.6a)"
    );
    assert!(
        in_graveyard(&engine, p1, power_sink()).is_some(),
        "a resolved instant goes to its owner's graveyard"
    );
}

/// The other half: p0 pays the {X} out of mana already floating from
/// casting their own spell, and the Elf resolves — paying is what Power
/// Sink's own text says keeps the countered spell alive.
#[test]
fn power_sink_lets_the_spell_resolve_when_its_controller_pays_x() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[llanowar_elves()])
        .battlefield(1, &[island(), island()])
        .hand(1, &[power_sink()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    let elves = in_hand(&engine, p0, llanowar_elves()).expect("Elves in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: elves })
        .unwrap();
    // Two Forests paid for a one-mana Elf: one green mana is still floating
    // when Power Sink's tax question comes, which is the whole point of
    // this board over the counterpart test's single Forest.
    let floating_before = engine.state().players[0].mana_pool.total();
    assert_eq!(
        floating_before, 1,
        "one Forest spent on the Elf, one left floating"
    );
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut engine, p1);
    cast_power_sink_at(&mut engine, p1, 1, elves);

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
        unreachable!("pass_until stopped on the tax question")
    };
    assert_eq!(mana, 1);
    assert_eq!(
        player, p0,
        "the payer is the targeted spell's controller, not Power Sink's caster"
    );

    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "paid: the spell is not countered and resolves"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_none(),
        "a resolved permanent spell is not in its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        floating_before - 1,
        "exactly the X was spent paying the tax"
    );
    assert!(
        in_graveyard(&engine, p1, power_sink()).is_some(),
        "a resolved instant goes to its owner's graveyard"
    );
}
