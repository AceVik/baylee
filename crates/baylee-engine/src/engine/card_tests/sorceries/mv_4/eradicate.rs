//! `cards/sorceries/mv_4/eradicate.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn eradicate() -> CardIndex {
    card_index("782ff740-ff43-4c07-af0b-433cb9770661")
}

#[test]
fn eradicate_exiles_target_nonblack_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[eradicate()])
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[gray_ogre(), festering_goblin()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    cast_from_hand(&mut engine, p0, eradicate());
    let black = on_battlefield(&engine, p1, festering_goblin()).unwrap();
    let land = on_battlefield(&engine, p0, swamp()).unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("nonblack creature targets")
    };
    assert!(options.contains(&ogre));
    assert!(!options.contains(&black));
    assert!(!options.contains(&land));
    aim_at(&mut engine, p0, ogre);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, gray_ogre()).is_none());
    assert_eq!(engine.state().object(ogre).unwrap().zone, Zone::Exile);
}
