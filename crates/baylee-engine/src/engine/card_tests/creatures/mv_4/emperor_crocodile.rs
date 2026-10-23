//! `cards/creatures/mv_4/emperor_crocodile.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn emperor_crocodile() -> CardIndex {
    card_index("26eb7ee3-c0b6-4be0-bc80-92952f55a6f9")
}

#[test]
fn emperor_crocodile_sacrifices_itself_when_you_control_no_other_creatures() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[emperor_crocodile(), llanowar_elves()])
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert!(
        on_battlefield(&engine, p0, emperor_crocodile()).is_some(),
        "another creature you control keeps it alive"
    );
    let elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    lk::float(&mut engine, p0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut engine, p0, lightning_bolt(), &[elf], &[]);

    assert!(on_battlefield(&engine, p0, emperor_crocodile()).is_none());
    assert!(in_graveyard(&engine, p0, emperor_crocodile()).is_some());
}
