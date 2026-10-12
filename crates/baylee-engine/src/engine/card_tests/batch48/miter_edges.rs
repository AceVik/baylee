//! Additional acceptance edges for this card.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn miter_rewards_a_destroyed_artifact_token() {
    let mut e = prepared(
        &[index::SHATTER],
        &[
            index::URZA_S_MITER,
            index::STORM_THE_VAULT,
            index::GRIZZLY_BEARS,
        ],
        &[],
    );
    let bear = source(&e, P0, index::GRIZZLY_BEARS);
    attackers(&mut e, &[bear]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut e, |e| {
        e.state.turn.step == crate::turn::Step::CombatEnd && stack_is_empty(e)
    });
    let treasure = e
        .state
        .battlefield_seen()
        .find(|id| {
            e.state.object(*id).is_some_and(|o| {
                o.characteristics()
                    .subtypes
                    .contains(subtypes::artifact::TREASURE)
            })
        })
        .unwrap();
    let library = library_size(&e, P0);
    cast(&mut e, index::SHATTER);
    targets(&mut e, vec![treasure], vec![]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    yes(&mut e, true);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(library_size(&e, P0), library - 1);
    assert_eq!(token_count(&e, subtypes::artifact::TREASURE), 0);
}

#[test]
fn miter_does_not_reward_an_artifact_sacrificed_by_a_resolving_spell() {
    let mut e = prepared(
        &[index::TRANSMUTE_ARTIFACT, index::MANA_VAULT],
        &[index::URZA_S_MITER, index::SOL_RING],
        &[],
    );
    let found = in_hand(&e, P0, index::MANA_VAULT).unwrap();
    e.state
        .move_object(
            found,
            ZoneLocation::Library(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    let ring = source(&e, P0, index::SOL_RING);
    let library = library_size(&e, P0);
    cast(&mut e, index::TRANSMUTE_ARTIFACT);
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::CostSacrifice,
                ..
            }
        )
    });
    choose(&mut e, vec![ring]);
    choose(&mut e, vec![found]);
    // An erroneous Miter trigger would stop this helper at its payment prompt.
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(ring).unwrap().zone, Zone::Graveyard);
    assert_eq!(e.state.object(found).unwrap().zone, Zone::Battlefield);
    assert_eq!(library_size(&e, P0), library - 1);
}

#[test]
fn miter_observes_its_own_nonsacrifice_death_using_departure_rules() {
    let mut e = prepared(&[index::SHATTER], &[index::URZA_S_MITER], &[]);
    let miter = source(&e, P0, index::URZA_S_MITER);
    let library = library_size(&e, P0);
    cast(&mut e, index::SHATTER);
    targets(&mut e, vec![miter], vec![]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    yes(&mut e, true);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(miter).unwrap().zone, Zone::Graveyard);
    assert_eq!(library_size(&e, P0), library - 1);
}

#[test]
fn miter_distinguishes_lethal_damage_death_from_exile_replacement() {
    for exile in [false, true] {
        let spell = if exile {
            index::DISINTEGRATE
        } else {
            index::LIGHTNING_BOLT
        };
        let mut e = prepared(&[spell], &[index::URZA_S_MITER, index::ORNITHOPTER], &[]);
        let thopter = source(&e, P0, index::ORNITHOPTER);
        let library = library_size(&e, P0);
        cast(&mut e, spell);
        if exile {
            e.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
        }
        targets(&mut e, vec![thopter], vec![]);
        if !exile {
            pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
            yes(&mut e, true);
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state.object(thopter).unwrap().zone,
            if exile { Zone::Exile } else { Zone::Graveyard }
        );
        assert_eq!(library_size(&e, P0), library - usize::from(!exile));
    }
}
