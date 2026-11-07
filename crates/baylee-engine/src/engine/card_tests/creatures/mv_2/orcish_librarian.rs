//! `cards/creatures/mv_2/orcish_librarian.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::{draw_kit, partials_w1b_kit as kit};
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn orcish_librarian() -> CardIndex {
    card_index("3a05904d-5439-446d-a2a1-1475c9c7371d")
}

/// Orcish Librarian (3a05904d-5439-446d-a2a1-1475c9c7371d) — {1}{R} 1/1 Creature — Orc:
/// casts for {1}{R} as a 1/1 Orc.
#[test]
fn orcish_librarian_casts_for_one_and_a_red_as_a_one_one_orc() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[orcish_librarian()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, orcish_librarian());
    pass_until(&mut engine, stack_is_empty);

    let orc = on_battlefield(&engine, P0, orcish_librarian()).unwrap();
    assert_eq!(pt(&engine, orc), (1, 1));
}

/// Builds the same deterministic table and activates the librarian on it.
#[track_caller]
fn activate_librarian(top_first: &[CardIndex; 10]) -> (Engine<RegistryLookup>, Vec<ObjectId>) {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[orcish_librarian(), mountain()])
        .hand(0, top_first)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    // This starting permanent has been controlled continuously since the first
    // turn began, so CR 302.6 does not prevent paying its tap cost. Staying in
    // this main phase also keeps the ten prepared cards out of cleanup.
    let stacked = kit::replace_library(&mut engine, P0, top_first);
    let librarian = on_battlefield(&engine, P0, orcish_librarian()).unwrap();
    tap_all_mana(&mut engine, P0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);

    activate(&mut engine, P0, orcish_librarian(), 0);
    assert!(is_tapped(&engine, librarian), "{{T}} is part of the cost");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the one floating red mana pays {{R}}"
    );
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    (engine, stacked)
}

/// "{R}, {T}: Look at the top eight cards of your library. Exile four of them
/// at random, then put the rest on top of your library in any order."
#[allow(clippy::too_many_lines)] // one activation, its private answer and a deterministic replay
#[test]
fn orcish_librarian_exiles_a_seeded_random_four_and_orders_the_other_four() {
    let top_first = [
        index::SWAMP,
        index::FOREST,
        index::LIGHTNING_BOLT,
        index::MOUNTAIN,
        index::PLAINS,
        index::ISLAND,
        index::GRIZZLY_BEARS,
        index::LLANOWAR_ELVES,
        index::SWAMP,
        index::MOUNTAIN,
    ];
    // Two independently built games with one seed must make the same choice.
    let (mut engine, stacked) = activate_librarian(&top_first);
    let (mut replay, replay_stacked) = activate_librarian(&top_first);
    assert_eq!(stacked, replay_stacked);
    let librarian = on_battlefield(&engine, P0, orcish_librarian()).unwrap();

    let exiled = engine.state().zones.list(ZoneLocation::Exile(P0)).clone();
    assert_eq!(exiled.len(), 4, "exactly four cards leave the library");
    assert!(
        exiled.iter().all(|card| stacked[..8].contains(card)),
        "every exiled card came from the inspected eight: {exiled:?}"
    );
    assert_ne!(
        draw_kit::sorted(&exiled),
        draw_kit::sorted(&stacked[..4]),
        "this seed must distinguish a random subset from merely exiling the top four"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Exile(P1)).len(),
        0,
        "the cards are exiled by their owner, not by the opponent"
    );
    assert_eq!(
        replay.state().zones.list(ZoneLocation::Exile(P0)),
        &exiled,
        "replaying the seed selects the same four cards"
    );

    let Pending::Arrange {
        player,
        cards,
        piles,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("the four remaining cards need an order");
    };
    let remaining: Vec<_> = stacked[..8]
        .iter()
        .copied()
        .filter(|card| !exiled.contains(card))
        .collect();
    assert_eq!(player, P0);
    assert_eq!(prompt, ArrangePrompt::Order);
    assert_eq!(piles.len(), 1);
    assert_eq!(piles[0].place, ArrangePlace::LibraryTop);
    assert_eq!(draw_kit::sorted(&cards), draw_kit::sorted(&remaining));
    let Pending::Arrange {
        player: replay_player,
        cards: replay_cards,
        ..
    } = replay.pending()
    else {
        panic!("the replay asks for the same private order");
    };
    assert_eq!(*replay_player, P0);
    assert_eq!(
        replay_cards, &cards,
        "the same seed also presents the same private remainder"
    );
    assert_eq!(
        &kit::library_of(&engine, P0)[..2],
        &stacked[8..].iter().rev().copied().collect::<Vec<_>>(),
        "the two uninspected cards remain at the bottom"
    );
    assert!(
        !engine
            .state()
            .journal
            .entries()
            .iter()
            .any(|entry| matches!(entry.event, GameEvent::Revealed { .. })),
        "looking and privately ordering do not reveal the remaining cards"
    );
    assert!(
        engine
            .apply(
                P1,
                PlayerAction::Arrange {
                    piles: vec![cards.clone()],
                },
            )
            .is_err(),
        "only the librarian's controller may answer the private order question"
    );

    let order = vec![remaining[2], remaining[0], remaining[3], remaining[1]];
    engine
        .apply(
            P0,
            PlayerAction::Arrange {
                piles: vec![order.clone()],
            },
        )
        .expect("any order of the remaining four is legal");
    replay
        .apply(
            P0,
            PlayerAction::Arrange {
                piles: vec![order.clone()],
            },
        )
        .expect("the replay accepts the same order");
    pass_until(&mut engine, stack_is_empty);
    pass_until(&mut replay, stack_is_empty);

    let mut expected: Vec<_> = stacked[8..].iter().rev().copied().collect();
    expected.extend(order.iter().rev());
    assert_eq!(kit::library_of(&engine, P0), expected);
    assert_eq!(kit::library_of(&replay, P0), expected);
    assert!(is_tapped(&engine, librarian));
}
