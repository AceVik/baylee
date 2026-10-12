//! Child module of `sacrifice_outcomes`. Explicit retained-DSL integration fixture.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::{
    AbilityDef, Amount, Effect, Filter, PlayerRel, StepKind, TargetSpec, Trigger,
};

static RETAINED_ARM: &[AbilityDef] = &[baylee_cards_dsl::triggered!(
    Trigger::StepBegin {
        step: StepKind::Upkeep,
        whose: PlayerRel::You
    },
    &[Effect::SacrificeOneThen {
        filter: &Filter::LAND,
        if_it_was: &Filter::LAND,
        then: &[Effect::DealDamage {
            amount: Amount::Fixed(3),
            target: TargetSpec::Player(PlayerRel::You)
        }],
    }]
)];

#[test]
fn retained_sacrifice_arm_preserves_token_identity_and_sacrifice_provenance() {
    for (token, exile) in [(false, false), (true, false), (false, true)] {
        let opponents = if exile {
            vec![index::DAUTHI_VOIDWALKER]
        } else {
            vec![]
        };
        let mut e = prepared(&[index::SERENDIB_DJINN], &[index::ISLAND], &opponents);
        cast_with_floating(&mut e, P0, index::SERENDIB_DJINN);
        pass_until(&mut e, stack_is_empty);
        let source = on_battlefield(&e, P0, index::SERENDIB_DJINN).unwrap();
        let victim = on_battlefield(&e, P0, index::ISLAND).unwrap();
        // Only the body is synthetic: exercise retained `SacrificeOneThen` rather
        // than the completed Djinn's equivalent SacrificeAndIf implementation.
        let acquisition = e.state.next_timestamp();
        e.state
            .object_mut(source)
            .unwrap()
            .take_abilities(crate::object::AbilityList {
                acquisition,
                abilities: crate::copiable_abilities::AbilityDefs::from_static(RETAINED_ARM),
                printed: None,
                token: None,
            });
        if token {
            // Explicit token-copy boundary fixture, retaining Island characteristics.
            e.state.object_mut(victim).unwrap().card = None;
        }
        e.state.invalidate_projections();
        e.sync_static_effects();
        e.refresh_offer();
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
        });
        let before = e.state.players[0].life;
        let hash = e.state.snapshot_hash();
        // An unoffered object cannot count as a sacrifice or run the paid branch.
        assert!(
            e.apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: vec![source]
                }
            )
            .is_err()
        );
        assert_eq!(e.state.snapshot_hash(), hash);
        e.apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state.players[0].life, before - 3);
        assert!(e.journal().entries().iter().any(|entry| {
            matches!(entry.event, crate::event::GameEvent::ZoneChanged { object, from: Zone::Battlefield, .. } if object == victim)
                && entry.departure.as_ref().is_some_and(|departure| departure.sacrificed)
        }));
        if token {
            assert!(
                e.state.object(victim).is_none(),
                "token must cease to exist after leaving battlefield"
            );
        } else {
            assert_eq!(
                e.state.object(victim).unwrap().zone,
                if exile { Zone::Exile } else { Zone::Graveyard }
            );
        }
    }
}
