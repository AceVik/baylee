//! `cards/lands/cycling/fetid_pools.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fetid Pools prints three lines and a real game reads all three: an
/// `Island Swamp` that enters tapped, and cycling `{2}`. The land is *played*
/// rather than seeded, because `SeatSpec::starting_battlefield` places a
/// permanent with no entry at all and the tapped clause is an entry
/// replacement (CR 614.1c) — a board built that way would arrive untapped
/// whatever the card says. The second copy is the cycling half: it is
/// discarded from hand for `{2}` out of three Forests while the copy on the
/// battlefield is tapped and so contributes nothing, which is what makes the
/// pool an exact statement about where the mana went. One turn later the
/// land's own `{T}` is read, the only reading of "Add {U} or {B}" that the
/// tapped clause does not already answer for free.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn fetid_pools_enters_tapped_cycles_for_two_and_adds_either_of_its_two_colors() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(4242, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[fetid_pools(), fetid_pools()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // "This land enters tapped": a real land drop, so the entry modifier runs.
    let land = play_land(&mut engine, p0, fetid_pools());
    assert!(
        entered_tapped(&engine, land),
        "the printed enters-tapped clause is an entry replacement (CR 614.1c), \
         and `play_land` plays the card instead of seeding it"
    );

    // The other copy is the one still in hand, and cycling lives there.
    let held = in_hand(&engine, p0, fetid_pools()).expect("one copy is left in hand");
    assert_ne!(
        held, land,
        "the played copy is on the battlefield, not in hand"
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // The Fetid Pools on the battlefield is tapped, so the three Forests are
    // the whole of the {2} and the pool says so exactly.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests, and nothing at all from the tapped land"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| *id == held)
        .expect("cycling is offered from hand, which is the zone the card prints");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("{{2}} is floating and the card discards itself");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{2}} came out of the pool"
    );
    assert!(
        in_graveyard(&engine, p0, fetid_pools()).is_some(),
        "`DiscardSelf` is the other half of the cost: the card cycles itself \
         rather than asking which card to pitch"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\""
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "one card discarded and one drawn, so the hand is the size it was"
    );

    // The land entered tapped, so its own {T} waits for an untap step
    // (CR 502.3): across the opponent's turn and back.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "the land's controller takes another turn"
    );
    assert!(!is_tapped(&engine, land), "the untap step ran");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the phase that held it (CR 500.5)"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    if legal.mana_abilities.contains(&land) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: land })
            .unwrap();
    } else {
        let index = legal
            .abilities
            .iter()
            .find(|(id, _)| *id == land)
            .map(|(_, index)| *index)
            .expect("the land's `{{T}}` is offered, whichever list carries it");
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: land,
                    ability_index: index,
                },
            )
            .unwrap();
    }

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`{{U}}` or `{{B}}` is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat names the color");
    assert!(
        options.contains(&ManaColor::Blue) && options.contains(&ManaColor::Black),
        "\"Add {{U}} or {{B}}\" offers both of the types it carries: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and its two basic land types are the whole menu — a five-color menu \
         would be a Mox and not an Island Swamp: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colors it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the color that was named, and not a default"
    );
    assert_eq!(pool.total(), 1, "one land tapped, one mana");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
}
