//! `cards/artifacts/mv_4/cumber_stone.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn cumber_stone() -> CardIndex {
    card_index("49199997-65cd-4cb9-bdd6-74c93c1f6b5a")
}

/// "Creatures your opponents control get -1/-0." Opponents' Ogre loses a
/// point of power and keeps its toughness; the controller's own Elves are
/// untouched; the same board without the Stone is the control.
#[test]
fn opposing_creatures_lose_one_power() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let board = |stone: bool| {
        let mine: Vec<CardIndex> = if stone {
            vec![cumber_stone(), llanowar_elves()]
        } else {
            vec![llanowar_elves()]
        };
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &mine)
            .battlefield(1, &[gray_ogre(), llanowar_elves()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        engine
    };

    let engine = board(false);
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    assert_eq!(pt(&engine, ogre), (2, 2), "control: a Gray Ogre is 2/2");

    let engine = board(true);
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    assert_eq!(pt(&engine, ogre), (1, 2), "-1/-0 on an opponent's creature");
    assert_eq!(pt(&engine, their_elf), (0, 1), "every opposing creature");
    assert_eq!(pt(&engine, my_elf), (1, 1), "never the Stone's own side");
}
