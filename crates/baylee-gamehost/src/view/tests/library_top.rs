use super::*;
use baylee_engine::{event::Cause, zone::ZonePosition};

fn named(name: &str) -> DeckEntry {
    DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card: baylee_cards::decks::by_name(name).unwrap(),
        print: PrintRef::new(0),
    }
}

/// Reaches the priority after Gwenom's real attack trigger has resolved.
fn after_gwenom_attacks(engine: &mut Engine<Registry>, gwenom: ObjectId) {
    let me = PlayerId::new(0);
    let them = PlayerId::new(1);
    for _ in 0..100 {
        if baylee_engine::casting::library_top_visible_to(engine.state(), me, me)
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == me)
        {
            return;
        }
        let (player, action) = match engine.pending().clone() {
            Pending::Priority { player, .. } => (player, PlayerAction::PassPriority),
            Pending::ChooseAttackers { player, .. } if player == me => (
                player,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(gwenom, baylee_core::ids::Defender::Player(them))],
                },
            ),
            other => panic!("unexpected question: {other:?}"),
        };
        engine.apply(player, action).expect("a legal answer");
    }
    panic!("Gwenom's trigger did not resolve");
}

#[test]
fn gwenoms_private_top_is_host_filtered_and_a_changed_top_waits_for_the_cast() {
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let mut preset = mixed_print_preset();
    preset.seats[0].starting_battlefield = vec![named("Gwenom, Remorseless")];
    preset.seats[0].starting_hand = Some(vec![named("Dark Ritual"), named("Lightning Bolt")]);
    let mut engine = Engine::new(&preset, Registry).expect("game starts");
    let initial = settle(&mut engine, None);
    let gwenom = initial
        .battlefield
        .iter()
        .find(|object| object.name == "Gwenom, Remorseless")
        .expect("Gwenom on the battlefield")
        .id;
    let ritual = initial
        .hand
        .iter()
        .find(|object| object.name == "Dark Ritual")
        .expect("Dark Ritual in hand")
        .id;
    let bolt = initial
        .hand
        .iter()
        .find(|object| object.name == "Lightning Bolt")
        .expect("Lightning Bolt in hand")
        .id;
    for card in [ritual, bolt] {
        engine
            .dev_state_mut(me)
            .unwrap()
            .move_object(
                card,
                ZoneLocation::Library(me),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
    }
    engine.refresh_offer();
    after_gwenom_attacks(&mut engine, gwenom);

    let mine = seen_by(&engine, me);
    assert_eq!(mine.library_tops.len(), 1, "only one library top is shown");
    assert_eq!(mine.library_tops[0].id, bolt);
    assert!(mine.library_tops[0].card.is_some());
    assert!(
        !mine.library_tops.iter().any(|object| object.id == ritual),
        "the card below the top stays hidden"
    );
    assert!(seen_by(&engine, them).library_tops.is_empty());
    let spectator = spectator_view(
        engine.state(),
        1,
        None,
        SeatSet::new(),
        engine.library_reveal_blocked(),
        &[],
        &[],
    );
    assert!(
        spectator.library_tops.is_empty(),
        "a private permission is not a public reveal"
    );

    engine
        .apply(me, PlayerAction::CastSpell { card: bolt })
        .expect("the offered top spell begins casting");
    assert!(matches!(engine.pending(), Pending::ChooseTargets { .. }));
    assert!(
        !engine.library_reveal_blocked().contains(me),
        "announcing the already visible top does not hide that same card"
    );
    let mine = seen_by(&engine, me);
    assert_eq!(mine.library_tops.len(), 1);
    assert_eq!(mine.library_tops[0].id, bolt);
    assert!(mine.library_tops[0].card.is_some());
    assert!(seen_by(&engine, them).library_tops.is_empty());

    // Harness intervention during the real, still-open casting procedure:
    // changing the top must not reveal the newly exposed card mid-announcement.
    engine
        .dev_state_mut(me)
        .unwrap()
        .move_object(
            bolt,
            ZoneLocation::Graveyard(me),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    assert!(engine.library_reveal_blocked().contains(me));
    assert!(
        seen_by(&engine, me).library_tops.is_empty(),
        "the newly exposed Ritual waits until the cast is complete"
    );
    assert!(seen_by(&engine, them).library_tops.is_empty());
    let spectator = spectator_view(
        engine.state(),
        1,
        None,
        SeatSet::new(),
        engine.library_reveal_blocked(),
        &[],
        &[],
    );
    assert!(spectator.library_tops.is_empty());
}
