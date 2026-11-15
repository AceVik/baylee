//! A paid action is public; the owner's remaining run stock is not.
use super::*;
use baylee_core::preset::SeatSupplies;

#[test]
fn private_inventory_is_absent_from_opponents_spectators_and_counter_logs() {
    let owner = PlayerId::new(0);
    let opponent = PlayerId::new(1);
    let mut preset = baylee_cards::decks::probe_preset(71, island()).unwrap();
    preset.format = FormatId::Custom;
    preset.house_rules.starting_player = Some(0);
    preset.house_rules.supplies = vec![SeatSupplies {
        seat: 0,
        healing: 2,
        mana: 0,
        seedling: false,
        concealed: true,
    }];
    let mut engine = Engine::new(&preset, crate::RegistryLookup).unwrap();
    while let Pending::Mulligan { player, .. } = *engine.pending() {
        engine.apply(player, PlayerAction::MulliganKeep).unwrap();
    }
    let context = SeatContext::default();
    let own = player_view(engine.state(), owner, 0, None, &context, &[]);
    assert_eq!(
        own.command[0].len(),
        2,
        "empty and nonempty inventory share a fixed shape"
    );
    let source = own.command[0][0].id;
    assert_eq!(
        own.command[0][0].counter_count(baylee_view::CounterKind::Charge),
        2
    );
    let other = player_view(engine.state(), opponent, 0, None, &context, &[]);
    assert!(other.command[0].is_empty());
    assert!(public_object(&ViewBoard::new(engine.state()), source, opponent).is_none());
    assert!(visible_event_object(engine.state(), source, opponent).is_none());
    let public = spectator_view(
        engine.state(),
        0,
        None,
        SeatSet::new(),
        SeatSet::new(),
        &[],
        &[],
    );
    assert!(public.command[0].is_empty());
    let controlled = SeatContext {
        controlled_players: std::iter::once(owner).collect(),
        ..SeatContext::default()
    };
    assert_eq!(
        player_view(engine.state(), opponent, 0, None, &controlled, &[]).command[0].len(),
        2
    );

    let mut log = crate::log::GameLog::new(engine.state());
    let mut delayed_log = crate::log::GameLog::new(engine.state());
    engine
        .apply(
            owner,
            PlayerAction::ActivateAbility {
                source,
                ability_index: 0,
            },
        )
        .unwrap();
    log.consume(engine.state());
    let other = player_view(engine.state(), opponent, 1, None, &context, &[]);
    assert!(other.command[0].is_empty());
    assert_eq!(
        other.stack.len(),
        1,
        "the paid effect remains public on the stack"
    );
    assert_eq!(other.stack[0].name, "Tower Healing Potion");
    assert!(
        log.told(opponent, 0, log.len())
            .iter()
            .all(|line| !matches!(line.event, baylee_view::LogEvent::Counters { .. }))
    );
    assert_eq!(
        player_view(engine.state(), owner, 1, None, &context, &[]).command[0][0]
            .counter_count(baylee_view::CounterKind::Charge),
        1
    );
    engine.apply(owner, PlayerAction::Concede).unwrap();
    delayed_log.consume(engine.state());
    assert!(
        delayed_log
            .told(opponent, 0, delayed_log.len())
            .iter()
            .all(|line| !matches!(line.event, baylee_view::LogEvent::Counters { .. })),
        "batched consumption must remain private after its owner leaves"
    );
}
