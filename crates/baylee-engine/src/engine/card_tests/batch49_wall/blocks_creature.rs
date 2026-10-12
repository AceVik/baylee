//! Multiple blocked creatures and the distinction between blocking and being blocked.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn wall_blocks_two_creatures_with_independent_recipients_and_copied_trigger() {
    let mut e = prepared(
        &[index::VIRTUE_OF_KNOWLEDGE],
        &[index::WALL_OF_DUST],
        &[index::GRIZZLY_BEARS, index::GRIZZLY_BEARS],
    );
    let wall = on_battlefield(&e, P0, index::WALL_OF_DUST).unwrap();
    let bears: Vec<_> = e
        .state
        .battlefield_seen()
        .filter(|id| {
            e.state.object(*id).is_some_and(|o| {
                o.controller == P1 && o.card.is_some_and(|c| c.index == index::GRIZZLY_BEARS)
            })
        })
        .collect();
    allow_additional_block(&mut e, wall);
    next_main(&mut e, P1);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { player: P1, .. })
    });
    e.apply(
        P1,
        PlayerAction::DeclareAttackers {
            attackers: bears.iter().map(|id| (*id, Defender::Player(P0))).collect(),
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { player: P0, .. })
    });
    e.apply(
        P0,
        PlayerAction::DeclareBlockers {
            blockers: bears.iter().map(|id| (wall, *id)).collect(),
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        !stack_is_empty(e) && matches!(e.pending(), Pending::Priority { .. })
    });
    let triggers = e.state.zones.list(ZoneLocation::Stack).clone();
    assert_eq!(triggers.len(), 2, "one trigger per blocked creature");
    let mut recipients: Vec<_> = triggers
        .iter()
        .map(|id| e.state.object(*id).unwrap().event_object.unwrap())
        .collect();
    recipients.sort_unstable();
    let mut expected = bears.clone();
    expected.sort_unstable();
    assert_eq!(recipients, expected);
    let original = triggers[0];
    let copied_recipient = e.state.object(original).unwrap().event_object.unwrap();
    copy_trigger(&mut e, original);
    assert_eq!(e.state.zones.list(ZoneLocation::Stack).len(), 3);
    let copied_count = e
        .state
        .zones
        .list(ZoneLocation::Stack)
        .iter()
        .filter(|id| e.state.object(**id).unwrap().event_object == Some(copied_recipient))
        .count();
    assert_eq!(
        copied_count, 2,
        "copied non-targeted trigger preserves its blocked creature"
    );
    pass_until(&mut e, stack_is_empty);
    let affected: Vec<_> = e
        .state
        .delayed
        .iter()
        .filter_map(|d| match d.action {
            crate::state::DelayedAction::RestrictAttacksForTurn { affected, .. } => {
                Some(affected.object)
            }
            _ => None,
        })
        .collect();
    assert_eq!(affected.len(), 3);
    for bear in bears {
        assert_eq!(
            affected.iter().filter(|id| **id == bear).count(),
            if bear == copied_recipient { 2 } else { 1 }
        );
    }
}

#[test]
fn attacking_wall_becoming_blocked_does_not_trigger_its_blocks_creature_ability() {
    let mut e = prepared(&[], &[index::WALL_OF_DUST], &[index::GRIZZLY_BEARS]);
    let wall = on_battlefield(&e, P0, index::WALL_OF_DUST).unwrap();
    let bear = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    // Transparent attack-permission fixture; the real card retains its ability.
    // No unimplemented attack-as-though-defender helper is used.
    let keywords = e
        .state
        .object(wall)
        .unwrap()
        .base
        .keywords
        .difference(baylee_cards_dsl::KeywordSet::DEFENDER);
    e.state.object_mut(wall).unwrap().base_mut().keywords = keywords;
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { player: P0, .. })
    });
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(wall, Defender::Player(P1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { player: P1, .. })
    });
    e.apply(
        P1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(bear, wall)],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| matches!(e.pending(), Pending::Priority { .. }));
    assert!(
        stack_is_empty(&e),
        "becoming blocked is not blocking a creature"
    );
    assert!(!e.state.delayed.iter().any(|d| matches!(
        d.action,
        crate::state::DelayedAction::RestrictAttacksForTurn { .. }
    )));
}

fn allow_additional_block(e: &mut Engine<RegistryLookup>, wall: ObjectId) {
    // Neither Brave the Sands nor High Ground has a registered implementation.
    // Install ONLY the additional-block capacity; declarations/triggers are real.
    let modifier = baylee_cards_dsl::Modifier::CanBlockAdditional(1);
    let version = e.state.object(wall).unwrap().version;
    let timestamp = e.state.next_timestamp();
    e.state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: None,
        controller: P0,
        origin: crate::effects::EffectOrigin::Resolution,
        layer: modifier.layer(),
        timestamp,
        duration: baylee_cards_dsl::Duration::Indefinitely,
        filter: crate::effects::EffectFilter::ObjectIs(wall, version),
        modifier,
    });
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
}

fn copy_trigger(e: &mut Engine<RegistryLookup>, original: ObjectId) {
    priority(e, P0);
    for color in ManaColor::ALL {
        e.state.players[0].mana_pool.add(color, 30);
    }
    e.refresh_offer();
    let visions = in_hand(e, P0, index::VIRTUE_OF_KNOWLEDGE).unwrap();
    e.apply(P0, PlayerAction::CastSpell { card: visions })
        .unwrap();
    if let Pending::ChooseCastMode { options, .. } = e.pending() {
        let face = options
            .iter()
            .position(|o| matches!(o.kind, crate::choice::CastModeKind::Face(1)))
            .unwrap();
        e.apply(P0, PlayerAction::ChooseMode(face)).unwrap();
    }
    e.apply(
        P0,
        PlayerAction::ChooseTargets {
            objects: vec![original],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(e, |e| {
        e.state
            .object(visions)
            .is_some_and(|o| o.zone == Zone::Exile)
            && matches!(e.pending(), Pending::Priority { .. })
    });
}
