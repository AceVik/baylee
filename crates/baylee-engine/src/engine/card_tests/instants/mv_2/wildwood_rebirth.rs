//! Oracle: return target creature card from your graveyard to your hand.
#[allow(clippy::wildcard_imports)]
use super::*;
fn wildwood_rebirth() -> CardIndex {
    card_index("7a4ddb8f-08ad-4bf9-82f6-72b073dd859a")
}
/// Checklist: only a creature card in your graveyard is offered and returns to hand, not battlefield.
#[test]
fn returns_creature_card_from_graveyard_to_hand() {
    let p = PlayerId::new(0);
    let mut g = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[wildwood_rebirth()])
        .start();
    keep_mulligans(&mut g);
    reach_main_phase(&mut g, p);
    let c = on_battlefield(&g, p, llanowar_elves()).unwrap();
    let foe_creature = on_battlefield(&g, PlayerId::new(1), llanowar_elves()).unwrap();
    let noncreature = g
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| {
            g.state().object(*id).is_some_and(|o| {
                o.controller == p && o.card.is_some_and(|card| card.index == forest())
            })
        })
        .unwrap();
    g.dev_state_mut(p)
        .unwrap()
        .move_object(
            c,
            ZoneLocation::Graveyard(p),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    g.dev_state_mut(p)
        .unwrap()
        .move_object(
            noncreature,
            ZoneLocation::Graveyard(p),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    g.dev_state_mut(p)
        .unwrap()
        .move_object(
            foe_creature,
            ZoneLocation::Graveyard(PlayerId::new(1)),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    g.refresh_offer();
    cast_from_hand(&mut g, p, wildwood_rebirth());
    let Pending::ChooseTargets { options, .. } = g.pending().clone() else {
        panic!("graveyard target")
    };
    assert_eq!(
        options,
        vec![c],
        "noncreature and opponent graveyard cards are excluded"
    );
    g.apply(
        p,
        PlayerAction::ChooseTargets {
            objects: vec![c],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut g, stack_is_empty);
    assert!(in_hand(&g, p, llanowar_elves()).is_some());
    assert!(on_battlefield(&g, p, llanowar_elves()).is_none());
}
