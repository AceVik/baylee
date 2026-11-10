//! `cards/creatures/mv_4/krosan_archer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Krosan Archer — {3}{G} Centaur Archer, 2/3, printed with reach and with
/// "{G}, Discard a card: This creature gets +0/+2 until end of turn."
///
/// The activated ability is the whole card, and its price has two halves that
/// land in two different places: the {G} out of the pool, and the discarded card
/// in its owner's graveyard. One Forest is deliberately the entire board, so the
/// pool reads exactly one green before the activation and nothing after it —
/// which makes the mana a real payment and not a label — and the fodder is a
/// *named* card in hand, the only way to tell "discard a card" from a discard
/// the engine picked for itself. The pump is read as a body: +0/+2 on a printed
/// 2/3 is a (2, 5), where a (2, 3) would mean nothing happened and a (4, 5) that
/// the power had been pumped too.
#[test]
fn krosan_archer_trades_a_card_and_a_green_for_two_toughness() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[krosan_archer(), forest()])
        .hand(0, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let archer = on_battlefield(&engine, p0, krosan_archer()).expect("the Archer is out");
    assert!(
        keywords(&engine, archer).contains(KeywordSet::REACH),
        "the printed reach reaches the permanent through the layers"
    );
    assert_eq!(pt(&engine, archer), (2, 3), "the body the card prints");

    // The whole price is a green and a card, so one Forest in the pool is
    // exactly enough and the reading after the activation is exact.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one Forest, and the Archer makes no mana of its own"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(archer, 0)),
        "the one line the card prints, now that its {{G}} is in the pool: {:?}",
        legal.abilities
    );

    let fodder = in_hand(&engine, p0, llanowar_elves()).expect("the fodder is in hand");
    activate(&mut engine, p0, krosan_archer(), 0);
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
            "the discard is a cost and is asked before it is paid, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        ChoicePrompt::CostDiscard,
        "a cost and not a search or a rummage, which is all a client has to tell apart"
    );
    assert_eq!((min, max), (1, 1), "one card, no more and no fewer");
    assert!(
        options.contains(&fodder),
        "the named card in hand is on the menu: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("the card the question offered pays the cost");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{G}} came out of the pool"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "and the discarded card is in its owner's graveyard"
    );
    assert!(
        in_hand(&engine, p0, llanowar_elves()).is_none(),
        "which it could not be while it were still in hand"
    );
    assert!(
        !stack_is_empty(&engine),
        "a pump is no mana ability, so the ability is waiting on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, archer),
        (2, 5),
        "+0/+2 until end of turn on the Archer itself"
    );

    // "until end of turn": one turn later the Archer carries its printed body
    // again, so the two toughness were a duration and not a permanent grant.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, archer),
        (2, 3),
        "the pump lasted the turn it was paid for and no longer"
    );
    assert!(
        on_battlefield(&engine, p0, krosan_archer()).is_some(),
        "and the creature is still standing, so the pump left rather than the creature"
    );
}
