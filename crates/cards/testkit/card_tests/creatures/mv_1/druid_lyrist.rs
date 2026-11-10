//! `cards/creatures/mv_1/druid_lyrist.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Druid Lyrist prints one line — "{G}, {T}, Sacrifice this creature:
/// Destroy target enchantment" — and its three cost pieces are three
/// different kinds of thing: mana, the tap symbol, and the creature itself.
/// The mana has to be *floating* before the offer is asked for, because
/// `legal.abilities` is filtered through `can_afford` and that reads the pool
/// rather than the untapped Forest beside the Lyrist; and the sacrifice is
/// the piece that makes the Lyrist the price, which is why its own graveyard
/// is an assertion and not the board it left. The Elf standing beside the
/// enchantment across the table is the control on the filter: one target, and
/// "target enchantment" is not "target permanent".
#[test]
fn druid_lyrist_sacrifices_itself_to_destroy_an_enchantment() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), druid_lyrist()])
        .battlefield(1, &[fastbond(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lyrist = on_battlefield(&engine, p0, druid_lyrist()).expect("the Lyrist is out");
    let bond = on_battlefield(&engine, p1, fastbond()).expect("the enchantment is out");
    let elf = on_battlefield(&engine, p1, quiet_creature()).expect("the Elf is out");

    // Mana in the pool first: the offer is filtered through `can_afford`,
    // which reads the pool and not the untapped Forest (Regel 6).
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(lyrist, 0)),
        "with {{G}} floating the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, druid_lyrist(), 0);
    // Targets are chosen before costs are paid (CR 601.2c before 601.2h), so
    // the Lyrist is still standing while this question is open.
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "destroying an enchantment is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&bond),
        "the opponent's enchantment is a legal target across the table: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "\"target enchantment\" is not \"target permanent\": {options:?}"
    );
    assert!(
        !options.contains(&lyrist),
        "and not a creature either, the Lyrist itself included: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bond],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, fastbond()).is_none(),
        "the targeted enchantment left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, fastbond()).is_some(),
        "and was put into its *owner's* graveyard, which is the seat across the table"
    );
    assert!(
        on_battlefield(&engine, p0, druid_lyrist()).is_none(),
        "\"Sacrifice this creature\" is the third piece of the cost, so the Lyrist is gone"
    );
    assert!(
        in_graveyard(&engine, p0, druid_lyrist()).is_some(),
        "and lands in the graveyard of the seat that paid it"
    );
}
