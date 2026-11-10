//! `cards/creatures/mv_1/vigilant_martyr.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn vigilant_martyr() -> CardIndex {
    card_index("122dfe9f-e0e2-4f69-9637-289c956fd54a")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Vigilant Martyr (`{W}`, `Coverage::Partial`): "Sacrifice this creature:
/// Regenerate target creature." Any creature is a target, the Martyr is gone
/// as the cost, and the shield lands on the target only. The second ability
/// (counter a spell that targets an enchantment) is not played, so it must not
/// be offered either: a plain counter would hit any spell.
#[test]
fn vigilant_martyr_sacrifices_to_regenerate_the_target_creature_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[vigilant_martyr(), grizzly_bears()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let martyr = on_battlefield(&engine, p0, vigilant_martyr()).unwrap();
    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let their_bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    assert!(
        ability_offered(&engine, martyr, 0),
        "the regeneration is offered"
    );
    assert!(
        !ability_offered(&engine, martyr, 1),
        "the counter ability is not played"
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: martyr,
                ability_index: 0,
            },
        )
        .unwrap();
    let menu = aim_at(&mut engine, p0, bear);
    assert!(
        menu.contains(&their_bear),
        "any creature may be named, an opponent's too"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, vigilant_martyr()).is_none(),
        "the Martyr is sacrificed"
    );
    assert!(in_graveyard(&engine, p0, vigilant_martyr()).is_some());
    assert_eq!(engine.state().object(bear).unwrap().regeneration_shields, 1);
    assert_eq!(
        engine
            .state()
            .object(their_bear)
            .unwrap()
            .regeneration_shields,
        0,
        "the other Bears has no shield"
    );
}
