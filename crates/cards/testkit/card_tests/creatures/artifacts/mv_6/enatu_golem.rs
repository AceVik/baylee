//! `cards/creatures/artifacts/mv_6/enatu_golem.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn enatu_golem() -> CardIndex {
    card_index("1cb3f27c-2670-43b8-9305-8adf2e7bd3ae")
}

/// Enatu Golem — "When this creature dies, you gain 4 life."
#[test]
fn enatu_golem_gains_its_controller_four_life_when_it_dies() {
    let p0 = PlayerId::new(0);
    for index in [0u8, 1] {
        let owner = usize::from(index);
        let mut engine = Duel::new(SEED, forest())
            .battlefield(owner, &[enatu_golem()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let seat = PlayerId::new(index);
        let golem = on_battlefield(&engine, seat, enatu_golem()).expect("seated");
        assert_eq!(pt(&engine, golem), (3, 5));
        let (mine, theirs) = (life_of(&engine, owner), life_of(&engine, 1 - owner));
        kill(&mut engine, golem);
        assert!(in_graveyard(&engine, seat, enatu_golem()).is_some());
        assert_eq!(life_of(&engine, owner), mine + 4);
        assert_eq!(life_of(&engine, 1 - owner), theirs);
    }
}
