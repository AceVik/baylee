//! `cards/instants/mv_1/seedling_charm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn seedling_charm() -> CardIndex {
    card_index("b8d66cd6-15db-41ff-bae3-60f1aad3ea23")
}

fn pacifism() -> CardIndex {
    baylee_core::generated::index::PACIFISM
}

/// Seedling Charm (`{G}`, `Coverage::Implemented`): modal instant that can
/// return target Aura attached to a creature to its owner's hand, regenerate
/// target green creature, or grant target creature trample until end of turn.
#[test]
fn seedling_charm_modes_bounce_aura_regenerate_green_and_grant_trample() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                forest(),
                forest(),
                forest(),
                grizzly_bears(),
            ],
        )
        .battlefield(1, &[mountain()])
        .hand(
            0,
            &[
                pacifism(),
                seedling_charm(),
                seedling_charm(),
                seedling_charm(),
            ],
        )
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let my_plains: Vec<ObjectId> = lands_of(&engine, p0)
        .into_iter()
        .filter(|&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == plains()))
        })
        .collect();
    let my_forests: Vec<ObjectId> = lands_of(&engine, p0)
        .into_iter()
        .filter(|&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == forest()))
        })
        .collect();

    // Cast Pacifism on Grizzly Bears using the two Plains.
    tap_mana_where(&mut engine, p0, |id| my_plains.contains(&id));
    cast_with_floating(&mut engine, p0, pacifism());
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);
    let aura = on_battlefield(&engine, p0, pacifism()).unwrap();

    // Mode 0: return target Aura attached to a creature to its owner's hand.
    tap_mana_where(&mut engine, p0, |id| id == my_forests[0]);
    cast_with_floating(&mut engine, p0, seedling_charm());
    engine.apply(p0, PlayerAction::ChooseMode(0)).unwrap();
    let menu = aim_at(&mut engine, p0, aura);
    assert_eq!(menu, vec![aura]);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, pacifism()).is_none());
    assert!(in_hand(&engine, p0, pacifism()).is_some());

    // Mode 2 (second legal option now that no Aura is on the battlefield): target creature gains trample until end of turn.
    tap_mana_where(&mut engine, p0, |id| id == my_forests[1]);
    cast_with_floating(&mut engine, p0, seedling_charm());
    engine.apply(p0, PlayerAction::ChooseMode(1)).unwrap();
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, bear).contains(KeywordSet::TRAMPLE));

    // Mode 1 (first legal option now that no Aura is on the battlefield): regenerate target green creature against Lightning Bolt.
    tap_mana_where(&mut engine, p0, |id| id == my_forests[2]);
    cast_with_floating(&mut engine, p0, seedling_charm());
    engine.apply(p0, PlayerAction::ChooseMode(0)).unwrap();
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, lightning_bolt());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![bear],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, grizzly_bears()).is_some(),
        "Grizzly Bears regenerates from Lightning Bolt"
    );
}
