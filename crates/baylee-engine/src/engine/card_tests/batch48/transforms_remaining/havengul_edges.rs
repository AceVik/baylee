#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn havengul_two_sacrificed_clues_plus_destroyed_clue_do_not_meet_three_sacrifices() {
    let mut e = prepared(&[index::DISENCHANT], &[index::HAVENGUL_LABORATORY], &[]);
    let lab = source(&e, P0, index::HAVENGUL_LABORATORY);
    for n in 0..3 {
        e.state.set_tapped(lab, false);
        e.refresh_offer();
        ability(&mut e, P0, index::HAVENGUL_LABORATORY, 1);
        pass_until(&mut e, stack_is_empty);
        let clue = e
            .state
            .battlefield_seen()
            .find(|id| {
                e.state.object(*id).is_some_and(|o| {
                    o.characteristics()
                        .subtypes
                        .contains(subtypes::artifact::CLUE)
                })
            })
            .unwrap();
        if n < 2 {
            priority(&mut e, P0);
            e.state.players[0].mana_pool.add(ManaColor::Colorless, 2);
            e.refresh_offer();
            e.apply(
                P0,
                PlayerAction::ActivateAbility {
                    source: clue,
                    ability_index: 0,
                },
            )
            .unwrap();
        } else {
            cast(&mut e, index::DISENCHANT);
            targets(&mut e, vec![clue], vec![]);
        }
        pass_until(&mut e, stack_is_empty);
    }
    pass_until(&mut e, |e| {
        e.state.turn.step == crate::turn::Step::End
            && stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { .. })
    });
    assert_eq!(
        face(&e, lab),
        0,
        "three departed Clues are not three sacrifices"
    );
}

#[test]
fn havengul_printed_departure_trigger_uses_current_mystery_controller() {
    let (mut e, lab, bear) = super::havengul::linked_bear();
    // Isolated land control change, after real reanimation. The printed
    // observer belongs to current Mystery, not original reanimation caster.
    e.state.object_mut(lab).unwrap().set_controller(P1);
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
    cast(&mut e, index::UNSUMMON);
    targets(&mut e, vec![bear], vec![]);
    pass_until(&mut e, |e| {
        e.state.object(bear).is_some_and(|o| o.zone == Zone::Hand)
            && !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { .. })
    });
    let trigger = *e.state.zones.list(ZoneLocation::Stack).last().unwrap();
    assert_eq!(e.state.object(trigger).unwrap().controller, P1);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(face(&e, lab), 0);
}

#[test]
fn havengul_copied_reanimation_links_both_creatures_but_front_face_does_not_observe() {
    let mut e = prepared(
        &[
            index::GRIZZLY_BEARS,
            index::SERRA_ANGEL,
            index::VIRTUE_OF_KNOWLEDGE,
            index::UNSUMMON,
            index::UNSUMMON,
        ],
        &[index::HAVENGUL_LABORATORY, index::TIRELESS_TRIBE],
        &[],
    );
    let lab = source(&e, P0, index::HAVENGUL_LABORATORY);
    let bear = in_hand(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let angel = in_hand(&e, P0, index::SERRA_ANGEL).unwrap();
    for card in [bear, angel] {
        ability(&mut e, P0, index::TIRELESS_TRIBE, 0);
        choose(&mut e, vec![card]);
        pass_until(&mut e, stack_is_empty);
    }
    for _ in 0..3 {
        e.state.set_tapped(lab, false);
        e.refresh_offer();
        ability(&mut e, P0, index::HAVENGUL_LABORATORY, 1);
        pass_until(&mut e, stack_is_empty);
        let clue = e
            .state
            .battlefield_seen()
            .find(|id| {
                e.state.object(*id).is_some_and(|o| {
                    o.characteristics()
                        .subtypes
                        .contains(subtypes::artifact::CLUE)
                })
            })
            .unwrap();
        priority(&mut e, P0);
        e.state.players[0].mana_pool.add(ManaColor::Colorless, 2);
        e.refresh_offer();
        e.apply(
            P0,
            PlayerAction::ActivateAbility {
                source: clue,
                ability_index: 0,
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
    }
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    targets(&mut e, vec![bear], vec![]);
    let original = *e.state.zones.list(ZoneLocation::Stack).last().unwrap();
    priority(&mut e, P0);
    for color in ManaColor::ALL {
        e.state.players[0].mana_pool.add(color, 30);
    }
    e.refresh_offer();
    let visions = in_hand(&e, P0, index::VIRTUE_OF_KNOWLEDGE).unwrap();
    e.apply(P0, PlayerAction::CastSpell { card: visions })
        .unwrap();
    if let Pending::ChooseCastMode { options, .. } = e.pending() {
        let mode = options
            .iter()
            .position(|o| matches!(o.kind, crate::choice::CastModeKind::Face(1)))
            .unwrap();
        e.apply(P0, PlayerAction::ChooseMode(mode)).unwrap();
    }
    targets(&mut e, vec![original], vec![]);
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::ChooseTargets {
                reason: crate::choice::TargetPrompt::Retarget { .. },
                ..
            }
        )
    });
    targets(&mut e, vec![angel], vec![]);
    pass_until(&mut e, stack_is_empty);
    for card in [bear, angel] {
        assert_eq!(e.state.object(card).unwrap().zone, Zone::Battlefield);
    }
    // The first creature returned (copy=Angel) is still linked after original
    // returns Bear: keeping only the most recent return would miss this.
    cast(&mut e, index::UNSUMMON);
    targets(&mut e, vec![angel], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(face(&e, lab), 0);
    cast(&mut e, index::UNSUMMON);
    targets(&mut e, vec![bear], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        face(&e, lab),
        0,
        "front has no printed linked observer, despite older linked creature leaving"
    );
}
