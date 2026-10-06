//! `cards/lands/utility/otawara_soaring_city.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Otawara, Soaring City prints a plain `{T}: Add {U}` and, beside it, a
/// Channel line that is not a land ability at all: "{3}{U}, Discard this
/// card: Return target artifact, creature, enchantment, or planeswalker to
/// its owner's hand", activated from hand. This plays the Channel half, so
/// both readings that make it what it is are struck: the card leaves the
/// *hand* as a cost — the graveyard and not the battlefield has to hold it
/// afterwards — and the creature across the table is bounced to its owner's
/// hand while the Forest beside it, a land and so none of the four named
/// card types, is never offered as a target in the first place.
#[test]
fn otawara_channels_from_hand_to_bounce_a_creature_and_discards_itself() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(97, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[otawara_soaring_city()])
        .battlefield(1, &[llanowar_elves(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own first main"
    );
    assert!(
        in_hand(&engine, p0, otawara_soaring_city()).is_some(),
        "the Channel line is activated from hand, which is where the card is"
    );

    // The cost is {3}{U} and the engine reads it off the *pool*, so the mana
    // goes in first: four Islands are the basic land type of CR 305.6, which
    // is the whole of what `tap_all_mana` taps.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        4,
        "four Islands pay three generic and the one blue"
    );

    let their_elves = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let their_forest = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    let library_after_channel = library_size(&engine, p0);

    // Ability 1 is the Channel line. Ability 0 is the printed mana ability,
    // which a card in hand cannot pay a {T} for and so is never offered.
    activate(&mut engine, p0, otawara_soaring_city(), 1);

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the Channel line asks for its target before anything is paid, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat names the target");
    assert_eq!((min, max), (1, 1), "exactly one permanent comes back");
    assert!(
        options.contains(&their_elves),
        "\"target artifact, creature, …\" reaches across the table: {options:?}"
    );
    assert!(
        !options.contains(&their_forest),
        "a land is none of the four card types the sentence names: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_elves],
            },
        )
        .expect("the creature the question offered is the one it was aimed at");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, otawara_soaring_city()).is_some(),
        "discarding this card is the cost, so it never reached the battlefield \
         and lies in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "the Elf left the battlefield"
    );
    assert!(
        in_hand(&engine, p1, llanowar_elves()).is_some(),
        "…to its *owner's* hand, the seat that cast nothing"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "and the permanent that was never a legal target never moved"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_after_channel,
        "a bounce draws nothing, for either seat"
    );
}
