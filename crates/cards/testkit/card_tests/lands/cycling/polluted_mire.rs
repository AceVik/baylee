//! `cards/lands/cycling/polluted_mire.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Polluted Mire prints three lines — it enters tapped, `{T}: Add {B}`, and
/// cycling {2} — and they live in two zones, so the test plays one copy as the
/// land drop and cycles the other out of hand. The tapped entry is read off
/// the permanent `play_land` really put on the table (CR 614.1a) rather than
/// off the card file, and the Swamps beside it are the control for "the untap
/// step ran" a turn later: a land that never stood back up would satisfy the
/// mana line for the wrong reason. The cycling half is read where a
/// discard-and-draw leaves its marks — graveyard, library, and the empty pool
/// the {2} came out of — while the played permanent is still standing, which
/// is what tells the two copies apart.
#[test]
fn polluted_mire_enters_tapped_and_cycles_its_other_copy_for_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[polluted_mire(), polluted_mire()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The land drop, and the first printed line: the permanent arrives under
    // a tap this test never applied.
    let land = play_land(&mut engine, p0, polluted_mire());
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" — the replacement effect is the engine's, \
         not the harness'"
    );

    // The second copy is the one the cycling line is about, and it has to be
    // in hand: `ActivationZone::Hand` is where that ability lives.
    let hand_copy = in_hand(&engine, p0, polluted_mire()).expect("the other copy is in hand");
    let library_before = library_size(&engine, p0);

    // {2} into the pool first: `legal.abilities` is filtered through
    // `can_afford`, which reads the pool and not the untapped lands.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Swamps, and the Mire on the battlefield is already tapped"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| *id == hand_copy)
        .expect("cycling is offered on the card in hand, with its {2} floating");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the offer was the engine's own");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}} was the cost, and it was paid"
    );
    assert!(
        in_graveyard(&engine, p0, polluted_mire()).is_some(),
        "\"Discard this card\" puts the cycled card in the graveyard (CR 702.29a)"
    );
    assert!(
        in_hand(&engine, p0, polluted_mire()).is_none(),
        "and it took the hand copy: an ability that discarded the permanent \
         would have left the hand alone"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, polluted_mire()).len(),
        1,
        "the land played on turn one is still on the battlefield"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "and \"draw a card\" took exactly one off the top"
    );

    // A turn later for the second printed line. The untap step is asserted
    // before anything is tapped, so the mana below has one possible source
    // and a land that never came back cannot fake it.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step ran: the Mire is standing again"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and nothing is floating, so the next reading is exact"
    );

    let taken = tap_mana_where(&mut engine, p0, |id| id == land);
    assert_eq!(taken, 1, "the Mire's own {{T}} is the whole price");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.total(),
        1,
        "one mana, and neither Swamp was touched to make it"
    );
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "{{B}}, which is what the card prints it makes"
    );
    assert!(is_tapped(&engine, land), "and the land paid its own tap");
}
