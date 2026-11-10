//! `cards/creatures/mv_1/tireless_tribe.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tireless Tribe is a {W} 1/1 printing one line — "Discard a card: This
/// creature gets +0/+4 until end of turn" — and both halves of it are only
/// readable by playing it. The feed is a *cost*, so it is asked before
/// anything moves (CR 601.2h) and arrives as `ChoicePrompt::CostDiscard`, and
/// the pump is `+0/+4` on **this** creature: a `(1, 5)` is the only reading
/// that gets both numbers right, and the Elf beside it staying `(1, 1)` is
/// what separates "this creature" from "creatures you control". The discarded
/// card is followed into the graveyard, because a discard that left the card
/// in hand would satisfy the size assertion just as well.
#[test]
fn tireless_tribe_discards_a_card_to_become_a_one_five() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), quiet_creature()])
        .hand(0, &[tireless_tribe(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, tireless_tribe());
    pass_until(&mut engine, stack_is_empty);
    let tribe = on_battlefield(&engine, p0, tireless_tribe()).expect("the Tribe resolved");
    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("the Elf is out");
    assert_eq!(
        pt(&engine, tribe),
        (1, 1),
        "a printed 1/1 before anything is fed to it"
    );

    let fodder = in_hand(&engine, p0, island()).expect("the Island is in hand to be fed to it");
    // Ability 0 is the whole card: discard a card, pump this creature.
    activate(&mut engine, p0, tireless_tribe(), 0);

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the discard is a cost, so it is chosen before it is paid: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat pays");
    assert_eq!(
        prompt,
        ChoicePrompt::CostDiscard,
        "a cost and not a cleanup or a search, which is all a client has to \
         tell the three apart"
    );
    assert_eq!((min, max), (1, 1), "one card, no more and no fewer");
    assert!(
        options.contains(&fodder),
        "the card named in the opening hand is on the menu: {options:?}"
    );
    assert!(
        !options.contains(&tribe),
        "the Tribe is a permanent on the battlefield and no longer a card in \
         hand: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("the card the question offered pays the cost");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, island()).is_some(),
        "the discarded card is in its owner's graveyard"
    );
    assert_eq!(
        pt(&engine, tribe),
        (1, 5),
        "+0/+4 lands on the creature that paid, and the power is untouched"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "\"This creature\" is not \"creatures you control\": the Elf beside it \
         was never fed and never grew"
    );
}
