#[allow(clippy::wildcard_imports)]
use super::*;

fn catastrophe() -> CardIndex {
    card_index("ed68160e-5e3d-44f2-ac7a-3749fda10119")
}

#[test]
fn catastrophe_destroys_all_creatures_when_creature_mode_chosen() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[catastrophe()])
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, catastrophe());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. }) || stack_is_empty(e)
    });
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        engine.apply(p0, PlayerAction::ChooseMode(1)).unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_none());
    assert!(on_battlefield(&engine, p1, gray_ogre()).is_none());
    assert!(on_battlefield(&engine, p0, plains()).is_some());
}
