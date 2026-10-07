//! `cards/lands/cycling/canyon_slough.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Canyon Slough prints three sentences and this scenario plays all three: it
/// goes down as the turn's land drop, a second copy is cycled out of hand for
/// `{2}`, and the first is tapped for mana on the following turn. The tap is
/// read a turn later because a land that entered tapped has a `{T}` on the
/// table and no way to spend it until its controller's untap step, and the
/// untapped Forest check is what tells that apart from a game that never
/// advanced. The colour question is the whole of `Add {B} or {R}`: a pool
/// reading alone would never say whether the card offered a choice or simply
/// picked for you.
#[test]
fn canyon_slough_enters_tapped_cycles_for_two_and_taps_for_black_or_red() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[canyon_slough(), canyon_slough()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The land drop, and the only way this sentence can be read: a placement
    // onto `starting_battlefield` is not an entry and runs no replacement, so
    // that copy of the card would be standing untapped whatever it prints.
    let card = in_hand(&engine, p0, canyon_slough()).expect("the land is in hand");
    engine
        .apply(p0, PlayerAction::PlayLand { card })
        .expect("a land card in hand on an untapped board is a legal land drop");
    let slough = on_battlefield(&engine, p0, canyon_slough()).expect("it reached the battlefield");
    assert!(
        is_tapped(&engine, slough),
        "\"This land enters tapped\" — so it gives nothing this turn"
    );

    // Cycling {2}: ability 1, behind the mana line at index 0.
    let library_before = library_size(&engine, p0);
    let graveyard_before = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, canyon_slough(), 1);
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "the cycle resolves back to a quiet priority"
    );
    assert!(
        in_graveyard(&engine, p0, canyon_slough()).is_some(),
        "\"Discard this card\" put the cycled copy in its owner's graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "and the cycle drew the card the cost was paid for"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        graveyard_before + 1,
        "one card discarded, and no other on the way"
    );

    // A turn further on the untap step has stood the land back up, which is
    // the only turn its `{T}` can be read in at all.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, slough),
        "the untap step ran, and the Forests beside it came back with it"
    );

    // Nothing is tapped before the claim: the whole price is the land's own
    // `{T}`, so an empty pool is what makes "one black and nothing else" an
    // exact reading rather than a coincidence.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    // The offered route may be the card's printed line or the CR 305.6
    // shortcut its Swamp and Mountain types earn it; either one is pressed.
    let route = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, index)| *id == slough && *index == 0)
        .map(|(source, ability_index)| PlayerAction::ActivateAbility {
            source,
            ability_index,
        })
        .unwrap_or(PlayerAction::ActivateManaAbility { source: slough });
    engine
        .apply(p0, route)
        .expect("the offer carried the land's mana ability");
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{B}} or {{R}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that taps the land names the colour");
    assert_eq!(options.len(), 2, "two colours and no default: {options:?}");
    assert!(
        options.contains(&ManaColor::Black),
        "the Swamp half: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Red),
        "and the Mountain half: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the two colours it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, off the land's own tap"
    );
    assert_eq!(pool.total(), 1, "one mana, and no other source was tapped");
    assert!(is_tapped(&engine, slough), "the land paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
}
