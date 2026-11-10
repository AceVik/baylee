//! `cards/instants/mv_3/ferocious_charge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Ferocious Charge` is a `{2}{G}` Instant under `Coverage::Implemented`.
/// It gives target creature +4/+4 until end of turn and scries 2.
/// Casting it targets a friendly creature, prompting for the target upon casting,
/// and during resolution asks a `Pending::Arrange` with `ArrangePrompt::Scry` for the
/// scry 2 before pumping the creature from 1/1 to 5/5 until end of turn and moving to
/// the graveyard.
#[test]
fn ferocious_charge_pumps_target_creature_and_scries_two() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), llanowar_elves()])
        .hand(0, &[ferocious_charge()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("Llanowar Elves on battlefield");
    assert_eq!(pt(&engine, elf), (1, 1));

    cast_from_hand(&mut engine, p0, ferocious_charge());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets for Ferocious Charge, got {:?}",
            engine.pending()
        );
    };
    assert!(options.contains(&elf), "elf is a legal target");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::Arrange {
                prompt: ArrangePrompt::Scry,
                ..
            }
        )
    });

    let Pending::Arrange {
        player,
        cards,
        piles,
        prompt,
    } = engine.pending().clone()
    else {
        panic!("expected a scry arrangement, got {:?}", engine.pending());
    };
    assert_eq!(player, p0, "active player scries");
    assert_eq!(
        piles,
        scry_piles(2),
        "Scry 2 allows choosing 0 to 2 cards to bottom"
    );
    assert_eq!(prompt, ArrangePrompt::Scry);

    engine.apply(p0, look_answer(&cards, &[])).unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, elf),
        (5, 5),
        "target creature receives +4/+4 until end of turn"
    );
    assert!(
        in_graveyard(&engine, p0, ferocious_charge()).is_some(),
        "Ferocious Charge is in graveyard after resolution"
    );
}
