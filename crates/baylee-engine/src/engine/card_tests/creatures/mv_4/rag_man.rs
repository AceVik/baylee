//! `cards/creatures/mv_4/rag_man.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn rag_man() -> CardIndex {
    card_index("8132c521-44e2-4fc0-b3de-e5b92d04463a")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn float(engine: &mut Engine<RegistryLookup>, seat: usize, mana: &[(ManaColor, u32)]) {
    let state = engine
        .dev_state_mut(PlayerId::new(0))
        .expect("the harness may set boards up");
    for (color, n) in mana {
        state.players[seat].mana_pool.add(*color, *n);
    }
    engine.refresh_offer();
}

fn offered(engine: &Engine<RegistryLookup>, card: CardIndex, index: u32) -> bool {
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    legal.abilities.iter().any(|(id, ai)| {
        *ai == index
            && engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
    })
}

fn target_player(engine: &mut Engine<RegistryLookup>, player: PlayerId) {
    let seat = engine
        .pending()
        .asked()
        .expect("a target choice is pending");
    if matches!(engine.pending(), Pending::ChoosePlayer { .. }) {
        engine
            .apply(seat, PlayerAction::ChoosePlayer(player))
            .expect("a legal player");
    } else {
        engine
            .apply(
                seat,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![player],
                },
            )
            .expect("a legal player");
    }
}

fn board(their_hand: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[rag_man()])
        .hand(1, their_hand)
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, PlayerId::new(0)),
        "p0 reaches its own main"
    );
    engine
}

fn revealed_by(engine: &Engine<RegistryLookup>, from: usize, player: PlayerId) -> bool {
    engine.state().journal.entries()[from..]
        .iter()
        .any(|entry| {
            matches!(
                &entry.event,
                crate::event::GameEvent::Revealed { player: who, .. } if *who == player
            )
        })
}

/// Rag Man (8132c521-44e2-4fc0-b3de-e5b92d04463a) — "{B}{B}{B}, {T}: Target
/// opponent reveals their hand and discards a creature card at random.
/// Activate only during your turn."
#[test]
fn rag_man_reveals_the_hand_and_discards_one_creature_card_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = board(&[island(), grizzly_bears(), hill_giant()]);
    let rag = on_battlefield(&engine, p0, rag_man()).expect("Rag Man is out");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();
    let journal_before = engine.state().journal.entries().len();

    float(&mut engine, 0, &[(ManaColor::Black, 3)]);
    activate(&mut engine, p0, rag_man(), 0);
    target_player(&mut engine, p1);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        revealed_by(&engine, journal_before, p1),
        "p1's hand was revealed"
    );
    assert!(!revealed_by(&engine, journal_before, p0), "p0's was not");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        hand_before - 1,
        "exactly one card discarded"
    );
    assert!(
        in_hand(&engine, p1, island()).is_some(),
        "the land is not a creature card"
    );
    let bears_gone = in_graveyard(&engine, p1, grizzly_bears()).is_some();
    let giant_gone = in_graveyard(&engine, p1, hill_giant()).is_some();
    assert!(
        bears_gone != giant_gone,
        "one creature card, at random: bears {bears_gone}, giant {giant_gone}"
    );
    assert!(is_tapped(&engine, rag), "{{T}} is part of the cost");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{B}}{{B}}{{B}} paid"
    );
}

/// No creature card in hand: the hand is shown and nothing is discarded.
#[test]
fn rag_man_reveals_but_discards_nothing_without_a_creature_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = board(&[island(), forest()]);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();
    let journal_before = engine.state().journal.entries().len();
    float(&mut engine, 0, &[(ManaColor::Black, 3)]);
    activate(&mut engine, p0, rag_man(), 0);
    target_player(&mut engine, p1);
    pass_until(&mut engine, stack_is_empty);
    assert!(revealed_by(&engine, journal_before, p1));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        hand_before,
        "no creature card, no discard"
    );
    assert!(in_graveyard(&engine, p1, island()).is_none());
}

/// "Activate only during your turn."
#[test]
fn rag_man_can_only_be_activated_on_its_controllers_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = board(&[grizzly_bears()]);
    float(&mut engine, 0, &[(ManaColor::Black, 3)]);
    assert!(
        offered(&engine, rag_man(), 0),
        "on p0's own turn it is offered"
    );

    reach_their_main_phase(&mut engine, p1);
    engine
        .apply(p1, PlayerAction::PassPriority)
        .expect("p1 passes");
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "p0 has priority on p1's turn, got {:?}",
        engine.pending()
    );
    float(&mut engine, 0, &[(ManaColor::Black, 3)]);
    assert!(
        !offered(&engine, rag_man(), 0),
        "on p1's turn it is not, however much black is floating"
    );
}
