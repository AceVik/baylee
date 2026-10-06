//! `cards/creatures/mv_3/benalish_knight.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "32b401e9-163f-4917-a728-fc63b25ef602"

/// Benalish Knight prints exactly two lines — flash and first strike — on a
/// printed 2/2, and neither can be read off the card file: flash is a *timing*
/// permission, so proving it means casting the Knight outside its controller's
/// main phase, while an opponent's creature spell is on the stack. Ondu Cleric
/// costs `{1}{W}`, sits in the same hand and is affordable out of exactly the
/// same pool, and is the control: with no flash it has to stay off the offer
/// beside the Knight, which is what tells a flash permission from a pool that
/// merely got lucky. The body and the first-strike keyword are then read on the
/// permanent that resolved, where nothing else on either board could have
/// granted or modified either.
#[test]
fn benalish_knight_flashes_in_on_the_opponents_turn_and_lands_as_a_two_two_first_striker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[benalish_knight(), ondu_cleric()])
        .battlefield(1, &[forest()])
        .hand(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);

    // p1's turn, and a creature spell of p1's on the stack. A creature is
    // castable only in its controller's own main phase at sorcery speed, so
    // the only thing that may put a creature spell of the non-active seat's
    // into *this* window is flash.
    reach_their_main_phase(&mut engine, p1);
    tap_all_mana(&mut engine, p1);
    assert_eq!(
        engine.state().players[1]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "p1's lone Forest is the {{G}} the Elf costs"
    );
    cast_with_floating(&mut engine, p1, llanowar_elves());
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    assert!(
        !stack_is_empty(&engine),
        "p1's Elf is still on the stack, which is the window the flash cast \
         has to happen in"
    );

    // Mana into the pool *before* the offer is read: `castable` is filtered
    // through `can_afford`, which reads the pool and not the untapped lands.
    let knight = in_hand(&engine, p0, benalish_knight()).expect("the Knight is in hand");
    let cleric = in_hand(&engine, p0, ondu_cleric()).expect("the Cleric is in hand");
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        3,
        "three Plains, three white — {{2}}{{W}} and {{1}}{{W}} are both payable"
    );

    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!(
            "a priority window on the opponent's turn, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "p1 passed, and the window is p0's");
    assert!(
        legal.castable.contains(&knight),
        "Flash (CR 702.8a): this creature is castable outside its controller's \
         main phase because it says so: {:?}",
        legal.castable
    );
    assert!(
        !legal.castable.contains(&cleric),
        "the same pool and the same window, and the Cleric prints no flash: \
         sorcery timing is not a mana question: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, benalish_knight());
    pass_until(&mut engine, stack_is_empty);

    // The Knight resolved above p1's Elf rather than in place of it.
    let knight = on_battlefield(&engine, p0, benalish_knight()).expect("the Knight resolved");
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "p1's Elf resolved behind it — the Knight was cast in response to the \
         spell, not instead of it"
    );
    assert!(
        in_hand(&engine, p0, ondu_cleric()).is_some(),
        "and the flash-less creature the offer declined never left the hand"
    );
    assert_eq!(
        pt(&engine, knight),
        (2, 2),
        "the printed 2/2, read off the permanent the layers projected"
    );
    let granted = keywords_of(&engine, knight);
    assert!(
        granted.contains(KeywordSet::FLASH),
        "the printed flash reaches the permanent"
    );
    assert!(
        granted.contains(KeywordSet::FIRST_STRIKE),
        "and so does the printed first strike"
    );
}
