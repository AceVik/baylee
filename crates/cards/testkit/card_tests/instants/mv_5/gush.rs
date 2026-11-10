//! `cards/instants/mv_5/gush.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

// oracle_id = "16d8ee99-8ec8-429f-9ba7-818a74b6f910"
fn gush() -> CardIndex {
    card_index("16d8ee99-8ec8-429f-9ba7-818a74b6f910")
}

fn controlled_islands(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine.state().object(*id).is_some_and(|object| {
                object.controller == seat && object.card.is_some_and(|card| card.index == island())
            })
        })
        .collect()
}

fn cast_for_islands(engine: &mut Engine<RegistryLookup>) -> ObjectId {
    let spell = in_hand(engine, me(), gush()).unwrap();
    cast_with_floating(engine, me(), gush());
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let alternative = choose_cast_kind(engine, CastModeKind::Alternative(0));
        engine
            .apply(me(), PlayerAction::ChooseMode(alternative))
            .unwrap();
    }
    spell
}

/// Gush — {4}{U} Instant: its ordinary mana cost draws exactly two cards and
/// leaves every Island on the battlefield.
#[test]
fn gush_draws_two_cards_for_its_mana_cost() {
    let mut engine = table(
        SEED,
        &[gush()],
        &[island(), island(), island(), island(), island()],
        &[],
        &[],
    );
    let islands = controlled_islands(&engine, me());
    let hand_before = hand_len(&engine, me());
    let library_before = library_len(&engine, me());
    cast_from_hand(&mut engine, me(), gush());
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let normal = choose_cast_kind(&engine, CastModeKind::Normal);
        engine
            .apply(me(), PlayerAction::ChooseMode(normal))
            .unwrap();
    }
    settle(&mut engine);

    assert_eq!(hand_len(&engine, me()), hand_before - 1 + 2);
    assert_eq!(library_len(&engine, me()), library_before - 2);
    assert!(islands.iter().all(|island| {
        engine
            .state()
            .zones
            .contains(*island, ZoneLocation::Battlefield)
    }));
}

/// The alternative cost asks for two distinct controlled Islands. They reach
/// the hand before Gush resolves, no mana is spent, and the spell still draws
/// exactly two cards. A Forest cannot pay either part.
#[test]
fn gush_returns_two_distinct_islands_instead_of_paying_mana() {
    let mut engine = table(SEED, &[gush()], &[island(), island(), forest()], &[], &[]);
    let islands = controlled_islands(&engine, me());
    let forest = on_battlefield(&engine, me(), forest()).unwrap();
    let library_before = library_len(&engine, me());
    let spell = cast_for_islands(&mut engine);

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
            "expected the first Island return, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(
        (player, min, max, prompt),
        (me(), 1, 1, ChoicePrompt::CostReturn)
    );
    assert_eq!(options, islands);
    assert!(!options.contains(&forest));
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![islands[0]],
            },
        )
        .unwrap();

    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!(
            "expected the second Island return, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(options, vec![islands[1]]);
    assert!(
        engine
            .apply(
                me(),
                PlayerAction::ChooseObjects {
                    objects: vec![islands[0]]
                }
            )
            .is_err()
    );
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![islands[1]],
            },
        )
        .unwrap();

    assert!(engine.state().object(spell).unwrap().alt_cast);
    assert_eq!(engine.state().object(spell).unwrap().zone, Zone::Stack);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert!(islands.iter().all(|island| {
        engine
            .state()
            .zones
            .contains(*island, ZoneLocation::Hand(me()))
    }));
    assert_eq!(hand_len(&engine, me()), 2);
    assert!(
        engine
            .state()
            .zones
            .contains(forest, ZoneLocation::Battlefield)
    );
    settle(&mut engine);
    assert_eq!(
        hand_len(&engine, me()),
        4,
        "two Islands and two drawn cards"
    );
    assert_eq!(library_len(&engine, me()), library_before - 2);
    assert!(in_graveyard(&engine, me(), gush()).is_some());
}

/// One controlled Island, a Forest and an opponent's Islands do not make two
/// Islands the caster controls. With only two mana floating and no sources
/// left, the printed cost cannot rescue this cast either.
#[test]
fn gush_needs_two_controlled_islands_for_its_alternative_cost() {
    let mut engine = table(
        SEED,
        &[gush()],
        &[island(), forest()],
        &[],
        &[island(), island()],
    );
    tap_all_mana(&mut engine, me());
    let spell = in_hand(&engine, me(), gush()).unwrap();
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(!legal.castable.contains(&spell));
    assert!(!legal.payable.contains(&spell));
    assert!(
        engine
            .apply(me(), PlayerAction::CastSpell { card: spell })
            .is_err()
    );
    assert!(
        engine
            .state()
            .zones
            .contains(spell, ZoneLocation::Hand(me()))
    );
    assert_eq!(controlled_islands(&engine, me()).len(), 1);
    assert_eq!(controlled_islands(&engine, them()).len(), 2);
}
