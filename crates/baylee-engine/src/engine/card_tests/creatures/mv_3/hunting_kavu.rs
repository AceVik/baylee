//! Hunting Kavu — {1}{R}{G} — Creature — Kavu
//! Oracle: {1}{R}{G}, {T}: Exile this creature and target creature without flying that's attacking you.

use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

#[test]
fn hunting_kavu_exiles_itself_and_target_nonflying_attacker() {
    let kavu = card_index("a1b7fbfc-c510-4b1c-a2b9-c1204c699393");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = lk::game(
        SEED,
        &[
            index::GRIZZLY_BEARS,
            index::SERRA_ANGEL,
            index::LLANOWAR_ELVES,
        ],
        &[kavu],
        &[],
        &[],
    );
    let attacker = lk::obj(&engine, p0, index::GRIZZLY_BEARS);
    let flier = lk::obj(&engine, p0, index::SERRA_ANGEL);
    let at_home = lk::obj(&engine, p0, index::LLANOWAR_ELVES);
    let kavu_id = lk::obj(&engine, p1, kavu);

    let _ = lk::attack(&mut engine, &[attacker, flier]);
    lk::block(&mut engine, &[]);
    lk::pass_once(&mut engine);
    lk::float(
        &mut engine,
        p1,
        &[
            (ManaColor::Colorless, 1),
            (ManaColor::Red, 1),
            (ManaColor::Green, 1),
        ],
    );
    activate(&mut engine, p1, kavu, 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert!(options.contains(&attacker));
    assert!(!options.contains(&flier));
    assert!(!options.contains(&at_home));

    lk::aim(&mut engine, &[attacker], &[]);
    lk::settle(&mut engine);

    assert_eq!(lk::zone_of(&engine, kavu_id), Zone::Exile);
    assert_eq!(lk::zone_of(&engine, attacker), Zone::Exile);
    assert_eq!(lk::zone_of(&engine, flier), Zone::Battlefield);
}
