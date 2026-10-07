//! `cards/creatures/mv_3/devout_witness.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Devout Witness — {2}{W}, a 2/2 Human Spellshaper: "{1}{W}, {T}, Discard a
/// card: Destroy target artifact or enchantment."
///
/// Three parts of one price are read in one activation, each in a different
/// place: the {1}{W} out of a pool the four Plains actually filled, the tap
/// symbol on the Witness itself, and the discarded card in its owner's
/// graveyard. The filter is the other half, and it needs bystanders on both
/// sides — the offer must hold the artifact *and* the enchantment standing
/// across the table while declining the Elf between them, because a creature
/// is neither one. Only the artifact is named, so exactly one permanent leaves
/// the battlefield and the enchantment is what says "target" is singular.
#[test]
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
fn devout_witness_discards_and_taps_to_destroy_the_artifact_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), devout_witness()],
        )
        .battlefield(
            1,
            &[quiet_artifact(), their_enchantment(), quiet_creature()],
        )
        .hand(0, &[festering_goblin()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let witness = on_battlefield(&engine, p0, devout_witness()).expect("the Witness is out");
    let artifact = on_battlefield(&engine, p1, quiet_artifact()).expect("their artifact is out");
    let enchantment =
        on_battlefield(&engine, p1, their_enchantment()).expect("their enchantment is out");
    let their_elf = on_battlefield(&engine, p1, quiet_creature()).expect("their creature is out");
    let fodder = in_hand(&engine, p0, festering_goblin()).expect("the card to discard is in hand");

    // `legal.abilities` is filtered through `can_afford`, which reads the pool
    // and not the untapped lands (CR 601.2h), so the mana comes first.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        4,
        "four Plains tapped, and the Witness makes no mana of its own"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(witness, 0)),
        "with {{1}}{{W}} floating the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, devout_witness(), 0);

    // The two questions one activation asks — which permanent (CR 601.2c) and
    // which card is given up (CR 601.2h) — answered in the order they arrive
    // rather than in the order they are expected.
    let (mut aimed, mut discarded) = (false, false);
    for _ in 0..12 {
        if aimed && discarded {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                options,
                min,
                max,
                ..
            } => {
                assert_eq!((min, max), (1, 1), "one artifact or enchantment");
                assert!(
                    options.contains(&artifact),
                    "\"target artifact\" reaches across the table: {options:?}"
                );
                assert!(
                    options.contains(&enchantment),
                    "and so does \"or enchantment\": {options:?}"
                );
                assert!(
                    !options.contains(&their_elf),
                    "a creature is neither of the two: {options:?}"
                );
                assert!(
                    !options.contains(&witness),
                    "the Witness is a creature and no artifact: {options:?}"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![artifact],
                        },
                    )
                    .expect("the permanent the question offered is the one that dies");
                aimed = true;
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                ..
            } => {
                assert_eq!(
                    prompt,
                    crate::choice::ChoicePrompt::CostDiscard,
                    "a cost and not a loot: the card is given up, not cycled"
                );
                assert_eq!((min, max), (1, 1), "exactly one card, no more and no fewer");
                assert!(
                    options.contains(&fodder),
                    "the hand the seat is holding is the menu: {options:?}"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![fodder],
                        },
                    )
                    .expect("the card the question offered pays the cost");
                discarded = true;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the Witness' ability resolves: {other:?}"),
        }
    }
    assert!(aimed, "the target is chosen as the ability is announced");
    assert!(discarded, "and a card is given up to pay for it");

    assert!(
        !stack_is_empty(&engine),
        "destroying is no mana ability, so the ability is what is waiting"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        is_tapped(&engine, witness),
        "{{T}} is the other half of the price, paid by the Witness itself"
    );
    assert!(
        in_graveyard(&engine, p0, festering_goblin()).is_some(),
        "\"Discard a card\" — the discarded card is in its owner's graveyard"
    );
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "the artifact that was named is destroyed and went to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, their_enchantment()).is_some(),
        "one target, one destruction: the enchantment the ability did not name never moved"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_creature()).is_some(),
        "nor did the creature, which was never a legal target"
    );
    assert!(
        on_battlefield(&engine, p0, devout_witness()).is_some(),
        "the Witness survives its own ability"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        2,
        "{{1}}{{W}} came out of the four white that were floating"
    );
}
