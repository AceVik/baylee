//! `cards/creatures/mv_3/mercenary_knight.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mercenary Knight prints one clause of its own — "When this creature enters,
/// sacrifice it unless you discard a creature card" — and it is a price with a
/// **filter** inside it, so a single board answers both halves of the card.
/// Two copies are cast off six Swamps in the same main phase: the first is
/// asked for the discard, and the menu it publishes has to be exactly the
/// creature cards in hand, with the Forest beside them as the control for
/// `Filter::CREATURE`; the second, with no creature card left to give up, is
/// sacrificed with no question asked at all, which is where a may-pay-cost-or
/// goes when its price cannot be paid. The Knight that paid is a 4/4 still on
/// the battlefield with the Elf it paid with in the graveyard, and the one
/// that could not pay is in the graveyard beside it.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn mercenary_knight_keeps_itself_for_a_creature_card_and_is_sacrificed_without_one() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp(), swamp()])
        .hand(
            0,
            &[
                mercenary_knight(),
                mercenary_knight(),
                llanowar_elves(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Swamps, and each Knight costs {{2}}{{B}}"
    );
    cast_with_floating(&mut engine, p0, mercenary_knight());

    // The entry trigger, answered in the order it is asked and out of what it
    // published: the discard menu is the creature cards in hand, and the cost
    // is taken from that very menu rather than named from outside it.
    let mut paid = false;
    for _ in 0..20 {
        if paid && stack_is_empty(&engine) {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::YesNo { player, .. } => {
                engine.apply(player, PlayerAction::YesNo(true)).unwrap();
            }
            Pending::ChooseCards {
                player,
                options,
                max,
                prompt,
                ..
            } => {
                assert_eq!(player, p0, "the seat that cast it pays its own price");
                assert_eq!(
                    prompt,
                    crate::choice::ChoicePrompt::CostDiscard,
                    "a cost and not a search, which is all a client has to tell the two apart"
                );
                assert_eq!(max, 1, "one card is the whole price");
                let elf = in_hand(&engine, p0, llanowar_elves()).expect("the Elf is in hand");
                let land = in_hand(&engine, p0, forest()).expect("the Forest is in hand");
                assert!(
                    options.contains(&elf),
                    "a creature card in hand is the price: {options:?}"
                );
                assert!(
                    !options.contains(&land),
                    "`Filter::CREATURE` is read and not skipped: a Forest is a card \
                     and no creature card: {options:?}"
                );
                assert_eq!(
                    options.len(),
                    2,
                    "the Elf and the second Knight, which has not been cast yet: {options:?}"
                );
                for option in &options {
                    assert!(
                        engine.state().object(*option).is_some_and(|o| {
                            o.characteristics().types.contains(TypeSet::CREATURE)
                        }),
                        "every card on the menu is a creature card: {options:?}"
                    );
                }
                engine
                    .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
                    .expect("the Elf the question offered pays the price");
                paid = true;
            }
            other => panic!("unexpected while the entry trigger asks: {other:?}"),
        }
    }
    assert!(paid, "the discard was asked for and paid");

    let survivor =
        on_battlefield(&engine, p0, mercenary_knight()).expect("the first Knight is on the table");
    assert_eq!(pt(&engine, survivor), (4, 4), "the body the card prints");
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "the discarded creature card is in the graveyard, which is where a discard puts it"
    );

    // The second Knight. No card in hand is a creature card any more — the Elf
    // is in a graveyard, and the Knight being cast is on the stack while its
    // own trigger asks — so the price cannot be paid and nothing is asked:
    // the print's "unless" is what happens, and the body is sacrificed.
    cast_with_floating(&mut engine, p0, mercenary_knight());
    let mut resolved = false;
    for _ in 0..8 {
        if stack_is_empty(&engine) && in_graveyard(&engine, p0, mercenary_knight()).is_some() {
            resolved = true;
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!(
                "with no creature card to discard there is no price to offer, so \
                 nothing should be asked — got {other:?}"
            ),
        }
    }
    assert!(resolved, "the second Knight's trigger resolved");
    assert_eq!(
        all_on_battlefield(&engine, p0, mercenary_knight()).len(),
        1,
        "the second Knight sacrificed itself: only the copy that paid its price is left"
    );
    assert!(
        in_graveyard(&engine, p0, mercenary_knight()).is_some(),
        "and it is in its owner's graveyard, which is where a sacrificed permanent goes"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the two casts spent the six Swamps down to nothing"
    );
}
