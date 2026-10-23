#[allow(clippy::wildcard_imports)]
use super::*;

fn body_snatcher() -> CardIndex {
    card_index("e04688e0-672b-407f-a2c9-fd7292926483")
}

#[test]
fn body_snatcher_exiles_itself_on_death_to_return_creature_card_from_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[body_snatcher(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    bury(&mut engine, &[ogre]);
    let ogre_gy = in_graveyard(&engine, p0, gray_ogre()).unwrap();
    let snatcher = on_battlefield(&engine, p0, body_snatcher()).unwrap();

    let state = engine.dev_state_mut(p0).unwrap();
    crate::sba::destroy(state, snatcher);
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority");
    };
    engine.apply(player, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. }) || stack_is_empty(e)
    });
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        aim_at(&mut engine, p0, ogre_gy);
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, gray_ogre()).is_some());
}
