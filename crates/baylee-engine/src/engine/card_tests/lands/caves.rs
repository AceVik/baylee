//! The tests of the cards under `cards/lands/caves/`, one module per card.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

mod captivating_cave;
mod cavernous_maw;
mod echoing_deeps;
mod forgotten_monument;
mod hidden_cataract;
mod hidden_courtyard;
mod hidden_necropolis;
mod hidden_nursery;
mod hidden_volcano;
mod pit_of_offerings;
mod secret_tunnel;
mod sunken_citadel;
mod sunken_palace;
mod urza_s_cave;
mod volatile_fault;

/// Exercise each colored discover activation through its paid stack ability.
fn hidden_cave_discovers(card: CardIndex, color: ManaColor, cast: bool) {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[card])
        .hand(
            0,
            &[index::GRIZZLY_BEARS, index::SERRA_ANGEL, index::MOUNTAIN],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let cave = on_battlefield(&e, p0, card).unwrap();
    let bear = hand_to_library_top(&mut e, p0, index::GRIZZLY_BEARS);
    let angel = hand_to_library_top(&mut e, p0, index::SERRA_ANGEL);
    let mountain = hand_to_library_top(&mut e, p0, index::MOUNTAIN);
    let state = e.dev_state_mut(p0).unwrap();
    state.set_tapped(cave, false);
    state.players[0].mana_pool.add(color, 5);
    e.refresh_offer();
    activate(&mut e, p0, card, 1);
    assert_eq!(e.state().object(cave).unwrap().zone, Zone::Graveyard);
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::Discover { .. },
                ..
            }
        )
    });
    assert!(
        matches!(e.pending(), Pending::YesNo { prompt: crate::choice::YesNoPrompt::Discover { card }, .. } if *card==bear)
    );
    let library = e.state().zones.list(ZoneLocation::Library(p0));
    assert!(library[..2].contains(&angel) && library[..2].contains(&mountain));
    e.apply(p0, PlayerAction::YesNo(cast)).unwrap();
    if cast {
        assert_eq!(e.state().object(bear).unwrap().zone, Zone::Stack);
    }
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().object(bear).unwrap().zone,
        if cast { Zone::Battlefield } else { Zone::Hand }
    );
}

/// "{4}{N}, {T}, Sacrifice this land: Discover 4. Activate only as a
/// sorcery." The five mana are in the pool every time, so what refuses the
/// ability is the clause under test: a spell on the stack, the opponent's
/// turn, a tapped Cave. Each refusal changes nothing, and the same board is
/// offered the ability once the one thing in the way is lifted.
fn hidden_cave_refuses_discover(card: CardIndex, color: ManaColor) {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let offered = |e: &Engine<RegistryLookup>, cave: ObjectId| matches!(e.pending(), Pending::Priority { legal, .. } if legal.abilities.contains(&(cave, 1)));
    let refused = |e: &mut Engine<RegistryLookup>, cave: ObjectId, why: &str| {
        let before = e.snapshot_hash();
        assert!(
            e.apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: cave,
                    ability_index: 1,
                },
            )
            .is_err(),
            "{why}"
        );
        assert_eq!(
            e.snapshot_hash(),
            before,
            "{why}: the refusal changed nothing"
        );
    };

    // A spell on the stack, in the Cave's controller's own main phase.
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[card])
        .hand(0, &[index::SOL_RING])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let cave = on_battlefield(&e, p0, card).unwrap();
    let state = e.dev_state_mut(p0).unwrap();
    state.set_tapped(cave, false);
    state.players[0].mana_pool.add(color, 6);
    e.refresh_offer();
    assert!(
        offered(&e, cave),
        "own main phase, empty stack, untapped Cave, {{4}}{{N}} in the pool: offered"
    );
    cast_with_floating(&mut e, p0, index::SOL_RING);
    assert!(!stack_is_empty(&e), "Sol Ring is on the stack");
    assert!(!is_tapped(&e, cave), "the Cave is untapped");
    assert!(
        e.state().players[0].mana_pool.available(color) >= 5,
        "the cost is still in the pool after Sol Ring's {{1}}"
    );
    assert!(!offered(&e, cave), "not offered with a spell on the stack");
    refused(
        &mut e,
        cave,
        "\"Activate only as a sorcery\": a spell is on the stack",
    );
    pass_until(&mut e, stack_is_empty);
    assert!(
        offered(&e, cave),
        "the stack empty again, the same mana and Cave are offered the ability"
    );

    // The opponent's turn: the Cave's controller holds priority in their main phase.
    let mut e = Duel::new(SEED, forest()).battlefield(0, &[card]).start();
    keep_mulligans(&mut e);
    reach_their_main_phase(&mut e, p1);
    let cave = on_battlefield(&e, p0, card).unwrap();
    let Pending::Priority { player, .. } = e.pending().clone() else {
        panic!("expected priority, got {:?}", e.pending())
    };
    assert_eq!(player, p1, "the active player has priority first");
    e.apply(p1, PlayerAction::PassPriority).unwrap();
    assert!(
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
        "then the Cave's controller does: {:?}",
        e.pending()
    );
    assert_eq!(e.state().turn.active, p1);
    let state = e.dev_state_mut(p0).unwrap();
    state.set_tapped(cave, false);
    state.players[0].mana_pool.add(color, 5);
    e.refresh_offer();
    assert!(!offered(&e, cave), "not offered on the opponent's turn");
    refused(
        &mut e,
        cave,
        "\"Activate only as a sorcery\": it is the opponent's turn",
    );

    // The Cave tapped: {T} is part of the cost.
    let mut e = Duel::new(SEED, forest()).battlefield(0, &[card]).start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let cave = on_battlefield(&e, p0, card).unwrap();
    let state = e.dev_state_mut(p0).unwrap();
    state.set_tapped(cave, false);
    state.players[0].mana_pool.add(color, 5);
    e.refresh_offer();
    assert!(offered(&e, cave), "untapped, it is offered");
    e.dev_state_mut(p0).unwrap().set_tapped(cave, true);
    e.refresh_offer();
    assert!(!offered(&e, cave), "not offered with the Cave tapped");
    refused(
        &mut e,
        cave,
        "\"{{T}}\" is in the cost and the Cave is tapped",
    );
}
