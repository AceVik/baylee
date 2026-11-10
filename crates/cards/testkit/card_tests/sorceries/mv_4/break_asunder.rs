//! `cards/sorceries/mv_4/break_asunder.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "d2c53737-c265-46e3-a779-52c6b4f82d7d"

/// Break Asunder prints two lines and neither stands in for the other:
/// "Destroy target artifact or enchantment", and "Cycling {2}" — an activated
/// ability the card offers while it sits in the **hand**, which is the half a
/// board test of the sorcery alone would never reach.
///
/// One first main phase plays both, because the mana is one pool: six Forests
/// are tapped once, the {2}{G}{G} leaves exactly the {2} the cycling charges
/// (CR 500.5 keeps what the first spell did not spend), and the target menu is
/// the printed "or" itself — the opponent's artifact and the opponent's
/// enchantment are on it while the creature and the land beside them are
/// neither, and only the permanent that was named ends up in a graveyard.
#[test]
#[allow(clippy::too_many_lines)]
fn break_asunder_destroys_an_artifact_or_enchantment_and_cycles_a_second_copy() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .hand(0, &[break_asunder(), break_asunder()])
        // Both halves of the printed disjunction, plus a creature and a land
        // that are neither.
        .battlefield(1, &[quiet_artifact(), exploration(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let chant = on_battlefield(&engine, p1, exploration()).expect("their enchantment is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let land = on_battlefield(&engine, p0, forest()).expect("my Forest is out");

    // Six Forests into the pool in one go: {2}{G}{G} for the sorcery and the
    // {2} the cycling charges are paid out of the same pool, which stays
    // floating until the step ends (CR 500.5) — and this whole scenario never
    // leaves that one main phase.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Forests, and the artifact and the Elf across the table make no mana for me"
    );

    cast_with_floating(&mut engine, p0, break_asunder());
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"destroy target artifact or enchantment\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it is the one that aims it");
    assert_eq!((min, max), (1, 1), "one target, and the spell asks once");
    assert!(
        options.contains(&ring) && options.contains(&chant),
        "both halves of the printed \"or\" are on the menu: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "a creature is neither an artifact nor an enchantment: {options:?}"
    );
    assert!(
        !options.contains(&land),
        "and a land is neither — not even the Forest that paid for the spell: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chant],
            },
        )
        .expect("the enchantment the question offered was chosen");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        in_graveyard(&engine, p1, exploration()).is_some(),
        "the enchantment the spell named is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "and the artifact it did not name never moved"
    );
    assert_eq!(
        mine(&engine, p0, break_asunder(), Zone::Graveyard).len(),
        1,
        "the sorcery resolved rather than being discarded: one copy is in the \
         graveyard and one is still in hand"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "{{2}}{{G}}{{G}} is four of the six, and the two the cycling charges \
         are exactly what is left"
    );

    // The second printed line: cycling is an activated ability the card offers
    // out of the hand, and its price is that {2} plus the card itself.
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let library_before = library_size(&engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.iter().any(|(source, index)| {
            *index == 0
                && engine
                    .state()
                    .object(*source)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == break_asunder()))
        }),
        "cycling is the card's own ability and it is offered from the hand \
         now that its {{2}} is in the pool: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, break_asunder(), 0);
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        mine(&engine, p0, break_asunder(), Zone::Graveyard).len(),
        2,
        "\"Discard this card\" is the cost, so the cycled copy is in the \
         graveyard beside the copy that resolved"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "and it reached the hand — one card discarded and one drawn, so a \
         cycling that only discarded would leave the hand a card short"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}} the cycling charges came out of the pool"
    );
    assert!(
        stack_is_empty(&engine),
        "the draw resolved, so nothing is left waiting on the stack"
    );
}
