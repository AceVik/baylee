//! Grouped legacy returns through actual spell and ability actions.
#[allow(clippy::wildcard_imports)]
use super::*;

fn group_stack(e: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    e.state.zones.list(ZoneLocation::Stack).clone()
}
fn group_priority(e: &mut Engine<RegistryLookup>, player: PlayerId) {
    pass_until(
        e,
        |e| matches!(e.pending(), Pending::Priority { player: p, .. } if *p == player),
    );
}
fn group_mana(e: &mut Engine<RegistryLookup>, player: PlayerId) {
    for color in ManaColor::ALL {
        e.state.players[player.get() as usize]
            .mana_pool
            .add(color, 30);
    }
    e.refresh_offer();
}
fn interlude(e: &mut Engine<RegistryLookup>, player: PlayerId, objects: Vec<ObjectId>) {
    group_priority(e, player);
    group_mana(e, player);
    let card = in_hand(e, player, index::EERIE_INTERLUDE).unwrap();
    e.apply(player, PlayerAction::CastSpell { card }).unwrap();
    targets(e, objects);
    pass_until(e, stack_is_empty);
}
fn group_end(e: &mut Engine<RegistryLookup>) {
    pass_until(e, |e| {
        e.state.turn.step == crate::turn::Step::End
            && !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { .. })
    });
}
fn tidebinder(e: &mut Engine<RegistryLookup>, target: ObjectId) {
    group_priority(e, P0);
    group_mana(e, P0);
    let card = in_hand(e, P0, index::TISHANA_S_TIDEBINDER).unwrap();
    e.apply(P0, PlayerAction::CastSpell { card }).unwrap();
    pass_until(e, |e| matches!(e.pending(), Pending::ChooseTargets { .. }));
    targets(e, vec![target]);
}

#[test]
fn eerie_multiple_cards_share_one_delayed_trigger_and_one_resolution() {
    let mut e = prepared(
        &[index::EERIE_INTERLUDE],
        &[index::GRIZZLY_BEARS, index::LLANOWAR_ELVES],
        &[],
    );
    let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let elf = on_battlefield(&e, P0, index::LLANOWAR_ELVES).unwrap();
    interlude(&mut e, P0, vec![bear, elf]);
    for id in [bear, elf] {
        assert_eq!(e.state.object(id).unwrap().zone, Zone::Exile);
    }
    group_end(&mut e);
    assert_eq!(
        group_stack(&e).len(),
        1,
        "one delayed ability for the entire spell's group"
    );
    pass_until(&mut e, stack_is_empty);
    for id in [bear, elf] {
        assert_eq!(e.state.object(id).unwrap().zone, Zone::Battlefield);
        assert_eq!(e.state.object(id).unwrap().controller, P0);
    }
    assert_eq!(e.state.turn.step, crate::turn::Step::End);
}

#[test]
fn countering_eerie_return_cancels_entire_group_permanently() {
    let mut e = prepared(
        &[index::EERIE_INTERLUDE, index::TISHANA_S_TIDEBINDER],
        &[index::GRIZZLY_BEARS, index::LLANOWAR_ELVES],
        &[],
    );
    let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let elf = on_battlefield(&e, P0, index::LLANOWAR_ELVES).unwrap();
    interlude(&mut e, P0, vec![bear, elf]);
    group_end(&mut e);
    let stack = group_stack(&e);
    assert_eq!(stack.len(), 1);
    tidebinder(&mut e, stack[0]);
    pass_until(&mut e, stack_is_empty);
    for id in [bear, elf] {
        assert_eq!(e.state.object(id).unwrap().zone, Zone::Exile);
    }
    let turn = e.state.turn.number;
    main_after(&mut e, P0, turn);
    let turn = e.state.turn.number;
    main_after(&mut e, P1, turn);
    // The next end has no retry. No unrelated end trigger exists in this fixture.
    for id in [bear, elf] {
        assert_eq!(e.state.object(id).unwrap().zone, Zone::Exile);
    }
}

#[test]
fn copying_eerie_return_retains_entire_group_after_original_is_countered() {
    let mut e = prepared(
        &[
            index::EERIE_INTERLUDE,
            index::VIRTUE_OF_KNOWLEDGE,
            index::TISHANA_S_TIDEBINDER,
        ],
        &[index::GRIZZLY_BEARS, index::LLANOWAR_ELVES],
        &[],
    );
    let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let elf = on_battlefield(&e, P0, index::LLANOWAR_ELVES).unwrap();
    interlude(&mut e, P0, vec![bear, elf]);
    group_end(&mut e);
    let original = group_stack(&e)[0];
    group_priority(&mut e, P0);
    group_mana(&mut e, P0);
    let visions = in_hand(&e, P0, index::VIRTUE_OF_KNOWLEDGE).unwrap();
    e.apply(P0, PlayerAction::CastSpell { card: visions })
        .unwrap();
    if let Pending::ChooseCastMode { options, .. } = e.pending() {
        let face = options
            .iter()
            .position(|o| matches!(o.kind, crate::choice::CastModeKind::Face(1)))
            .unwrap();
        e.apply(P0, PlayerAction::ChooseMode(face)).unwrap();
    }
    targets(&mut e, vec![original]);
    pass_until(&mut e, |e| {
        e.state
            .object(visions)
            .is_some_and(|o| o.zone == Zone::Exile)
            && group_stack(e).len() == 2
            && matches!(e.pending(), Pending::Priority { .. })
    });
    tidebinder(&mut e, original);
    pass_until(&mut e, |e| !group_stack(e).contains(&original));
    for id in [bear, elf] {
        assert_eq!(
            e.state.object(id).unwrap().zone,
            Zone::Exile,
            "counter resolves above the copied return"
        );
    }
    pass_until(&mut e, stack_is_empty);
    for id in [bear, elf] {
        assert_eq!(
            e.state.object(id).unwrap().zone,
            Zone::Battlefield,
            "copied related-object payload retains every exact reference"
        );
    }
}

#[test]
fn ordinary_end_trigger_and_both_players_delayed_groups_form_one_apnap_batch() {
    let mut e = Duel::new(SEED, index::FOREST)
        .hand(0, &[index::EERIE_INTERLUDE])
        .hand(1, &[index::EERIE_INTERLUDE])
        .battlefield(0, &[index::GRIZZLY_BEARS, index::NIM_ABOMINATION])
        .battlefield(1, &[index::LLANOWAR_ELVES])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let elf = on_battlefield(&e, P1, index::LLANOWAR_ELVES).unwrap();
    interlude(&mut e, P0, vec![bear]);
    interlude(&mut e, P1, vec![elf]);
    group_end(&mut e);
    let stack = group_stack(&e);
    assert_eq!(
        stack.len(),
        3,
        "ordinary and both delayed triggers all wait for the same priority grant"
    );
    let controllers: Vec<_> = stack
        .iter()
        .map(|id| e.state.object(*id).unwrap().controller)
        .collect();
    assert_eq!(
        controllers,
        vec![P0, P0, P1],
        "bottom to top: active player's two triggers, then nonactive player's trigger"
    );
    let life = e.state.players[0].life;
    pass_until(&mut e, |e| {
        e.state
            .object(elf)
            .is_some_and(|o| o.zone == Zone::Battlefield)
    });
    assert_eq!(
        e.state.object(bear).unwrap().zone,
        Zone::Exile,
        "nonactive player's group resolves first"
    );
    assert_eq!(e.state.players[0].life, life);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(bear).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state.players[0].life, life - 3);
}
