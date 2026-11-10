//! `cards/enchantments/auras/mv_1/capashen_standard.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Capashen Standard prints three lines: "Enchant creature", "Enchanted
/// creature gets +1/+1", and "{2}, Sacrifice this Aura: Draw a card". A
/// creature stands on each side of the table so the static can be read as
/// `Filter::AttachedToBySource` and not as "creatures you control" — and the
/// pump is read *again* after the sacrifice, where a host that kept the
/// counter would mean the Aura is still modifying something it let go of.
/// Both mana costs are paid inside one main phase: three Plains tap and the
/// {W} leaves exactly the {2} the second line charges, which is what makes
/// the ability offered at all rather than refused for want of mana.
#[test]
fn capashen_standard_pumps_only_its_host_then_sells_itself_for_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[capashen_standard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let host = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the Aura");

    // The Elves are kept back: its {T} is a mana ability in the pool's own
    // terms, and tapping it here would put green in the pool that neither
    // printed line accounts for.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, capashen_standard());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "`enchant creature` is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&theirs),
        "\"enchant creature\" names any creature, on either side of the \
         table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, capashen_standard()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host),
        "an Aura enters attached to the creature it targeted"
    );
    assert_eq!(
        pt(&engine, host),
        (2, 2),
        "the enchanted creature gets +1/+1"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the creature across the table is not the one this Aura holds"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "one of the three Plains paid the {{W}}, and the {{2}} is still \
         floating — read off the pool, which is what the offer below reads"
    );

    // Ability 0 is the enchant spell, 1 the static that pumps, 2 this.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(aura, 2)),
        "with the {{2}} in the pool the sacrifice ability is offered: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    activate(&mut engine, p0, capashen_standard(), 2);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, capashen_standard()).is_some(),
        "sacrificing the Aura is the other half of the cost"
    );
    assert!(
        on_battlefield(&engine, p0, capashen_standard()).is_none(),
        "and it left the battlefield to pay it"
    );
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "the +1/+1 went with the Aura, which is the whole of what \
         `Filter::AttachedToBySource` says"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"draw a card\" — one off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and into the hand, where a count off the library alone would not \
         have put it"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}} was paid"
    );
}
