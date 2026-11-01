//! Locked activation prices and independently ordered generic reductions.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn batch47_power_artifact_activation_price_stays_locked_through_mana_payment() {
    let mut e = prepared(
        &[index::POWER_ARTIFACT],
        &[
            index::CANDELABRA_OF_TAWNOS,
            index::FOREST,
            index::FOREST,
            index::FOREST,
        ],
        &[],
    );
    let artifact = on_battlefield(&e, P0, index::CANDELABRA_OF_TAWNOS).unwrap();
    cast_at(&mut e, index::POWER_ARTIFACT, vec![artifact]);
    let aura = on_battlefield(&e, P0, index::POWER_ARTIFACT).unwrap();
    let lands: Vec<_> = e
        .state
        .battlefield_seen()
        .filter(|id| {
            e.state.object(*id).is_some_and(|object| {
                object.controller == P0
                    && object.card.is_some_and(|card| card.index == index::FOREST)
            })
        })
        .collect();
    assert_eq!(lands.len(), 3);
    set_pool(&mut e, P0, &[]);
    activate_as(&mut e, P0, artifact, 0);
    e.apply(P0, PlayerAction::ChooseNumber(3)).unwrap();
    targets(&mut e, lands.clone(), vec![]);
    assert!(matches!(e.pending(), Pending::Priority { .. }));
    assert!(
        e.mana_window.is_some(),
        "announced X awaits its locked one mana"
    );

    // Isolate the live-effect change during payment. A mana ability can remove
    // an animated aura as a sacrifice cost; its animation is irrelevant to
    // this regression. Moving it directly avoids adding unrelated card rules.
    e.state
        .move_object(
            aura,
            ZoneLocation::Graveyard(P0),
            ZonePosition::Top,
            Cause::Cost,
        )
        .unwrap();
    e.apply(P0, PlayerAction::ActivateManaAbility { source: lands[0] })
        .unwrap();
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    assert!(e.mana_window.is_none(), "the locked cost was paid in full");
    let stack = e.state.zones.list(ZoneLocation::Stack);
    let ability = e
        .state
        .object(*stack.last().expect("activation reached the stack"))
        .unwrap();
    let paid = ability.paid.as_ref().expect("activation receipt");
    assert_eq!(paid.mana_paid.total(), 1);
    assert_eq!(paid.fixed_mana_cost, baylee_core::mana::ManaCost::ZERO);
    assert_eq!(e.state.players[0].mana_pool.total(), 0);
    pass_until(&mut e, stack_is_empty);
    assert!(lands.iter().all(|land| !is_tapped(&e, *land)));
}

#[test]
fn batch47_power_artifact_floor_does_not_limit_an_independent_later_reduction() {
    use baylee_cards_dsl::{Amount, Cost, CostReduction};
    let mut e = prepared(
        &[index::POWER_ARTIFACT],
        &[index::CANDELABRA_OF_TAWNOS],
        &[],
    );
    let artifact = on_battlefield(&e, P0, index::CANDELABRA_OF_TAWNOS).unwrap();
    cast_at(&mut e, index::POWER_ARTIFACT, vec![artifact]);
    // No registered artifact combines its own unrestricted reduction with
    // Power Artifact. Supply only that independent printed reduction here.
    let reduction = Some(CostReduction::PerCount {
        amount: Amount::Fixed(1),
        each: 1,
    });
    let raw = Cost {
        mana: baylee_core::mana::ManaCost::parse("{3}"),
        parts: &[],
    };
    assert_eq!(
        e.activation_price(P0, artifact, 0, &raw, None)
            .mana
            .generic_total(),
        1
    );
    assert_eq!(
        e.activation_price(P0, artifact, 0, &raw, reduction)
            .mana
            .generic_total(),
        0,
        "apply Power Artifact first, then the independent unrestricted reduction"
    );
    let aura = on_battlefield(&e, P0, index::POWER_ARTIFACT).unwrap();
    e.state
        .move_object(
            aura,
            ZoneLocation::Graveyard(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    assert_eq!(
        e.activation_price(P0, artifact, 0, &raw, reduction)
            .mana
            .generic_total(),
        2,
        "the unrestricted reduction alone removes exactly one mana"
    );
}
