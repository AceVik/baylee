//! Horned Cheetah — {2}{G}{W} — Creature — Cat
//! Oracle: Whenever this creature deals damage, you gain that much life.

use super::*;
use crate::engine::card_tests::legends_kit::life;

#[test]
fn horned_cheetah_gains_life_equal_to_damage_dealt() {
    let cheetah = card_index("a552edbe-bd5d-4281-8d9f-d03f4b10815d");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest()).battlefield(0, &[cheetah]).start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let cheetah_id = on_battlefield(&engine, p0, cheetah).unwrap();
    assert_eq!(pt(&engine, cheetah_id), (2, 2));

    let _ = crate::engine::card_tests::legends_kit::attack(&mut engine, &[cheetah_id]);
    crate::engine::card_tests::legends_kit::block(&mut engine, &[]);
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain) && stack_is_empty(e)
    });

    assert_eq!(life(&engine, p1), 18);
    assert_eq!(life(&engine, p0), 22);
}
