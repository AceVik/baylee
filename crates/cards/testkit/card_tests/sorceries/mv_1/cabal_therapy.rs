//! `cards/sorceries/mv_1/cabal_therapy.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn cabal_therapy() -> CardIndex {
    card_index("018107d1-ebf7-4657-bc91-c8374af55cc5")
}

fn grizzly_bears() -> CardIndex {
    baylee_core::generated::index::GRIZZLY_BEARS
}

fn choose_name(engine: &mut Engine<RegistryLookup>, player: PlayerId, card: CardIndex) {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseCardName { .. })
    });
    engine
        .apply(player, PlayerAction::ChooseCardName { card, face: 0 })
        .unwrap();
}

/// The normal spell names only a nonland card, reveals the targeted player's
/// whole hand, and discards every copy of the name without asking which ones.
/// Its flashback then sacrifices a creature while being cast and exiles itself.
#[test]
#[allow(clippy::too_many_lines)] // one game: the cast, then its flashback
fn cabal_therapy_discards_all_named_cards_and_flashes_back_for_a_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), grizzly_bears()])
        .hand(0, &[cabal_therapy()])
        .hand(
            1,
            &[grizzly_bears(), grizzly_bears(), lightning_bolt(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let revealed = engine.state().zones.list(ZoneLocation::Hand(p1)).clone();
    cast_from_hand(&mut engine, p0, cabal_therapy());
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCardName { .. })
    });
    let Pending::ChooseCardName {
        player,
        types,
        excluded_types,
    } = engine.pending()
    else {
        unreachable!()
    };
    assert_eq!(
        (*player, *types, *excluded_types),
        (p0, TypeSet::EMPTY, TypeSet::LAND)
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseCardName {
                    card: forest(),
                    face: 0,
                },
            )
            .is_err(),
        "a land is not a legal Cabal Therapy name"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseCardName {
                card: grizzly_bears(),
                face: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine.state().journal.entries().iter().any(|entry| {
            matches!(&entry.event, GameEvent::Revealed { player, cards }
            if *player == p1 && *cards == revealed)
        }),
        "the targeted player's whole hand was revealed"
    );
    let discarded_bears = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p1))
        .iter()
        .filter(|&&id| {
            engine.state().object(id).is_some_and(|object| {
                object
                    .card
                    .is_some_and(|card| card.index == grizzly_bears())
            })
        })
        .count();
    assert_eq!(discarded_bears, 2, "all cards with the chosen name discard");
    assert!(in_hand(&engine, p1, lightning_bolt()).is_some());
    assert!(in_hand(&engine, p1, forest()).is_some());
    assert!(in_graveyard(&engine, p0, cabal_therapy()).is_some());

    let therapy = in_graveyard(&engine, p0, cabal_therapy()).unwrap();
    let creature = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: therapy })
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let flashback = choose_cast_kind(&engine, CastModeKind::Flashback);
        engine
            .apply(p0, PlayerAction::ChooseMode(flashback))
            .unwrap();
    }
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!(
            "expected the flashback creature sacrifice, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(options, vec![creature]);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![creature],
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().object(creature).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(engine.state().object(therapy).unwrap().zone, Zone::Stack);

    choose_name(&mut engine, p0, lightning_bolt());
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p1, lightning_bolt()).is_some());
    assert!(in_hand(&engine, p1, forest()).is_some());
    assert_eq!(engine.state().object(therapy).unwrap().zone, Zone::Exile);
}
